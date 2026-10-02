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

Exports use schema version 1, with five complete rows in both `[base]` and
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
those positions. Those three entries must read `Reserved`.
Exact row lengths are required, and errors identify the layer, row, and
physical position. An optional `[preservation]` section records only nonzero
bytes without physical key labels from the four maps. Omitted bytes are zero.

`hhkb diff [path]` defaults to `hhkb.toml`, checks device identity, and reports
physical assignments and preservation bytes that would change. It performs
read sessions without issuing configuration-write commands.
`hhkb format [path]` also defaults to `hhkb.toml` and works without a keyboard.
It canonicalizes names, regenerates aligned label comments, and keeps user
comments. Notes within multiline arrays move above the resulting one-line row.
The formatter validates first and replaces the file atomically.

The row layout currently supports US-layout PD-KB800 models in mode `0`.
Export device model, serial, layout, and mode identify the target keyboard.
The [complete sanitized example](tests/fixtures/rows.toml) shows the format.
See [row-format details](docs/row-format.md).

Import validates the schema, rows/actions, mode, and connected device identity
before writing. It saves the current configuration beside the input as
`<filename>.backup-<Unix timestamp in nanoseconds>.toml` and aborts if that save
fails. It takes a fresh preservation snapshot, writes the requested layers
alongside the requested modifier maps and unchanged sleep time, and verifies
all four maps, mode, and sleep time in a fresh read session. A failed write or
readback is an error; configuration writes are not retried and no rollback is
claimed.

The library's `read_current_keymaps()` and `write_current_keymaps()` retain
128-byte maps. `CurrentKeymaps::from_toml()` and `to_toml()` read and write the
row format. `read_snapshot()` exposes all four maps and sleep time.
See [verified protocol behavior](docs/protocol.md) and
[hardware acceptance](docs/hardware-acceptance.md).

## LSP and Neovim plugin design

This is the agreed design and implementation tracker for the next phase. The
usage above describes the **current CLI**; items marked Planned below are not
implemented yet. Done means present in the current code, Verify means the
behavior or device values still need evidence, Open means a design choice
remains, Deferred means intentionally outside the first version, and Excluded
means deliberately omitted. Keep this section current as work lands.

### Profile format and names

| Feature or sub-feature | Status | Agreed behavior |
| --- | --- | --- |
| Base/Fn physical rows, shortcuts, reserved Fn positions, schema version 1 | Done | Keep the current compact TOML arrays and five physical rows per layer. |
| Sparse opaque-byte preservation | Done | Export only nonzero nonphysical bytes; omit `[preservation]` when all are zero. |
| Portable profile metadata | Planned | Remove parsed `layout`, `model`, and `serial` fields. Export them as informational comments only; a serial never binds a profile to one keyboard. |
| Current mode support | Done | Only mode 0 and the current US physical row layout are supported today. |
| Mode-specific layouts | Deferred | Design and support different layouts for each mode later. |
| Editable sleep setting | Planned | Add top-level integer `sleep_minutes` to export, validation, Diff, Apply, backup, and Restore. |
| Supported sleep values | Verify | Extract the exact choices from the official tool; completion and validation accept only those choices. |
| Short canonical action names | Planned | Prefer familiar names such as `Esc`, `Bksp`, `Del`, `PgUp`, `PrtSc`, and `VolUp` in new exports. Continue accepting full names and aliases. |
| Preserve spelling during formatting | Planned | Formatting aligns rows and physical-label comments without rewriting a valid action or shortcut spelling. An explicit normalize action may use the short canonical names. |
| Portable Apply and opaque bytes | Planned | Before writing, check the connected keyboard's physical layout. Normal Apply keeps that keyboard's nonphysical bytes rather than copying opaque bytes from the profile. Exact opaque-byte restoration belongs to Restore Backup. |

The existing schema-1 parser still requires the metadata fields and the current
formatter still canonicalizes aliases. Those are migration tasks, not properties
of the planned format. The file's row structure will identify the supported
physical layout; the connected keyboard supplies the actual model and serial.
There is no profile-binding step. Layout detection from device information must
be verified before enabling writes across different models.

### Standalone language server

| Feature or sub-feature | Status | Agreed behavior |
| --- | --- | --- |
| Editor-independent LSP process | Planned | Keep TOML editing features in a standalone server usable beyond Neovim; it does not need keyboard access. |
| File association | Planned | Attach by `*.hhkb.toml`, including empty or temporarily invalid files. Validate the contents after attachment and coexist with general TOML tooling. |
| Diagnostics | Planned | Diagnose while typing and again on save, with ranges on the exact bad assignment or field. Cover row lengths, unknown actions, shortcut syntax, reserved positions, schema, and sleep values. |
| Save an invalid draft | Planned | Allow ordinary saves with diagnostics; skip formatting until the document is valid. Block Apply until validation passes. |
| Action and shortcut completion | Planned | Complete keys, official action aliases, shortcut modifiers and components. Make common-first versus full-catalog ordering configurable; common-first is the default. |
| Hover | Planned | Show the physical position, complete assignment, meaning, and encoded key/modifier bytes. |
| Quick fixes | Planned | Offer clear corrections such as a misspelled action name or restoring `Reserved` at an Fn position. |
| Formatting | Planned | Expose the alias-preserving row formatter through the LSP. The Neovim plugin enables format-on-save by default, with a setting to disable it. |
| Cursor display | Excluded | Do not add a persistent per-key cursor/status display while editing TOML. |

