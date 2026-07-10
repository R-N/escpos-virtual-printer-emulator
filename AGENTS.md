# AGENTS.md

Guidance for AI coding agents working in this repository. (Claude Code: see also `CLAUDE.md`, which holds the same architecture notes.)

## Project

Virtual ESC/POS receipt-printer emulator in Rust. Listens on TCP `127.0.0.1:9100` like a real network thermal printer, parses the ESC/POS byte stream from any app, and renders the receipt live in an `egui` GUI. Lets POS software be tested without physical hardware.

## Setup & commands

- `cargo run` — build and launch the GUI; the TCP server auto-starts on `127.0.0.1:9100`.
- `cargo run --release` — optimized run.
- `cargo build` / `cargo build --release` — dev / optimized build (release uses LTO, `panic=abort`, stripped).
- `cargo check` — fast type-check.

No test suite exists yet. If you add tests, target `src/escpos/parser.rs` — it holds the most hardware-independent logic.

## Architecture

Single binary. Two concurrent halves share one `Arc<Mutex<EmulatorState>>` (tokio `Mutex`):

- **TCP server** (`networking/server.rs`) — a `tokio` task spawned in `main.rs`. Reads raw bytes in 1 KiB chunks, feeds the parser, `.lock().await`s to mutate state.
- **GUI** (`gui/`) — `eframe`/`egui` window on the main thread. Uses **`try_lock()`** everywhere so a contended lock never freezes a frame. Never block the egui update loop on the mutex.

### Data pipeline

```
raw bytes → EscPosParser → Vec<EscPosCommand> → EmulatorState.process_command
          → PrinterState (flags + buffer: Vec<ReceiptLine>) → ReceiptViewer renders
```

- `escpos/commands.rs` — `EscPosCommand` enum (the parsed command vocabulary).
- `escpos/parser.rs` — `parse_stream` byte-walks dispatching on `ESC` (0x1B), `GS` (0x1D), `\n`, `\r`, or text runs. **Incremental**: returns `Ok(None)` and breaks when more bytes are needed. Handlers return `(command, bytes_consumed)`.
- `escpos/printer.rs` — `PrinterState`: live font/justification/emphasis flags + `buffer: Vec<ReceiptLine>` (Text / Bitmap / Separator). `process_command` applies each command.
- `emulator/mod.rs` — `EmulatorState`: wraps `PrinterState`, keeps bounded `command_history` (`VecDeque`, max 1000).
- `gui/` — `app.rs` (tab shell), `receipt_viewer.rs` (renders buffer, caches bitmap textures), `command_log.rs`, `settings_panel.rs` (shells out to PowerShell / CUPS to install the OS printer; also renders the QZ Tray integration snippet picker — `QzMode::DirectSocket` vs `QzMode::OsPrinter`). All three tab views wrap their content in `egui::ScrollArea` — keep that when adding content that can grow past window height.

### Adding a new ESC/POS command (most common task)

Touch four files in order:

1. `commands.rs` — add a variant to `EscPosCommand`.
2. `parser.rs` — add a match arm in `parse_esc_command` / `parse_gs_command`; return `Ok(None)` if more bytes are needed.
3. `printer.rs` — handle it in `PrinterState::process_command`.
4. `receipt_viewer.rs` — render it if it produces visible output.

## Gotchas

- **Double buffering**: `server.rs` keeps its own `buffer` and `EscPosParser` keeps an internal one. The parser's internal buffer retains unconsumed partial commands across reads. Edit either carefully.
- Bind address `127.0.0.1:9100` is hardcoded in `server.rs` and referenced in `settings_panel.rs` install commands and QZ Tray snippets — change all together.
- `render_receipt`/`bitmap_to_rgb` build full `RgbImage`s, but text is drawn via egui labels; the image path is mostly for bitmaps.
- Some inline comments are in French.
- **Windows printer driver**: `install_windows_printer` must use the built-in `'Generic / Text Only'` driver by exact name, not a `*Microsoft*` wildcard match — that grabs "Send To Microsoft OneNote" or "Microsoft Print To PDF", neither of which forwards raw bytes to the TCP port. Also keep `$ErrorActionPreference='Stop'` + `try/catch` + `exit 1` in that script; without it, PowerShell's non-terminating errors let an unconditional `Write-Host 'installed successfully'` run after a real failure, and `output.status.success()` on the Rust side reports a false positive.
- Building on Windows needs the MSVC linker (`link.exe`) and Windows SDK (for `kernel32.lib`) both present — if `cargo build` fails with an `LNK1181` or a bogus `link.exe` usage error, it's picking up a non-MSVC `link.exe` (coreutils/git-bash) from PATH instead of MSVC's; build from a VS Developer shell (`vcvars64.bat`) or the "x64 Native Tools Command Prompt" to fix.

## Conventions

- Match the surrounding style; keep the non-blocking `try_lock()` GUI pattern.
- Don't introduce new hardcoded addresses/ports without threading them through both call sites.
