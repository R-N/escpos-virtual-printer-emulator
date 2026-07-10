use crate::emulator::EmulatorState;
use egui::{ScrollArea, Ui};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QzMode {
    /// qz.configs.create({ host, port }) — QZ Tray opens a TCP socket straight
    /// to the emulator, no OS printer / driver involved.
    DirectSocket,
    /// qz.configs.create('ESC_POS_Virtual_Printer') — QZ Tray prints through
    /// the OS print queue, which forwards to the emulator via the installed
    /// Generic/Text-Only driver + RAW TCP port.
    OsPrinter,
}

const QZ_SNIPPET_DIRECT: &str = r#"var config = qz.configs.create({ host: '127.0.0.1', port: 9100 });

var data = [
  { type: 'raw', format: 'command', flavor: 'plain', data: [
     '\x1B\x40',            // init
     '\x1B\x61\x31',        // center
     'Hello World\x0A',
     '\x1D\x56\x00'         // cut
  ]}
];

qz.print(config, data).catch(e => console.error(e));"#;

const QZ_SNIPPET_OS_PRINTER: &str = r#"var config = qz.configs.create('ESC_POS_Virtual_Printer');

var data = [
  { type: 'raw', format: 'command', flavor: 'plain', data: [
     '\x1B\x40',            // init
     '\x1B\x61\x31',        // center
     'Hello World\x0A',
     '\x1D\x56\x00'         // cut
  ]}
];

qz.print(config, data).catch(e => console.error(e));"#;

pub struct SettingsPanel {
    qz_mode: QzMode,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            qz_mode: QzMode::DirectSocket,
        }
    }
}

impl SettingsPanel {
    pub fn show(&mut self, ui: &mut Ui, _state: &mut EmulatorState) {
        ui.heading("Emulator Settings");
        ui.separator();

        ScrollArea::vertical().show(ui, |ui| {
            self.show_content(ui);
        });
    }

    fn show_content(&mut self, ui: &mut Ui) {
        // Virtual printer management
        ui.group(|ui| {
            ui.label("Virtual Printer Management");
            ui.label("Installs the emulator as a system printer");
            
            ui.horizontal(|ui| {
                if ui.button("🖨️ Install Windows Printer").clicked() {
                    self.install_windows_printer();
                }
                
                if ui.button("🐧 Install Linux Printer").clicked() {
                    self.install_linux_printer();
                }
                
                if ui.button("🗑️ Uninstall Printer").clicked() {
                    self.uninstall_printer();
                }
            });

            ui.label("Note: Requires administrator privileges");
            
            // Check printer status
            if ui.button("🔍 Check Status").clicked() {
                self.check_printer_status();
            }
        });

        ui.separator();

        // Network settings
        ui.group(|ui| {
            ui.label("Network Configuration");
            ui.label("TCP Port: 9100");
            ui.label("Address: 127.0.0.1");
            
            if ui.button("📡 Test Connection").clicked() {
                self.test_network_connection();
            }
        });

        ui.separator();

        // QZ Tray integration
        ui.group(|ui| {
            ui.label("🖨️ QZ Tray Integration");
            ui.label("Choose how your POS app's QZ Tray client should reach this emulator:");

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.qz_mode, QzMode::DirectSocket, "Direct socket (no OS printer)");
                ui.selectable_value(&mut self.qz_mode, QzMode::OsPrinter, "Via installed OS printer");
            });

            let snippet = match self.qz_mode {
                QzMode::DirectSocket => QZ_SNIPPET_DIRECT,
                QzMode::OsPrinter => QZ_SNIPPET_OS_PRINTER,
            };

