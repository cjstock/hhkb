# Row format (schema version 1)

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
usages. HHKB-specific assignments are verified from the official Windows tool; see
[the action catalog and provenance](actions.md). They are not inferred from
numeric similarity to consumer-page or keyboard-page codes.

## Lossless translation

Schema version 1 requires `layout = "hhkb-us"`, a supported PD-KB800 model identity,
mode 0, and every row of both layers. Unknown fields,
missing rows, bad row lengths, invalid actions, unsupported layouts, and wrong
byte values are errors before the device is opened for import or diff.

Loading a document resolves each action to a key byte and a modifier byte at
the same fixed position, then fills omitted nonphysical bytes with zero and
restores any listed nonzero values. Saving reverses this process. There is no
base/Fn coupling.

`[preservation]` is omitted when all nonphysical bytes are zero. When needed,
`base` and `fn` contain only nonzero entries, keyed by their raw map index:

```toml
[preservation]
base = { "0" = 0xAB, "73" = 0x42 }
fn = { "127" = 0xFF }
```

Only index 0 and indices 61 through 127 belong here. Omitted indices are zero.
`base_modifiers` and `fn_modifiers` use the same form; omitted modifier entries
are zero. Every physical key's shortcut mask is encoded in its row assignment.

Canonical names cover every individual action code in the official Windows
Professional tool 2.0.1. See [the full action catalog](actions.md). The modifier
names distinguish left/right: `LCtrl`, `RCtrl`, `LShift`, `RShift`, `LAlt`,
`RAlt`, `LMeta`, `RMeta`. Zero normally serializes as `InvalidKey` (the official label),
which stores a literal zero and does not introduce inheritance. Unknown values
serialize as `raw:0xNN`; raw escapes require exactly two hex digits and still
work for every byte value, including named codes. Friendly labels accept
spaces and underscores as well as case differences.

## Shortcuts

A row entry can be a plain key or a shortcut. Write modifiers before
one key, separated by `+`: `"Ctrl+Shift+C"`, `"RCtrl+RAlt+Delete"`, or
`"LMeta+T"`. `Ctrl`, `Shift`, `Alt`, `Win`, and `Command` default to the left
side; `L`/`R` prefixes choose a side explicitly. Up to all eight modifier bits
can be combined with one ordinary key. A modifier-only shortcut such as
`"Ctrl+Shift"` or `"Ctrl+(M)"` stores key byte `00` with the selected
modifier bits, matching the official tool's `(M)` shortcut action. `Fn` is a
separate keyboard action and cannot be combined with modifiers in friendly
syntax. Standalone `"LCtrl"` remains an ordinary key assignment.

Canonical export and format use explicit names in bit order, for example
`"LCtrl+LShift+C"`. Modifier-only shortcuts export as `"LCtrl+(M)"`
or `"LCtrl+LShift+(M)"`. Replacing that string with `"C"` clears the modifier
mask at that physical key. The key byte and mask are read, written, and checked
at the same index in the base or Fn pair of maps. The modifier bits are `01`
LCtrl, `02` LShift, `04` LAlt, `08` LMeta, `10` RCtrl, `20` RShift, `40`
RAlt, and `80` RMeta. Unknown combinations retain a lossless
`"raw:0xNN/0xMM"` pair when friendly syntax would be ambiguous.

Import checks the connected firmware
before any write if modifier bytes change; the verified minimum is running
HYBRID application firmware A0.48.

On the Fn layer, physical Q (`q_row[1]`, byte 44), Ctrl (`home_row[0]`, byte
31), and RShift (`shift_row[11]`, byte 7) are reserved. Exports show
`"Reserved"` at those positions. Both the key byte and shortcut modifier
byte must be zero; imports reject changes to either byte before
writing. `"Reserved"` is rejected at every other position; these three entries
must use it exactly.

## Formatting and previews

`hhkb format` works offline. It validates the complete file, canonicalizes
aliases, aligns action columns with generated `# keys:` label comments, and
keeps each physical row on a single line. It retains other comments and moves
notes inside multiline arrays above their row.
A temporary file beside the original is synced and renamed over it only after
formatting succeeds; original permissions and symlink targets are retained.

`hhkb diff` compares a validated file against a fresh current keyboard snapshot.
It reports physical keys and shortcut masks by layer/row, and every changed
nonphysical key or modifier byte. It never sends sleep-setting, map-write, or
commit commands.

## Validation for this change

The captured baseline exports to all five expected physical rows and returns
exactly the same maps. Tests additionally cover every byte value, nonzero
preservation bytes, escaping and aliases, independent layer edits, unsupported
layouts/models, missing rows/layers, contextual errors, and formatter
idempotence with retained comments. Offline file tests check formatting and
that invalid input leaves the original file untouched.

A live `hhkb diff hhkb.toml` reported `No changes.` A separate preview changing
only the base row's physical T assignment to Y reported exactly one byte
change at physical T. That preview was not imported into the keyboard.
