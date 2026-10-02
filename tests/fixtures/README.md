These fixtures retain only complete, 64-byte configuration reports from the
local official-tool captures. Each hex line is one OUT (`aa aa`) or IN (`55 55`)
report in chronological order. HIDAPI's zero report-ID prefix is not present in
USB captures; tests add it to outgoing packets.

Device-info bytes 6..46 (model, revision, serial) were replaced with a fictional
TEST-KEYBOARD/TEST-SERIAL identity. USB addresses, URB identifiers, timestamps,
keyboard input reports, and unrelated traffic were removed. Keymap bytes,
sleep values, command framing, firmware bytes, and reply lengths are unchanged.
No captures are needed at test runtime.

| Fixture | Source in `caps/` | Content |
| --- | --- | --- |
| `startup.hex` | `session-start-20261001T041135Z.hhkb.json` | Complete startup read session |
| `baseline-write.hex` | `sleep-restore-20-to-30-20260923T005030Z.hhkb.json` | First write session only, restoring 30-minute sleep |
| `swap-write.hex` | `ctrl-escape-swap-20261001T041516Z.hhkb.json` | Ctrl/Escape swap write |
| `base-restore-write.hex` | `ctrl-escape-restore-20261001T042648Z.hhkb.json` | Base restoration, leaving the Fn difference |
| `fn-restored.hex` | `ctrl-escape-restore-readback-20261001T042834Z.hhkb.json` | Manual Fn restoration write and independent readback |

The tests replay the baseline exchange byte for byte, check the swap's three
specific byte changes, and confirm manual Fn restoration returns all four maps
to baseline. Fault tests use altered copies of these reports.

`rows.toml` is the human-readable schema version 1 representation of the
captured startup base/Fn maps. Its serial is fictional; the public PD-KB800WNS
model name identifies the supported US layout. The nonphysical map bytes are
all zero, so no preservation section is needed. Tests check that it decodes to
exactly the captured maps.

`official-action-codes.hex` is a code-only catalog extracted from the official
Windows Professional Keymap Tool 2.0.1, rather than a USB capture. It covers
English/Japanese assignable keys and English shortcut component keys. See
[the action provenance](../../docs/actions.md#evidence-and-validation) for the
installer hash, source methods, and distinction from hardware verification.
