#!/usr/bin/env bash

# Load the usbmon module
echo "Loading usbmon kernel module..."
sudo modprobe usbmon

# Change permissions for usbmon devices to allow non-root Wireshark capture
echo "Adjusting permissions for /dev/usbmon*..."
sudo chmod a+r /dev/usbmon*

# Find the HHKB device
# PFU's vendor ID is 04fe. We look for devices in the product ID range 0020-0022
hhkb_info=$(lsusb | grep -E -i "04fe:002[0-2]")

if [ -z "$hhkb_info" ]; then
    echo "WARNING: HHKB device not found in lsusb! Make sure it is connected via USB."
    exit 1
fi

echo "Found HHKB Device:"
echo "  $hhkb_info"

# Parse Bus and Device address
# Example: Bus 001 Device 014: ID 04fe:0021 ...
bus=$(echo "$hhkb_info" | awk '{print $2}')
device=$(echo "$hhkb_info" | awk '{print $4}' | sed 's/://')

# Strip leading zeros for the Wireshark filter (e.g. 001 -> 1, 014 -> 14)
bus_num=$((10#$bus))
device_num=$((10#$device))

echo ""
echo "=========================================================="
echo " Wireshark Packet Sniffing Instructions"
echo "=========================================================="
echo "1. Open Wireshark (non-root is fine if you rebooted)."
echo "2. Select interface: usbmon${bus_num}"
echo "3. Apply the following display filter at the top:"
echo "   usb.bus_id == ${bus_num} && usb.device_address == ${device_num}"
echo "=========================================================="
