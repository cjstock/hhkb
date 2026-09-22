# Verified raw keymap protocol

Evidence comes from the official tool captures listed in
[the fixture provenance](../tests/fixtures/README.md), plus the Linux hardware
acceptance described in [hardware-acceptance.md](hardware-acceptance.md).
The supported device is USB `04fe:0021`, programming interface 2, mode byte `0`.
Other mode/selector relationships are unverified and are rejected by this API.

## Reports and observed exchanges

USB reports are 64 bytes. HIDAPI writes include an additional zero report-ID
prefix (65 bytes total). OUT starts `aa aa <command>`; IN starts
`55 55 <command> 00`. Reads use a five-second timeout. Short reads/writes,
nonzero response byte 3, bad headers, and mismatched command IDs fail the
operation. Captured response lengths are checked for info, DIP, mode, sleep,
and keymap blocks.

`01 00 01 00` follows the OUT header to begin an exchange; `01 00 01 01`
ends it. These values bracket the tool's exchanges even while its UI stays
open. Calling them session open/close describes their observed placement;
their complete device-side semantics remain uncertain.

The read session sends commands `01 02 05 09`, reads the four maps using `87`,
then sends `06 01`. Sleep command `09` returns one byte at reply offset 6.
For `87`, OUT bytes after the command are `00 02 <selector0> <selector1>`.
Each map returns three reports. IN offset 4 holds block IDs `41 82 c3`,
offset 5 holds lengths `58 58 12` (decimal), and offset 6 begins map data.
Together these contain exactly 128 bytes, without echoed selectors.

The four selector pairs, in captured order, are:

| Selectors | API use |
| --- | --- |
| `00/00` | Mode 0 base map |
| `10/00` | Preserved companion map |
| `00/01` | Mode 0 Fn map |
| `10/01` | Preserved companion map |

A complete fresh read session precedes a write session. The write exchange
sends `01 02 05 06 08`, twelve `86` reports (three per map in the same selector
order), then `04 07 01`. Every OUT report receives its matching command reply
before the next OUT report. Command `08` sends `00 01 <existing sleep byte>`;
the implementation pauses one second after its reply. Each `86` payload begins
with block ID and length, followed by chunks of **two selectors plus 128 map
bytes**. Its block IDs are `41 82 c3`; lengths are `59 59 12` (decimal).
Command `07` sends `00 01 <mode>`; only the captured mode value zero is used.
The full meanings of `04`, `07`, and the `10` selectors remain unverified.

Only the requested base and Fn arrays replace the snapshot arrays. All bytes,
including zeroes, are treated literally. Captured Ctrl/Escape swapping changed
base indices 31 and 60 **and** Fn index 60. Restoring only the base left the Fn
change behind; manually restoring Fn completed the original configuration.
This implementation does not reproduce that automatic coupling or infer a
meaning for zero.

On any failure, configuration writes stop without automatic retry. A session
close is attempted; cleanup failures are reported along with the original
error. Pending map reports can prevent a matching closure reply. Partial
changes are possible; cleanup is not rollback. Successful writes are followed
by a new read session that verifies all four maps, sleep time, mode, and device
info. Any mismatch or failed readback is an error.

All library operations share a session lock. Direct HID use through
`AsRef<HidDevice>` bypasses it and must not overlap library calls. The older
`base_layer(mode)` and `fn_layer(mode)` signatures remain available; they now
reject unsupported modes instead of guessing selectors.
