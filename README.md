# HHKB Professional Hybrid Key Remapper (Linux)

An unofficial key remapping tool and library for the **Happy Hacking Keyboard (HHKB) Professional Hybrid** on Linux, written in Rust.

This tool communicates directly with the keyboard over its USB HID programming interface (using the `hidapi` crate) to read and modify keymaps, profiles, and DIP switch states without needing the official Windows/macOS keymap tool.

## Features

- **Read Keyboard Info**: Retrieve HHKB serial number, firmware versions, type number, and currently running firmware.
- **Query DIP Switches**: Read the hardware DIP switch states.
- **Export Keymaps**: Retrieve the base and Fn layers for different mode layouts (HHK, Mac, Windows).
- **Configure Remappings**: Support for custom TOML-based key layout configuration (in progress).

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
