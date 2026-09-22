# HHKB Professional Hybrid Key Remapper (Linux)

An unofficial key remapping tool and library for the **Happy Hacking Keyboard (HHKB) Professional Hybrid** on Linux, written in Rust.

This tool communicates directly with the keyboard over its USB HID programming interface (using the `hidapi` crate) to read keyboard information and DIP switches, and export/import raw base and Fn maps without the official Windows/macOS keymap tool.

## Features

- **Read Keyboard Info**: Retrieve HHKB serial number, firmware versions, type number, and currently running firmware.
- **Query DIP Switches**: Read the hardware DIP switch states.
- **Editable keyboard rows**: Export/import aligned base and Fn rows with friendly action names, physical labels, and lossless raw escapes.
- **Preview and format**: Show intended changes with `hhkb diff` and align rows with `hhkb format`.
- **Preservation and verification**: Preserve both companion maps and sleep time, save an import backup, and verify a fresh readback.
- US-layout PD-KB800 models in mode `0` are supported; additional modes/layouts remain unverified.

## Prerequisites

### Dependencies

On Debian/Ubuntu-based systems, you need `libusb` and `libhidapi`:

```bash
sudo apt install build-essential pkg-config libusb-1.0-0-dev libhidapi-dev
```

### USB Permissions (udev rules)

By default, Linux limits raw HID access to `root`. To run this tool as a regular user, create a udev rule.

Create a file named `/etc/udev/rules.d/99-hhkb.rules` with the following content:

```udev
# HHKB Professional Hybrid
SUBSYSTEMS=="usb", ATTRS{idVendor}=="04fe", ATTRS{idProduct}=="0021", MODE="0660", GROUP="plugdev"
KERNEL=="hidraw*", ATTRS{idVendor}=="04fe", ATTRS{idProduct}=="0021", MODE="0660", GROUP="plugdev"
```

Then reload the udev rules:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Make sure your user is a member of the `plugdev` group (or adjust the rule to use another group like `wheel` or `input` as appropriate for your distribution).

## Usage

### Build

Compile the project using Cargo:

```bash
cargo build --release
```

### Run

Run the tool to print keyboard information, DIP switch states, and the current base layer:

```bash
cargo run
```

### Edit keyboard rows

```bash
hhkb export                 # Save hhkb.toml in the current directory.
cp hhkb.toml edited.toml
# Edit the assignments in edited.toml.
hhkb format edited.toml      # Align each row and its physical-label comment.
hhkb diff edited.toml        # Preview changes against the connected keyboard.
hhkb import edited.toml     # Back up, write, and verify.
hhkb import hhkb.toml       # Restore the original base/Fn assignments.
```

You can supply a different export path: `hhkb export original.toml`. Export
refuses to overwrite existing files. From the source tree, prefix commands with
`cargo run --`, for example `cargo run -- export`.

Exports now use schema version 2, with five complete rows in both `[base]` and
`[fn]`. Each physical row stays on one line. Comments above the arrays label
physical key positions; array strings are their assigned actions. For example:

```toml
[base]
# keys:      Ctrl     A    S    D    F    G    H    J    K    L    ;    '    Enter
home_row = ["LCtrl", "B", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "Enter"]
```

Here the physical A key produces B. Friendly actions include letters, digits,
punctuation, navigation, modifiers, and F1–F24. Names are case-insensitive;
`Esc` and `Ctrl` are accepted aliases for `Escape` and `LCtrl`. Unknown and zero
values use explicit escapes such as `"raw:0xE8"` and `"raw:0x00"`. Vendor-specific
media and shortcut values retain raw names until their meanings are validated.

Both layers are complete and independent: changing base does not change Fn.
Exact row lengths are required, and errors identify the layer, row, and
physical position. The `[preservation]` section retains bytes without physical
key labels, including nonzero values. Keep this section when editing.

`hhkb diff [path]` defaults to `hhkb.toml`, checks device identity, and reports
physical assignments and preservation bytes that would change. It performs
read sessions without issuing configuration-write commands.
`hhkb format [path]` also defaults to `hhkb.toml` and works without a keyboard.
It canonicalizes names, regenerates aligned label comments, and keeps user
comments. Notes within multiline arrays move above the resulting one-line row.
The formatter validates first and replaces the file atomically. It upgrades
legacy raw files to rows, retaining their comments above the new document.

The row layout currently supports US-layout PD-KB800 models in mode `0`.
Export device model, serial, layout, and mode identify the target keyboard.
Existing version 1 raw exports still import; `hhkb export --raw original.toml`
creates that format. To upgrade an existing raw `hhkb.toml`, run `hhkb format`.
See the [complete sanitized example](tests/fixtures/rows.toml) and
[row-format details](docs/row-format.md).

Import validates the schema, rows/actions, mode, and connected device identity
before writing. It saves the current raw configuration beside the input as
`<filename>.backup-<Unix timestamp in nanoseconds>.toml` and aborts if that save
fails. It takes a fresh preservation snapshot, writes the requested layers
alongside the unchanged companion maps and sleep time, and verifies all four
maps, mode, and sleep time in a fresh read session. A failed write or readback
is an error; configuration writes are not retried and no rollback is claimed.

The library's `read_current_keymaps()` and `write_current_keymaps()` retain
128-byte maps. `CurrentKeymaps::from_toml()` reads either schema;
`to_visual_toml()` produces rows and `to_toml()` produces raw version 1 arrays.
`read_snapshot()` also exposes companion maps and sleep time.
See [verified protocol behavior](docs/protocol.md) and
[hardware acceptance](docs/hardware-acceptance.md). The experimental legacy
named-key structs are retained but are not used by this workflow.

### Packet Sniffing / Wireshark

A helper script is provided in `scripts/device.sh` to configure the Linux `usbmon` kernel module and find the appropriate bus/device addresses for capturing USB packets with Wireshark.

```bash
./scripts/device.sh
```

## Project Structure

- [src/main.rs](src/main.rs): Entry point of the CLI application.
- [src/hhkb.rs](src/hhkb.rs): Core HHKB USB HID protocol implementation (sending commands, reading keymap blocks).
- [src/config.rs](src/config.rs): Structs representing layouts, layers, keys, and deserialization/serialization definitions.
- [src/lib.rs](src/lib.rs): Library entry point exposing modules.
- [scripts/device.sh](scripts/device.sh): Script to set up usbmon and filter rules for packet capturing.

## Disclaimer

This is an unofficial tool and is not associated with or endorsed by PFU Limited or Fujitsu. Modifying device firmware or keymaps carries risks. Use at your own risk.
