# HHKB Professional Hybrid Key Remapper (Linux)

An unofficial key remapping tool and library for the **Happy Hacking Keyboard (HHKB) Professional Hybrid** on Linux, written in Rust.

This tool communicates directly with the keyboard over its USB HID programming interface (using the `hidapi` crate) to read keyboard information and DIP switches, and export/import base and Fn assignments, including shortcuts, without the official Windows/macOS keymap tool.

## Features

- **Read Keyboard Info**: Retrieve HHKB serial number, firmware versions, type number, and currently running firmware.
- **Query DIP Switches**: Read the hardware DIP switch states.
- **Editable keyboard rows**: Export/import aligned base and Fn rows with friendly action names, shortcuts, physical labels, and lossless raw escapes.
- **Preview and format**: Show intended changes with `hhkb diff` and align rows with `hhkb format`.
- **Preservation and verification**: Preserve all four maps and sleep time, save an import backup, and verify a fresh readback.
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

Exports use schema version 3, with five complete rows in both `[base]` and
`[fn]`. Each physical row stays on one line. Comments above the arrays label
physical key positions; array strings are their assigned actions. For example:

```toml
[base]
# keys:      Ctrl     A    S    D    F    G    H    J    K    L    ;    '    Enter
home_row = ["LCtrl", "B", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "Enter"]
```

Here the physical A key produces B. Replacing `"B"` with `"Ctrl+Shift+B"`
assigns a shortcut at physical A. Friendly actions cover every single-key
code in the official Windows Professional Keymap Tool 2.0.1, including
`VolumeDown`, `VolumeUp`, `Mute`, `Eject`, `BrightnessUp`, `BrightnessDown`,
`Power`, `Stop`, keypad keys, F1–F24, and Japanese input keys. Names are
case-insensitive; official labels such as `"Volume Up"` also work. `Esc` and
`Ctrl` are aliases for `Escape` and `LCtrl`. Zero normally exports as
`InvalidKey`, matching the official tool's label. Unknown bytes retain lossless escapes such
as `"raw:0xAB"`; existing raw escapes still import. See the
[action catalog](docs/actions.md) for names, aliases, and source evidence.

To assign a shortcut, write modifiers before one key, for example
`"Ctrl+Shift+C"`, `"RCtrl+RAlt+Delete"`, or `"LMeta+T"`. Left and right
modifiers are distinct. Modifier-only shortcuts such as `"Ctrl+Shift"`
or `"Ctrl+(M)"` also work. Export and format use explicit canonical names such as
`"LCtrl+LShift+C"`. A plain name like `"C"` clears that key's shortcut
modifiers. The official tool allows any number of modifiers and one ordinary
key per shortcut. `Fn` is not a shortcut modifier. See
[shortcut details](docs/row-format.md#shortcuts).

Newer actions and shortcut edits require compatible firmware. This tool checks
for running HYBRID application firmware A0.48 or later before editing shortcut
masks.

Both layers are complete and independent: changing base does not change Fn.
On the Fn layer, physical Q, Ctrl, and RShift export as `Reserved`; their key
and shortcut modifier bytes must remain zero. Import rejects assignments to
those positions in both row and raw files. Older `InvalidKey` entries at those
positions still load and are canonicalized to `Reserved` by `hhkb format`.
Exact row lengths are required, and errors identify the layer, row, and
physical position. The `[preservation]` section retains bytes without physical
key labels from all four maps, including nonzero values. Keep this section when editing.

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
Legacy version 1 raw and version 2 row files still import. When they omit
modifier maps, import preserves the device's current modifiers. Run
`hhkb export` to create a fresh version 3 file before adding shortcuts.
`hhkb export --raw original.toml` writes version 1 arrays including modifier
maps; formatting that file produces version 3 rows. The
[complete sanitized example](tests/fixtures/rows.toml) shows the older version
2 layout. See [row-format details](docs/row-format.md).

Import validates the schema, rows/actions, mode, and connected device identity
before writing. It saves the current raw configuration beside the input as
`<filename>.backup-<Unix timestamp in nanoseconds>.toml` and aborts if that save
fails. It takes a fresh preservation snapshot, writes the requested layers
alongside the requested modifier maps and unchanged sleep time, and verifies
all four maps, mode, and sleep time in a fresh read session. A failed write or
readback is an error; configuration writes are not retried and no rollback is
claimed.

The library's `read_current_keymaps()` and `write_current_keymaps()` retain
128-byte maps. `CurrentKeymaps::from_toml()` reads raw version 1 and row
versions 2/3; `to_visual_toml()` produces rows and `to_toml()` produces raw
version 1 arrays, including modifier maps when present. `read_snapshot()`
exposes all four maps and sleep time.
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