### Neovim layout view

| Feature or sub-feature | Status | Agreed behavior |
| --- | --- | --- |
| Window | Planned | Open over the TOML in a floating window by default; allow a configurable split instead. |
| Live source | Planned | Reflect unsaved buffer changes. Keep valid keys visible while marking incomplete or invalid entries rather than dropping the whole view. |
| Layer display | Planned | Show one layer at a time; `Tab` toggles Base/Fn. Keycaps show assigned actions only. |
| Keycap labels | Planned | Use short canonical action names. Render shortcut modifiers as symbols: `⌃` Ctrl, `⇧` Shift, `⌥` Alt, and `⌘` Meta (for example `⌃⇧C`). Show left/right modifier detail only for the selected key. |
| Selection detail | Planned | Show the selected key's physical position and full assignment outside the keycap. |
| Vim navigation | Planned | `h`/`l` move across keys, `j`/`k` move to the nearest key on the adjacent row, `0`/`^`/`$` jump within a row, and `gg`/`G` move to the first/last row. Support counts. |
| Jump to TOML | Planned | `Enter` jumps from the selected key to its assignment string in the source buffer. |
| Device diff highlights | Planned | Highlight changed keys in the layout when a device Diff or Apply preview is open. |

### Device commands and first use

The plugin exposes commands such as `:HHKBLayout`, `:HHKBDiff`,
`:HHKBApply`, `:HHKBExport`, `:HHKBRefresh`, `:HHKBRestore`, and
`:HHKBInspect`. These names are provisional. Window-local navigation has
defaults; global Neovim key mappings are left to the user.

| Feature or sub-feature | Status | Agreed behavior |
| --- | --- | --- |
| Explicit device reads | Planned | Read the keyboard only for Export, Refresh, Diff, Apply, Restore, Inspect, or an explicit recovery action. The layout view itself uses the buffer. |
| Export destination | Planned | Both CLI and plugin default to `~/.config/hhkb/profiles/default.hhkb.toml`. Create the profile directory if needed; never silently overwrite an existing profile. Ask for another name on collision. |
| Offline first profile | Planned | If Export finds no connected keyboard, create a profile from the user's current layout as a bundled template, without device identity fields. It remains editable offline. |
| Bundled template source | Verify | Capture the user's current assignments for the shipped template and remove device-identifying metadata. |
| Inspect Device | Planned | Show connection status, model, serial, running firmware, mode, DIP switches, and sleep setting in a floating panel. |
| Text diff | Done in CLI; planned in plugin | The CLI already reports key, shortcut, and opaque-byte changes. Add sleep changes and present the list in Neovim; highlight changed keycaps too. |
| Apply flow | Planned | Validate the current buffer, format and save it if valid, read the device, check layout compatibility, show text and layout diffs, then request confirmation in Neovim's bottom message area. Make a backup, write, read back, and report verification. Do not write when there are no changes. |
| Refresh From Keyboard | Planned | Back up the open buffer, including unsaved edits, ask for confirmation, then replace it with a fresh device export. |
| Restore Backup | Planned | List backups beside the current profile, preview the chosen backup against the device, save a copy of the current buffer, replace the open file, then require confirmation before exact restoration and readback verification. |
| Profile picker | Planned | List `*.hhkb.toml` profiles, show their filenames and available device comments, open a selection, and allow Diff/Apply from it. |
| Profile search paths | Planned | Search configured paths first, then the open profile's directory, then the project root and its `profiles/` directory, then `~/.config/hhkb/profiles`. Search shallowly by default and remove duplicates. |

Apply to a different serial is allowed when the physical layout is compatible;
serial comments are informational. The current mode-0 constraint still applies.
The existing CLI already backs up imports and verifies readback, but it
currently requires identity matching, preserves the device's sleep value, and
writes opaque profile bytes. These behaviors must change for the portable
profile design.

### Recovery

| Feature or sub-feature | Status | Agreed behavior |
| --- | --- | --- |
| Partial write or readback mismatch | Planned | Stop writing and attempt a fresh read-only snapshot. Show what actually differs from the intended profile and the backup. |
| Device cannot be read after failure | Planned | Keep the backup and report that the keyboard state is unknown. |
| Manual recovery | Planned | Offer Retry Apply and Restore Backup, each with a fresh device read, diff, and confirmation. Never perform an automatic rollback write. |
| Exact-restore provenance | Open | Decide how to confirm a backup belongs to the connected keyboard before restoring its opaque bytes, without making ordinary profiles serial-bound. |

### Remaining CLI and device questions

| Item | Status | What remains |
| --- | --- | --- |
| Physical-layout detection | Verify | Confirm how the connected keyboard exposes or implies its physical layout so a portable profile can be checked before writing. |
| Bare CLI Diff and Format paths | Open | Decide whether commands without a path should use the new default profile path, like Export. |

## Packet Sniffing / Wireshark

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
