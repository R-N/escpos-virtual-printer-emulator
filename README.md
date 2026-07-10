# 🖨️ ESC/POS Virtual Printer Emulator

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/Garletz/escpos-virtual-printer-emulator)

> **ESC/POS virtual printer emulator built in Rust.**
> Turns your computer into a virtual receipt printer for testing and development — no hardware required.

<img width="1920" height="1080" alt="Receipt viewer" src="https://github.com/user-attachments/assets/709335cd-79b9-40fd-ab51-7027f6ee0405" />
<img width="1920" height="1080" alt="Command log" src="https://github.com/user-attachments/assets/c02db29b-53ca-49e1-b145-6b7cb31e4fc1" />

## How it works

The emulator opens a TCP socket on `127.0.0.1:9100` — the standard port for network thermal printers. Any application that prints to that socket (directly, or via an installed OS printer) has its ESC/POS byte stream parsed and rendered live in the GUI. You see exactly what would come out of a real receipt printer.

```
your app → TCP :9100 → parser → printer state → live GUI preview
```

## Supported paper widths

| Width | Dots | Approx. chars | Use case |
|-------|------|---------------|----------|
| **50mm** | 384 | ~48 | Small receipts, tickets |
| **78mm** | 576 | ~72 | Standard receipts |
| **80mm** | 640 | ~80 | Large receipts, invoices |

Character counts vary with font size; paper width is detected from the ESC/POS stream.

## Quick start

### Prerequisites

- **Rust 1.70+** — [install Rust](https://rustup.rs/)
- **Windows 10/11** or **Linux** with CUPS
- **Administrator privileges** — only needed to install the virtual OS printer

### Build & run

```bash
git clone https://github.com/Garletz/escpos-virtual-printer-emulator.git
cd escpos-virtual-printer-emulator
cargo run --release
```

The GUI opens and the TCP server starts listening on `127.0.0.1:9100` automatically.

### Install the virtual printer (optional)

To print from real applications (rather than sending bytes to the socket yourself):

- Open the **Settings** tab.
- Click **🖨️ Install Windows Printer** or **🐧 Install Linux Printer** (requires admin / `sudo`).
- A printer named `ESC_POS_Virtual_Printer` appears in your OS printer list.
- **Uninstall Printer** removes it again.

### Usage

1. Start the emulator — server runs on port 9100.
2. (Optional) install the virtual printer from the Settings tab.
3. Print from any application, or send bytes straight to `127.0.0.1:9100`.
4. Watch the **Receipt** tab for a live preview and the **Commands** tab for the parsed command log.

## Supported ESC/POS commands

| Command | Description | Notes |
|---------|-------------|-------|
| `ESC @` | Initialize printer | |
| `ESC M n` | Select font | `0`=A, `1`=B, `2`=C |
| `ESC a n` | Justification | `0`=left, `1`=center, `2`=right |
| `ESC E` / `ESC F` | Emphasis (bold) on / off | |
| `ESC - n` | Underline | `n != 0` enables |
| `ESC 4` / `ESC 5` | Italic on / off | |
| `ESC 3 n` | Line height | |
| `ESC ! n` | Font size / print mode | |
| `ESC t n` | Select code page | |
| `ESC J n` | Paper feed | |
| `ESC m` / `ESC i` | Cut paper | |
| `ESC * m nL nH …` | Bit image | parsed; shown as placeholder |
| `GS v 0 …` | Raster bit image | rendered as a bitmap |
| `GS V n` | Cut paper (variants) | |
| `LF` (`\n`) / `CR` (`\r`) | New line / carriage return | |

Unrecognized sequences are captured as `Unknown` and listed in the command log.

## QZ Tray integration

The **Settings** tab has a QZ Tray Integration panel with a ready-to-copy JS snippet for either wiring style:

- **Direct socket** — `qz.configs.create({ host: '127.0.0.1', port: 9100 })`. QZ Tray opens a TCP connection straight to the emulator; no OS printer install needed.
- **Via installed OS printer** — `qz.configs.create('ESC_POS_Virtual_Printer')`. Prints through the OS spooler and the installed printer (see above), matching how a real network receipt printer is normally wired in production.

Toggle between the two in Settings to see the matching snippet and copy it into your POS app.

## Development

```bash
cargo run        # build + launch GUI (server on :9100)
cargo build      # dev build
cargo check      # fast type-check
```

### Project layout

```
src/
├── main.rs          # entry point: starts TCP server task + GUI
├── lib.rs           # module re-exports
├── escpos/          # command vocabulary, stream parser, printer state
│   ├── commands.rs  #   EscPosCommand enum
│   ├── parser.rs    #   incremental byte-stream parser
│   └── printer.rs   #   PrinterState: flags + receipt buffer + bitmap render
├── emulator/mod.rs  # EmulatorState: printer state + command history
├── networking/      # TCP server on 127.0.0.1:9100
│   └── server.rs
└── gui/             # eframe/egui interface (Receipt / Commands / Settings tabs)
```

See [`AGENTS.md`](AGENTS.md) / [`CLAUDE.md`](CLAUDE.md) for deeper architecture notes and how to add new commands.

### Dependencies

- **eframe / egui** — GUI
- **tokio** — async runtime + TCP networking
- **serde / serde_json** — serialization
- **image** — bitmap rendering
- **tracing** — structured logging
- **anyhow / thiserror** — error handling
- **chrono / uuid** — timestamps and ids

## License

MIT — see [LICENSE](LICENSE).
