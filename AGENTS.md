# Repository Guidelines

## Project Structure & Module Organization

This Rust 2024 project provides a Linux CLI and library for the HHKB Professional Hybrid. `src/main.rs` queries keyboard information, DIP switches, mode, and the base keymap. `src/lib.rs` exposes the library modules. Keep USB HID protocol handling in `src/hhkb.rs`, configuration and key types in `src/config.rs`, and shared byte/string helpers in `src/utils.rs`. Configuration support is unfinished; `KeyboardConfig::new` currently contains `todo!()`.

`scripts/device.sh` prepares USB packet capture. `README.md` documents setup and usage. There are currently no dedicated test or asset directories.

## Build, Test, and Development Commands

Install the Linux development packages listed in `README.md` and use a Rust toolchain supporting edition 2024.

- `cargo build` — compile the library and CLI for development.
- `cargo build --release` — produce the optimized executable in `target/release/hhkb`.
- `cargo run` — query a connected keyboard; requires USB HID access and the documented udev setup.
- `cargo test` — run Rust unit, integration, and documentation tests as they are added.
- `cargo fmt --check` — check formatting; use `cargo fmt` to apply it.
- `cargo clippy --all-targets` — inspect lint diagnostics.

No custom formatter, linter configuration, or CI workflow is checked in.

## Coding Style & Naming Conventions

Follow standard rustfmt formatting with four-space indentation. Use `snake_case` for functions, modules, and variables, `PascalCase` for types and enum variants, and `SCREAMING_SNAKE_CASE` for constants. Follow existing `anyhow::Result` and `?` patterns for fallible device operations. Keep protocol constants and packet validation close to the HID implementation.

## Testing Guidelines

No automated tests or coverage threshold are currently defined. Add hardware-independent unit tests in module-local `#[cfg(test)]` blocks; place public API integration tests in `tests/`. Use descriptive names such as `rejects_invalid_response_header`. Prioritize packet validation, string decoding, and configuration serialization. Document hardware checks separately, including keyboard mode and observed output.

## Commit & Pull Request Guidelines

The short Git history has no consistent enforced convention. Prefer concise, imperative subjects, such as `Validate keymap response lengths`. Keep changes focused. PRs should explain behavior changes, list validation commands and results, link relevant issues, and describe hardware prerequisites for reproduction. Update `README.md` when setup or usage changes.

## Device Access

Use the documented udev permissions for normal operation. Run `scripts/device.sh` only when packet capture is needed: it invokes `sudo`, loads `usbmon`, and broadens capture-device read permissions.
