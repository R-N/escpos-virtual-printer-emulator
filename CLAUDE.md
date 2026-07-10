# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A virtual ESC/POS receipt-printer emulator. It listens on a TCP socket like a real network thermal printer (port 9100), parses the ESC/POS byte stream sent by any application, and renders the result live in a GUI. Used for testing/developing POS software without physical hardware.

## Commands

```bash
cargo run              # Build + launch the GUI (server auto-starts on 127.0.0.1:9100)
cargo run --release    # Optimized run
cargo build            # Dev build
cargo build --release  # Optimized build (LTO, panic=abort, stripped)
cargo check            # Type-check only
```

There are currently **no tests** in the repo despite the README mentioning `cargo test`. If adding tests, the parser (`src/escpos/parser.rs`) is the most logic-heavy, hardware-independent unit to target.

## Architecture

Single binary (`escpos_emulator`). Two concurrent halves share one mutable state object:

- **TCP server** (`networking/server.rs`) — a `tokio` task spawned from `main.rs`. Accepts connections on `127.0.0.1:9100`, reads raw bytes in 1 KiB chunks.
- **GUI** (`gui/`) — `eframe`/`egui` native window on the main thread.

Both hold `Arc<Mutex<EmulatorState>>` (tokio `Mutex`). The server `.lock().await`s to mutate; the GUI uses **`try_lock()`** everywhere so a busy lock never freezes the UI frame — when the lock is contended the GUI just skips that frame's read. Keep that pattern: never block the egui update loop on the mutex.

### Data pipeline (the core flow)

```
raw bytes → EscPosParser → Vec<EscPosCommand> → EmulatorState.process_command
          → PrinterState (mutates flags + appends to buffer: Vec<ReceiptLine>)
          → ReceiptViewer renders buffer to the GUI
```

- `escpos/commands.rs` — `EscPosCommand` enum: the parsed command vocabulary (text, formatting flags, cut, images, codepage, unknown).
- `escpos/parser.rs` — `EscPosParser::parse_stream`. Byte-walks the buffer dispatching on `ESC` (0x1B), `GS` (0x1D), `\n`, `\r`, or runs of text. **Incremental**: returns `Ok(None)` for a command when not enough bytes have arrived yet and breaks to wait for more. Each handler returns `(command, bytes_consumed)`.
- `escpos/printer.rs` — `PrinterState` is the live printer: current font/justification/emphasis/etc. flags plus `buffer: Vec<ReceiptLine>` (Text / Bitmap / Separator). `process_command` applies a command: formatting commands flip flags, `Text` appends with wrapping at `get_max_chars`, `GS v 0` raster images become `ReceiptLine::Bitmap`. Also holds bitmap→RGB conversion for 1bpp packed data.
- `emulator/mod.rs` — `EmulatorState` wraps `PrinterState` and keeps a bounded `command_history` (`VecDeque`, max 1000) for the command-log tab.
- `gui/app.rs` — tab shell (Receipt / Commands / Settings). `receipt_viewer.rs` renders the buffer (caches bitmap textures by FNV-1a hash of data). `settings_panel.rs` shells out to PowerShell (Windows) / CUPS `lpadmin` (Linux) to install/uninstall the OS printer pointing at port 9100, and also renders a QZ Tray integration snippet picker (`QzMode::DirectSocket` vs `QzMode::OsPrinter`). All three tab views wrap their content in `egui::ScrollArea` — keep that when adding content that can grow past window height.

### Adding support for a new ESC/POS command

This is the most common change. Touch four places in order:

1. `commands.rs` — add a variant to `EscPosCommand`.
2. `parser.rs` — add a match arm in `parse_esc_command` or `parse_gs_command`; return `Ok(None)` if more bytes are needed, else `(cmd, consumed)`.
3. `printer.rs` — handle the variant in `PrinterState::process_command`.
4. `receipt_viewer.rs` — render it if it produces visible output.

## Gotchas

- **Server buffer vs parser buffer.** `server.rs` keeps its own `buffer` AND `EscPosParser` keeps an internal `buffer`; the server `extend`s the parser-fed slice then `buffer.clear()`s on every successful parse. The parser's internal buffer is the one that actually retains unconsumed partial commands across reads. Be careful editing either — double-buffering here is fragile.
- `render_receipt`/`bitmap_to_rgb` in `printer.rs` build full `RgbImage`s but the GUI renders text via egui labels, not the rendered image; the image path is mostly used for bitmaps.
- Bind address is hardcoded to `127.0.0.1:9100` in `server.rs` (also referenced in `settings_panel.rs` printer-install commands and QZ Tray snippets). Change all together.
- Some inline comments are in French.
- **Windows printer driver**: `install_windows_printer` must use the built-in `'Generic / Text Only'` driver by exact name, not a `*Microsoft*` wildcard match — that grabs "Send To Microsoft OneNote" or "Microsoft Print To PDF", neither of which forwards raw bytes to the TCP port. Also keep `$ErrorActionPreference='Stop'` + `try/catch` + `exit 1` in that script; without it, PowerShell's non-terminating errors let an unconditional `Write-Host 'installed successfully'` run after a real failure, and `output.status.success()` on the Rust side reports a false positive.
- Building on Windows needs the MSVC linker (`link.exe`) and Windows SDK (for `kernel32.lib`) both present — if `cargo build` fails with an `LNK1181` or a bogus `link.exe` usage error, it's picking up a non-MSVC `link.exe` (coreutils/git-bash) from PATH instead of MSVC's; build from a VS Developer shell (`vcvars64.bat`) or the "x64 Native Tools Command Prompt" to fix.
