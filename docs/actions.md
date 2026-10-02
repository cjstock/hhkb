# Configuration action names

Names are case-insensitive. Spaces and underscores are optional in friendly
names: `VolumeUp`, `volume_up`, and the official label `Volume Up` resolve to
the same byte. Raw escapes (`raw:0xNN`, exactly two hex digits) remain accepted
for every byte. Export, format, and diff use canonical names.

## Ordinary keys

- `A`–`Z`, `0`–`9`, `F1`–`F24`.
- Unshifted punctuation: `-`, `=`, `[`, `]`, `\`, `;`, `'`, backtick,
  `,`, `.`, `/`. Named aliases include `Minus`, `Equal`, `LeftBracket`,
  `RightBracket`, `Backslash`, `Semicolon`, `Quote`, `Backtick`/`Grave`,
  `Comma`, `Period`, and `Slash`.
- `Enter` (`Return`), `Escape` (`Esc`), `Backspace`, `Tab`, `Space`, `CapsLock`.
- `PrintScreen`, `ScrollLock`, `Pause` (`Pause/Break`, `Break`), `Insert`,
  `Home`, `PageUp` (`PgUp`), `Delete`, `End`, `PageDown` (`PgDn`),
  `Right`, `Left`, `Down`, `Up`.
- `NumLock` (`Clear/Numlock`, `Clear`), `Keypad0`–`Keypad9`, `KeypadDivide`,
  `KeypadMultiply`, `KeypadSubtract`, `KeypadAdd`, `KeypadEnter`,
  `KeypadDecimal`. Official labels such as `Keypad /`, `Keypad *`,
  `Keypad -`, `Keypad +`, and `Keypad .` also work.
- `LCtrl`, `RCtrl`, `LShift`, `RShift`, `LAlt`, `RAlt`, `LMeta`, `RMeta`.
  Aliases include `Ctrl`/`Control`, `LeftControl`/`RightControl`,
  `LeftShift`/`RightShift`, `LeftAlt`/`RightAlt`, and left/right `Win`,
  `Super`, `GUI`, `Command`, or `Meta` names (e.g. `LWin`, `LeftGUI`).
- `Menu` (`Application`, `Application Key`).

## Additional official codes

| Byte | Canonical name | Other accepted names |
| --- | --- | --- |
| `00` | `InvalidKey` | `Invalid Key`, `Invalid` |
| `01` | `Fn` | `Function` |
| `02` | `Fn2` | `Function2`, `LeftFn` |
| `32` | `NonUSHash` | |
| `64` | `NonUSBackslash` | |
| `66` | `Power` | |
| `78` | `Stop` | |
| `87` | `Ro` | `International1` |
| `88` | `Kana` | `International2` |
| `89` | `Yen` | `International3` |
| `8A` | `Henkan` | `International4`, `Convert`, `RDiamond`, `RightDiamond` |
| `8B` | `Muhenkan` | `International5`, `NonConvert`, `LDiamond`, `LeftDiamond` |
| `90` | `MacKana` | `Lang1`, `KanaMac` |
| `91` | `MacEisu` | `Lang2`, `Eisu`, `Alphanumeric characters (Mac)` |
| `E8` | `VolumeDown` | `Volume Down`, `VolumeDn`, `VolDown` |
| `E9` | `VolumeUp` | `Volume Up`, `VolUp` |
| `EA` | `Mute` | `VolumeMute` |
| `EB` | `Eject` | |
| `EC` | `BrightnessUp` | `Brightness Up` |
| `ED` | `BrightnessDown` | `Brightness Down`, `BrightnessDn` |

`InvalidKey` preserves a literal zero; it is not a default or inheritance
instruction. `Fn2` preserves the second Function code separately: the official
tool uses `02` for the left Fn on newer firmware, and labels both Fn codes
“Function”. Existing `Fn` assignments remain `01`.

`Reserved` is a position-specific label for physical Q, Ctrl, and RShift on
the Fn layer. It stores a zero byte, and those positions cannot be remapped.
See [the row rules](row-format.md#shortcuts).

Some labels depend on keyboard mode/layout. The official tool labels `8A` and
`8B` Henkan/Muhenkan or Right/Left Meta; its English HHK defaults put those
codes at the diamond positions. They are distinct from `E3`/`E7` (`LMeta`/
`RMeta`). Naming Japanese codes does not enable Japanese physical row layouts;
the supported configuration layout remains `hhkb-us`, mode 0.

These names also serve as the ordinary key in shortcut assignments, such as
`Ctrl+Shift+C`. See [shortcut syntax](row-format.md#shortcuts). `Stop` is the
keyboard-page Stop action, not a consumer media playback command. No
unsupported Play/NextTrack actions are inferred.

## Evidence and validation

The source is PFU's [Professional Keymap Tool 2.0.1 Windows installer](https://origin.pfultd.com/downloads/hhkb/win/HHKBKeymapTool_201.exe),
linked from the [official downloads page](https://happyhackingkb.com/download/).
Inspected on 2026-10-01; installer SHA-256:
`4da6eb4c1b6f7510e7e0f629080ba984710b7b850fe3ba5b8ce4e921287af54a`.
The installer was extracted offline; neither it nor its application was run.

In `HHKBKeymapTool.exe`, `KeyboardLibrary.GetSortedEnglishUsageIdList`,
`GetSortedJapaneseUsageIdList`, and `GetShortCutEnglishUsageIdList` identify
the assignable codes. `ToolUtility.GetDescriptionForTooltip` and the English
`ToolUtility_TooltipDescription_*` resources establish special-key labels.
`KeyboardDriver.ChangeLeftFnKeyCode` and the default keymap resources distinguish
the two Function codes. Japanese punctuation names follow their standard HID
International usages; `32` and `64` use the standard non-US usage names.

The extracted catalog is retained as a small
[code-only fixture](../tests/fixtures/official-action-codes.hex). Tests require
friendly names for all its codes, pin special labels to exact bytes, and check
all 256 byte values through import/export/format without loss. These tests
verify encoding; the newly named assignments have not each been exercised on
hardware. PFU lists firmware A0.48 or later for the HYBRID tool's new actions.
