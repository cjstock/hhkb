# Row format (schema version 2)

[The complete example](../tests/fixtures/rows.toml) contains sanitized device
identity and the captured baseline assignments. Rows always describe physical
positions, independently of the assigned actions or DIP-switch behavior.

## Physical positions and evidence

| Row | Entries | Raw indices, left to right |
| --- | --- | --- |
| `number_row` | 15 | 60 down to 46 |
| `q_row` | 14 | 45 down to 32 |
| `home_row` | 13 | 31 down to 19 |
| `shift_row` | 13 | 18 down to 6, including Fn |
| `bottom_row` | 5 | 5 down to 1 |

The complete reverse ordering is inferred from the captured base assignments
and the manufacturer's US physical layout. Ctrl at index 31 and Escape at
index 60 were independently checked with the official tool's swap/readback;
T at index 40 was checked separately on both layers in Linux hardware tests.
A byte-for-byte visual/raw round trip validates storage fidelity; it does not
constitute a separate physical test of every key position.

Bottom-row labels identify the Alt and diamond positions. A diamond position
may currently be assigned Fn or another action. The printed Delete position
may currently produce Backspace. Comments continue to describe the physical
position rather than renaming it after an assignment change.

Layout reference: [PFU HYBRID US-layout manual](https://origin.pfultd.com/downloads/hhkb/manual/P3PC-6641-05EN.pdf).
Ordinary action names follow the Keyboard/Keypad page in the
[USB-IF HID Usage Tables](https://www.usb.org/sites/default/files/hut1_3_0.pdf).
The observed HHKB Fn token `01` is treated separately from standard keyboard
usages. Vendor-specific values are not inferred from their numeric similarity
to consumer-page or keyboard-page codes.

## Lossless translation

Version 2 requires `layout = "hhkb-us"`, a supported PD-KB800 model identity,
mode 0, every row of both layers, and the preservation section. Unknown fields,
missing rows, bad row lengths, invalid actions, unsupported layouts, and wrong
byte values are errors before the device is opened for import or diff.

The raw library configuration remains version 1. Loading a visual document
resolves each action to one byte, puts it at the corresponding fixed position,
and restores all nonphysical bytes. Saving reverses this process. There are
no substitutions for zeros or omitted positions and no base/Fn coupling.

`[preservation].base` and `[preservation].fn` each contain exactly 68 bytes:
index 0 followed by indices 61 through 127. All are preserved literally. These
arrays represent only the nonphysical bytes of the two exposed maps; the
companion selector maps are independently preserved from the device's fresh
snapshot by the unchanged protocol write implementation.

Canonical names cover A–Z, 0–9, unshifted punctuation, F1–F24, navigation,
modifiers, common keypad actions, and the observed Fn token. The modifier
names distinguish left/right: `LCtrl`, `RCtrl`, `LShift`, `RShift`, `LAlt`,
`RAlt`, `LMeta`, `RMeta`. Unknown values and zeroes serialize as `raw:0xNN`;
raw escapes require exactly two hex digits and work for every byte value.
No `None`, `Default`, or inheritance token is introduced. Vendor-specific media,
brightness, and shortcut meanings remain unverified, so they keep raw names.

## Formatting and previews

`hhkb format` works offline. It validates the complete file, canonicalizes
aliases, aligns action columns with generated `# keys:` label comments, and
keeps each physical row on a single line. It retains other comments and moves
notes inside multiline arrays above their row. Formatting a version 1 raw
file creates version 2 and keeps its comment notes above the generated file.
A temporary file beside the original is synced and renamed over it only after
formatting succeeds; original permissions and symlink targets are retained.

`hhkb diff` compares a validated file against a fresh current keyboard snapshot.
It reports physical keys by layer/row and also reports every changed
preservation byte. It never sends sleep-setting, map-write, or commit commands.
The old raw file format remains accepted by import and diff, and raw exports
are available through `hhkb export --raw`.

## Validation for this change

The captured baseline exports to all five expected physical rows and returns
exactly the same maps. Tests additionally cover every byte value, nonzero
preservation bytes, escaping and aliases, independent layer edits, unsupported
layouts/models, missing rows/layers, contextual errors, and formatter
idempotence with retained comments. Offline file tests check upgrades and
that invalid input leaves the original file untouched.

The user's existing `hhkb.toml` was upgraded without changing its assignments.
A live `hhkb diff hhkb.toml` reported `No changes.` A separate preview changing
only the base row's physical T assignment to Y reported exactly one byte
change at physical T. That preview was not imported into the keyboard.