            ui.add_space(4.0);
            ui.add(egui::TextEdit::multiline(&mut snippet.to_string())
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY));

            if ui.button("📋 Copy snippet").clicked() {
                ui.output_mut(|o| o.copied_text = snippet.to_string());
            }

            match self.qz_mode {
                QzMode::DirectSocket => {
                    ui.label("No install step needed — QZ Tray connects straight to 127.0.0.1:9100.");
                }
                QzMode::OsPrinter => {
                    ui.label("Requires 'ESC_POS_Virtual_Printer' to be installed first (see above).");
                }
            }
        });

        ui.separator();

        // Information about operation
        ui.group(|ui| {
            ui.label("ℹ️  Automatic Operation");
            ui.label("• The emulator automatically respects ESC/POS standards");
            ui.label("• Paper width: 50mm, 78mm, 80mm (auto-detection)");
            ui.label("• Font, justification, emphasis: ESC/POS commands");
            ui.label("• No manual configuration needed!");
        });
    }

    fn install_windows_printer(&self) {
        // Use the built-in "Generic / Text Only" driver: it forwards raw bytes
        // unchanged to the RAW TCP port, which is what ESC/POS needs. Do NOT pick
        // a driver by '*Microsoft*' name match — that grabs "Send To Microsoft
        // OneNote" / "Microsoft Print To PDF", which capture output instead of
        // sending it to :9100 (or fail to bind a TCP port). $ErrorActionPreference
        // = 'Stop' + exit 1 makes a real failure surface instead of printing a
        // bogus success message.
        let output = Command::new("powershell")
            .args([
                "-Command",
                "$ErrorActionPreference='Stop'; \
                 try { \
                   if (-not (Get-PrinterPort -Name '127.0.0.1:9100' -ErrorAction SilentlyContinue)) { \
                     Add-PrinterPort -Name '127.0.0.1:9100' -PrinterHostAddress '127.0.0.1' -PortNumber 9100 \
                   } \
                   Add-PrinterDriver -Name 'Generic / Text Only'; \
                   Add-Printer -Name 'ESC_POS_Virtual_Printer' -DriverName 'Generic / Text Only' -PortName '127.0.0.1:9100'; \
                   Write-Host 'Printer installed successfully' \
                 } catch { Write-Error $_; exit 1 }"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("✅ {}", stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("❌ Error: {}", stderr);
                }
            }
            Err(e) => {
                println!("❌ Cannot execute printer installation: {}", e);
            }
        }
    }

    fn install_linux_printer(&self) {
        // Install Linux printer using CUPS
        let output = Command::new("bash")
            .args([
                "-c",
                "if command -v lpstat &> /dev/null; then \
                    echo 'Installing Linux printer...'; \
                    sudo lpadmin -p ESC_POS_Linux_Printer -E -v socket://127.0.0.1:9100 -m 'Generic Text-Only Printer'; \
                    sudo lpadmin -d ESC_POS_Linux_Printer; \
                    echo 'Linux printer installed successfully!'; \
                else \
                    echo 'CUPS not found. Please install CUPS first.'; \
                fi"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("ℹ️  {}", stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("ℹ️  {}", stderr);
                }
            }
            Err(e) => {
                println!("ℹ️  Linux installation attempted: {}", e);
            }
        }
    }

    fn uninstall_printer(&self) {
        // Simplified PowerShell command
        let output = Command::new("powershell")
            .args([
                "-Command",
                "Remove-Printer -Name 'ESC_POS_Virtual_Printer' -Confirm:$false; \
                 Remove-PrinterPort -Name '127.0.0.1:9100'; \
                 Write-Host 'Printer uninstalled successfully'"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("✅ {}", stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("❌ Error: {}", stderr);
                }
            }
            Err(e) => {
                println!("❌ Cannot execute printer uninstallation: {}", e);
            }
        }
    }

    fn check_printer_status(&self) {
        // Check if printer is installed
        let output = Command::new("powershell")
            .args([
                "-Command",
                "Get-Printer -Name 'ESC_POS_Virtual_Printer' -ErrorAction SilentlyContinue | Select-Object Name, PortName, DriverName, PrinterStatus"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.trim().is_empty() {
                        println!("ℹ️  Virtual printer not installed");
                    } else {
                        println!("✅ Virtual printer installed:");
                        println!("{}", stdout);
                    }
                }
            }
            Err(e) => {
                println!("❌ Cannot check status: {}", e);
            }
        }
    }

    fn test_network_connection(&self) {
        // Test connection to port 9100
        let output = Command::new("powershell")
            .args([
                "-Command",
                "Test-NetConnection -ComputerName 127.0.0.1 -Port 9100 -WarningAction SilentlyContinue | Select-Object TcpTestSucceeded"
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("True") {
                        println!("✅ Connection to port 9100 successful");
                    } else {
                        println!("❌ Connection to port 9100 failed");
                    }
                } else {
                    println!("❌ Cannot test connection");
                }
            }
            Err(e) => {
                println!("❌ Cannot test connection: {}", e);
            }
        }
    }
}
