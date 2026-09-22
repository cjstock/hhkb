//! Human-readable US-layout rows, translated losslessly to the raw layer API.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use super::{CurrentKeymaps, Keymap};

struct Row {
    name: &'static str,
    labels: &'static [&'static str],
    first: usize,
}

// Each physical row runs left-to-right while the captured map runs backwards.
// Index 0 and indices 61..128 are not represented by physical keys.
const ROWS: [Row; 5] = [
    Row {
        name: "number_row",
        labels: &[
            "Esc", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "\\", "`",
        ],
        first: 60,
    },
    Row {
        name: "q_row",
        labels: &[
            "Tab", "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]", "Delete",
        ],
        first: 45,
    },
    Row {
        name: "home_row",
        labels: &[
            "Ctrl", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "Enter",
        ],
        first: 31,
    },
    Row {
        name: "shift_row",
        labels: &[
            "LShift", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "RShift", "Fn",
        ],
        first: 18,
    },
    Row {
        name: "bottom_row",
        labels: &["LAlt", "LDiamond", "Space", "RDiamond", "RAlt"],
        first: 5,
    },
];

// Standard Keyboard/Keypad-page usages; HHKB's observed Fn token is 01.
// Vendor-specific media/shortcut values are intentionally left as raw bytes.
const ACTIONS: &[(u8, &str)] = &[
    (0x01, "Fn"),
    (0x28, "Enter"),
    (0x29, "Escape"),
    (0x2a, "Backspace"),
    (0x2b, "Tab"),
    (0x2c, "Space"),
    (0x2d, "-"),
    (0x2e, "="),
    (0x2f, "["),
    (0x30, "]"),
    (0x31, "\\"),
    (0x33, ";"),
    (0x34, "'"),
    (0x35, "`"),
    (0x36, ","),
    (0x37, "."),
    (0x38, "/"),
    (0x39, "CapsLock"),
    (0x46, "PrintScreen"),
    (0x47, "ScrollLock"),
    (0x48, "Pause"),
    (0x49, "Insert"),
    (0x4a, "Home"),
    (0x4b, "PageUp"),
    (0x4c, "Delete"),
    (0x4d, "End"),
    (0x4e, "PageDown"),
    (0x4f, "Right"),
    (0x50, "Left"),
    (0x51, "Down"),
    (0x52, "Up"),
    (0x53, "NumLock"),
    (0x54, "KeypadDivide"),
    (0x55, "KeypadMultiply"),
    (0x56, "KeypadSubtract"),
    (0x57, "KeypadAdd"),
    (0x58, "KeypadEnter"),
    (0x63, "KeypadDecimal"),
    (0x65, "Menu"),
    (0xe0, "LCtrl"),
    (0xe1, "LShift"),
    (0xe2, "LAlt"),
    (0xe3, "LMeta"),
    (0xe4, "RCtrl"),
    (0xe5, "RShift"),
    (0xe6, "RAlt"),
    (0xe7, "RMeta"),
];

pub fn action_name(byte: u8) -> String {
    match byte {
        0x04..=0x1d => char::from(b'A' + byte - 4).to_string(),
        0x1e..=0x26 => char::from(b'1' + byte - 0x1e).to_string(),
        0x27 => "0".into(),
        0x3a..=0x45 => format!("F{}", byte - 0x3a + 1),
        0x68..=0x73 => format!("F{}", byte - 0x68 + 13),
        0x59..=0x61 => format!("Keypad{}", byte - 0x59 + 1),
        0x62 => "Keypad0".into(),
        _ => ACTIONS
            .iter()
            .find(|(code, _)| *code == byte)
            .map(|(_, name)| (*name).to_owned())
            .unwrap_or_else(|| format!("raw:0x{byte:02X}")),
    }
}

pub fn action_byte(name: &str) -> Result<u8> {
    let normalized = name.to_ascii_lowercase();
    if let Some(hex) = normalized.strip_prefix("raw:0x") {
        ensure!(
            hex.len() == 2 && hex.bytes().all(|b| b.is_ascii_hexdigit()),
            "Raw action must use exactly two hex digits, e.g. raw:0xAB"
        );
        return Ok(u8::from_str_radix(hex, 16)?);
    }
    let canonical = match normalized.as_str() {
        "esc" => "escape",
        "return" => "enter",
        "ctrl" | "control" | "leftctrl" | "leftcontrol" => "lctrl",
        "rightctrl" | "rightcontrol" => "rctrl",
        "leftshift" => "lshift",
        "rightshift" => "rshift",
        "leftalt" => "lalt",
        "rightalt" => "ralt",
        "leftmeta" | "leftgui" | "lsuper" | "lwin" | "lcommand" => "lmeta",
        "rightmeta" | "rightgui" | "rsuper" | "rwin" | "rcommand" => "rmeta",
        "pgup" => "pageup",
        "pgdn" => "pagedown",
        "backslash" => "\\",
        "grave" | "backtick" => "`",
        "minus" => "-",
        "equal" => "=",
        "leftbracket" => "[",
        "rightbracket" => "]",
        "semicolon" => ";",
        "quote" => "'",
        "comma" => ",",
        "period" => ".",
        "slash" => "/",
        _ => normalized.as_str(),
    };
    for byte in 0..=255 {
        let exported = action_name(byte);
        if !exported.starts_with("raw:") && exported.eq_ignore_ascii_case(canonical) {
            return Ok(byte);
        }
    }
    bail!(
        "Unknown action {name:?}; use a key name such as A, Escape, Home, LCtrl, or an explicit raw:0xAB value"
    )
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisualLayer {
    number_row: Vec<String>,
    q_row: Vec<String>,
    home_row: Vec<String>,
    shift_row: Vec<String>,
    bottom_row: Vec<String>,
}
impl VisualLayer {
    fn rows(&self) -> [&[String]; 5] {
        [
            &self.number_row,
            &self.q_row,
            &self.home_row,
            &self.shift_row,
            &self.bottom_row,
        ]
    }
    fn from_map(map: &Keymap) -> Self {
        let mut rows = ROWS.iter().map(|row| {
            (0..row.labels.len())
                .map(|column| action_name(map.0[row.first - column]))
                .collect()
        });
        Self {
            number_row: rows.next().unwrap(),
            q_row: rows.next().unwrap(),
            home_row: rows.next().unwrap(),
            shift_row: rows.next().unwrap(),
            bottom_row: rows.next().unwrap(),
        }
    }
    fn apply(&self, layer: &str, preserved: Vec<u8>) -> Result<Keymap> {
        ensure!(
            preserved.len() == 68,
            "preservation.{layer} needs 68 bytes (index 0, then 61..127); found {}",
            preserved.len()
        );
        let mut map = [0; 128];
        map[0] = preserved[0];
        map[61..].copy_from_slice(&preserved[1..]);
        for (row, actions) in ROWS.iter().zip(self.rows()) {
            ensure!(
                actions.len() == row.labels.len(),
                "{layer}.{} needs {} entries; found {}. Physical positions: {}",
                row.name,
                row.labels.len(),
                actions.len(),
                row.labels.join(", ")
            );
            for (column, action) in actions.iter().enumerate() {
                map[row.first - column] = action_byte(action).with_context(|| {
                    format!(
                        "{layer}.{}, position {} (physical {})",
                        row.name,
                        column + 1,
                        row.labels[column]
                    )
                })?;
            }
        }
        Ok(Keymap(map))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preservation {
    base: Vec<u8>,
    #[serde(rename = "fn")]
    fn_layer: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisualConfig {
    schema_version: u32,
    layout: String,
    model: String,
    serial: String,
    mode: u8,
    base: VisualLayer,
    #[serde(rename = "fn")]
    fn_layer: VisualLayer,
    preservation: Preservation,
}

fn validate_layout(model: &str, layout: &str) -> Result<()> {
    ensure!(
        layout == "hhkb-us",
        "Unsupported layout {layout:?}; expected hhkb-us"
    );
    ensure!(
        model.starts_with("PD-KB800"),
        "Row format supports the PD-KB800 US layout; model {model:?} requires a raw version 1 file"
    );
    Ok(())
}

pub(super) fn from_toml(text: &str) -> Result<CurrentKeymaps> {
    let config: VisualConfig = toml::from_str(text)?;
    ensure!(
        config.schema_version == 2,
        "Unsupported visual schema version {}",
        config.schema_version
    );
    validate_layout(&config.model, &config.layout)?;
    let raw = CurrentKeymaps {
        schema_version: 1,
        model: config.model,
        serial: config.serial,
        mode: config.mode,
        base: config.base.apply("base", config.preservation.base)?,
        fn_layer: config.fn_layer.apply("fn", config.preservation.fn_layer)?,
    };
    raw.validate()?;
    Ok(raw)
}

fn quoted(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

pub(super) fn to_toml(raw: &CurrentKeymaps) -> Result<String> {
    raw.validate()?;
    validate_layout(&raw.model, "hhkb-us")?;
    let mut out = format!(
        "# Physical labels appear above each row; strings below are assigned actions.\n# Base and Fn are independent. Zero/unknown actions retain their raw bytes.\nschema_version = 2\nlayout = \"hhkb-us\"\nmodel = {}\nserial = {}\nmode = {}\n",
        quoted(&raw.model),
        quoted(&raw.serial),
        raw.mode
    );
    for (layer, map) in [("base", &raw.base), ("fn", &raw.fn_layer)] {
        out.push_str(&format!("\n[{layer}]\n"));
        let visual = VisualLayer::from_map(map);
        for (row, actions) in ROWS.iter().zip(visual.rows()) {
            let (labels, values) = render_row(row, actions);
            out.push_str(&format!("{labels}\n{values}\n"));
        }
    }
    out.push_str("\n# Preserve these bytes: index 0, then indices 61..127. No physical key labels.\n[preservation]\n");
    for (layer, map) in [("base", &raw.base), ("fn", &raw.fn_layer)] {
        let values: Vec<_> = std::iter::once(map.0[0])
            .chain(map.0[61..].iter().copied())
            .map(|byte| format!("0x{byte:02X}"))
            .collect();
        out.push_str(&format!("{layer} = [{}]\n", values.join(", ")));
    }
    Ok(out)
}

/// Describe every byte change without writing to the keyboard.
pub fn diff_keymaps(before: &CurrentKeymaps, after: &CurrentKeymaps) -> Result<Vec<String>> {
    before.validate()?;
    after.validate()?;
    validate_layout(&before.model, "hhkb-us")?;
    ensure!(
        before.model == after.model && before.serial == after.serial && before.mode == after.mode,
        "Cannot compare configurations for different devices or modes"
    );
    let mut changes = Vec::new();
    for (layer, old, new) in [
        ("Base", &before.base, &after.base),
        ("Fn", &before.fn_layer, &after.fn_layer),
    ] {
        // Report physical changes in the same order as the visual document.
        for row in &ROWS {
            for (column, label) in row.labels.iter().enumerate() {
                let index = row.first - column;
                if old.0[index] != new.0[index] {
                    changes.push(format!(
                        "{layer}: physical {label} ({}): {} -> {}",
                        row.name,
                        action_name(old.0[index]),
                        action_name(new.0[index])
                    ));
                }
            }
        }
        for index in std::iter::once(0).chain(61..128) {
            if old.0[index] != new.0[index] {
                changes.push(format!(
                    "{layer}: preserved byte {index}: raw:0x{:02X} -> raw:0x{:02X}",
                    old.0[index], new.0[index]
                ));
            }
        }
    }
    Ok(changes)
}

#[derive(Deserialize)]
struct RowSpans {
    number_row: toml::Spanned<Vec<String>>,
    q_row: toml::Spanned<Vec<String>>,
    home_row: toml::Spanned<Vec<String>>,
    shift_row: toml::Spanned<Vec<String>>,
    bottom_row: toml::Spanned<Vec<String>>,
}
impl RowSpans {
    fn rows(&self) -> [&toml::Spanned<Vec<String>>; 5] {
        [
            &self.number_row,
            &self.q_row,
            &self.home_row,
            &self.shift_row,
            &self.bottom_row,
        ]
    }
}
#[derive(Deserialize)]
struct DocumentSpans {
    base: RowSpans,
    #[serde(rename = "fn")]
    fn_layer: RowSpans,
}

fn render_row(row: &Row, actions: &[String]) -> (String, String) {
    let tokens: Vec<_> = actions.iter().map(|s| quoted(s)).collect();
    let widths: Vec<_> = tokens
        .iter()
        .zip(row.labels)
        .map(|(token, label)| token.len().max(label.len()))
        .collect();
    let labels = row
        .labels
        .iter()
        .zip(&widths)
        .map(|(label, width)| format!("{label:<width$}"))
        .collect::<Vec<_>>()
        .join("  ");
    let values: String = tokens
        .iter()
        .zip(&widths)
        .enumerate()
        .map(|(index, (token, width))| {
            if index + 1 == tokens.len() {
                token.clone()
            } else {
                format!("{token},{}", " ".repeat(width - token.len() + 1))
            }
        })
        .collect();
    (
        format!(
            "# keys:{}{}",
            " ".repeat(row.name.len() + 5 - 7),
            labels.trim_end()
        ),
        format!("{} = [{values}]", row.name),
    )
}

// TOML comments begin outside basic/literal strings. Keep notes when a
// multiline array is collapsed, including inline notes on individual entries.
fn comment_notes(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut notes = Vec::new();
    let mut quote: Option<(u8, bool)> = None;
    let mut index = 0;
    while index < bytes.len() {
        if let Some((delimiter, triple)) = quote {
            if delimiter == b'"' && bytes[index] == b'\\' {
                index += 2;
                continue;
            }
            if bytes[index] == delimiter
                && (!triple || bytes.get(index..index + 3) == Some(&[delimiter; 3]))
            {
                index += if triple { 3 } else { 1 };
                quote = None;
                continue;
            }
        } else if bytes[index] == b'#' {
            let end = text[index..]
                .find('\n')
                .map_or(text.len(), |offset| index + offset);
            notes.push(text[index..end].trim_end());
            index = end;
            continue;
        } else if matches!(bytes[index], b'"' | b'\'') {
            let delimiter = bytes[index];
            let triple = bytes.get(index..index + 3) == Some(&[delimiter; 3]);
            quote = Some((delimiter, triple));
            index += if triple { 3 } else { 1 };
            continue;
        }
        index += 1;
    }
    notes
}

/// Align rows and regenerate physical labels while retaining user comments.
/// Raw version 1 documents are upgraded to a complete row document.
pub fn format_toml(text: &str) -> Result<String> {
    let raw = CurrentKeymaps::from_toml(text)?;
    let document: toml::Value = toml::from_str(text)?;
    if document["schema_version"].as_integer() == Some(1) {
        // Keep notes when upgrading the old, structurally different schema.
        let comments = comment_notes(text);
        let prefix = if comments.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", comments.join("\n"))
        };
        return Ok(prefix + &to_toml(&raw)?);
    }
    let spans: DocumentSpans = toml::from_str(text)?;
    let mut edits = Vec::new();
    for (map, layer) in [(&raw.base, &spans.base), (&raw.fn_layer, &spans.fn_layer)] {
        let visual = VisualLayer::from_map(map);
        for ((row, values), span) in ROWS.iter().zip(visual.rows()).zip(layer.rows()) {
            let range = span.span();
            let line_start = text[..range.start].rfind('\n').map_or(0, |i| i + 1);
            let (labels, rendered) = render_row(row, values);
            let preceding = text[..line_start].trim_end_matches('\n');
            let prev_start = preceding.rfind('\n').map_or(0, |i| i + 1);
            let start = if preceding[prev_start..].trim_start().starts_with("# keys:") {
                prev_start
            } else {
                line_start
            };
            let notes = comment_notes(&text[range.clone()]);
            let prefix = if notes.is_empty() {
                String::new()
            } else {
                format!("{}\n", notes.join("\n"))
            };
            edits.push((start..range.end, format!("{prefix}{labels}\n{rendered}")));
        }
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = text.to_owned();
    for (range, replacement) in edits {
        output.replace_range(range, &replacement);
    }
    ensure!(
        CurrentKeymaps::from_toml(&output)? == raw,
        "Formatting changed configuration bytes"
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> CurrentKeymaps {
        let reports: Vec<Vec<u8>> = include_str!("../../tests/fixtures/startup.hex")
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .map(|byte| u8::from_str_radix(byte, 16).unwrap())
                    .collect()
            })
            .collect();
        let mut maps = Vec::new();
        let mut bytes = Vec::new();
        for report in reports {
            if report[..3] == [0x55, 0x55, 0x87] {
                bytes.extend_from_slice(&report[6..6 + report[5] as usize]);
                if report[4] == 0xc3 {
                    maps.push(Keymap::try_from(std::mem::take(&mut bytes)).unwrap());
                }
            }
        }
        CurrentKeymaps {
            schema_version: 1,
            model: "PD-KB800WNS".into(),
            serial: "TEST-SERIAL".into(),
            mode: 0,
            base: maps[0].clone(),
            fn_layer: maps[2].clone(),
        }
    }

    #[test]
    fn captured_maps_export_as_physical_rows_and_round_trip() {
        let raw = baseline();
        assert_eq!(
            CurrentKeymaps::from_toml(include_str!("../../tests/fixtures/rows.toml")).unwrap(),
            raw
        );
        let text = raw.to_visual_toml().unwrap();
        let doc: VisualConfig = toml::from_str(&text).unwrap();
        assert_eq!(
            doc.base.number_row,
            [
                "Escape", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "=", "\\", "`"
            ]
        );
        assert_eq!(
            doc.base.q_row,
            [
                "Tab",
                "Q",
                "W",
                "E",
                "R",
                "T",
                "Y",
                "U",
                "I",
                "O",
                "P",
                "[",
                "]",
                "Backspace"
            ]
        );
        assert_eq!(
            doc.base.home_row,
            [
                "LCtrl", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'", "Enter"
            ]
        );
        assert_eq!(
            doc.base.shift_row,
            [
                "LShift", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/", "RShift", "Fn"
            ]
        );
        assert_eq!(
            doc.base.bottom_row,
            ["LAlt", "Fn", "Space", "RMeta", "RAlt"]
        );
        assert_eq!(CurrentKeymaps::from_toml(&text).unwrap(), raw);
        assert_eq!(format_toml(&text).unwrap(), text);
        let row_lines: Vec<_> = text
            .lines()
            .filter(|line| {
                ROWS.iter()
                    .any(|row| line.starts_with(&format!("{} = [", row.name)))
            })
            .collect();
        assert_eq!(row_lines.len(), 10);
        assert!(row_lines.iter().all(|line| line.ends_with(']')));
        // Every physical index occurs once; preservation never overlaps a row.
        let mut indices: Vec<_> = ROWS
            .iter()
            .flat_map(|row| (0..row.labels.len()).map(|column| row.first - column))
            .collect();
        indices.sort();
        assert_eq!(indices, (1..61).collect::<Vec<_>>());
    }

    #[test]
    fn all_action_bytes_and_nonphysical_bytes_are_lossless() {
        for byte in 0..=255 {
            assert_eq!(action_byte(&action_name(byte)).unwrap(), byte);
        }
        let mut raw = baseline();
        raw.serial = "serial \"quoted\" #tag\nline".into();
        for offset in [0u8, 128] {
            raw.base = Keymap(std::array::from_fn(|i| (i as u8).wrapping_add(offset)));
            raw.fn_layer = Keymap(std::array::from_fn(|i| {
                255 - (i as u8).wrapping_add(offset)
            }));
            let text = raw.to_visual_toml().unwrap();
            assert_eq!(CurrentKeymaps::from_toml(&text).unwrap(), raw);
            assert_eq!(format_toml(&text).unwrap(), text);
        }
        assert_eq!(action_name(0), "raw:0x00");
        assert_eq!(action_name(0xe8), "raw:0xE8");
    }

    #[test]
    fn accepts_aliases_and_rejects_unknown_or_invalid_raw_actions() {
        for (alias, byte) in [
            ("a", 4),
            ("esc", 0x29),
            ("CTRL", 0xe0),
            ("LeftShift", 0xe1),
            ("LWin", 0xe3),
            ("pgdn", 0x4e),
            ("Backslash", 0x31),
            ("raw:0xab", 0xab),
        ] {
            assert_eq!(action_byte(alias).unwrap(), byte);
        }
        for name in [
            "None",
            "Default",
            "raw:0x0",
            "raw:0x100",
            "raw:0xGG",
            "raw:256",
            "a typo",
        ] {
            assert!(action_byte(name).is_err());
        }
    }

    #[test]
    fn row_errors_identify_layer_and_physical_position_before_writing() {
        let mut doc: toml::Value = toml::from_str(&baseline().to_visual_toml().unwrap()).unwrap();
        doc["base"]["home_row"].as_array_mut().unwrap().pop();
        let error = CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap())
            .unwrap_err()
            .to_string();
        assert!(error.contains("base.home_row needs 13 entries; found 12"));
        assert!(error.contains("Ctrl, A, S, D"));
        let mut doc: toml::Value = toml::from_str(&baseline().to_visual_toml().unwrap()).unwrap();
        doc["fn"]["q_row"][5] = toml::Value::String("Typo".into());
        let error = format!(
            "{:#}",
            CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap()).unwrap_err()
        );
        assert!(error.contains("fn.q_row, position 6 (physical T)"));
    }

    #[test]
    fn rejects_missing_rows_layers_preservation_and_unsupported_layouts() {
        let good: toml::Value = toml::from_str(&baseline().to_visual_toml().unwrap()).unwrap();
        for field in ["base", "fn", "preservation"] {
            let mut doc = good.clone();
            doc.as_table_mut().unwrap().remove(field);
            assert!(CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap()).is_err());
        }
        for field in ROWS.iter().map(|r| r.name) {
            let mut doc = good.clone();
            doc["fn"].as_table_mut().unwrap().remove(field);
            assert!(CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap()).is_err());
        }
        for (field, value) in [
            ("layout", toml::Value::String("jis".into())),
            ("model", toml::Value::String("PD-KB820".into())),
            ("mode", toml::Value::Integer(1)),
            ("schema_version", toml::Value::Integer(3)),
        ] {
            let mut doc = good.clone();
            doc[field] = value;
            assert!(CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap()).is_err());
        }
        for value in [
            toml::Value::Array(vec![toml::Value::Integer(0); 67]),
            toml::Value::Array(vec![toml::Value::Integer(256); 68]),
        ] {
            let mut doc = good.clone();
            doc["preservation"]["base"] = value;
            assert!(CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap()).is_err());
        }
    }

    #[test]
    fn independent_row_edits_and_diff_cover_all_preserved_bytes() {
        let raw = baseline();
        let mut doc: toml::Value = toml::from_str(&raw.to_visual_toml().unwrap()).unwrap();
        doc["base"]["q_row"][5] = toml::Value::String("Y".into());
        let edited = CurrentKeymaps::from_toml(&toml::to_string(&doc).unwrap()).unwrap();
        assert_eq!(edited.fn_layer, raw.fn_layer);
        let mut expected = raw.base.clone();
        expected.0[40] = 0x1c;
        assert_eq!(edited.base, expected);
        assert_eq!(
            diff_keymaps(&raw, &edited).unwrap(),
            ["Base: physical T (q_row): T -> Y"]
        );
        let mut edited = raw.clone();
        edited.fn_layer.0[40] = 0x1c;
        edited.base.0[0] = 0xab;
        edited.fn_layer.0[127] = 0xff;
        assert_eq!(
            diff_keymaps(&raw, &edited).unwrap(),
            [
                "Base: preserved byte 0: raw:0x00 -> raw:0xAB",
                "Fn: physical T (q_row): T -> Y",
                "Fn: preserved byte 127: raw:0x00 -> raw:0xFF"
            ]
        );
        assert!(diff_keymaps(&raw, &raw).unwrap().is_empty());
        edited.serial = "other".into();
        assert!(diff_keymaps(&raw, &edited).is_err());
    }

    #[test]
    fn formatting_keeps_comments_and_condenses_multiline_rows() {
        let raw = baseline();
        let text = raw.to_visual_toml().unwrap();
        let start = text.find("home_row = [").unwrap();
        let end = start + text[start..].find(']').unwrap() + 1;
        let messy = format!(
            "# My custom layout\n{}home_row=[\n# Keep inner note\n\"ctrl\",\"a\",\"s\",\"d\",\"f\",\"g\",\"h\",\"j\",\"k\",\"l\",\";\",\"'\",\"enter\"\n] # Keep this note\n{}",
            &text[..start],
            &text[end + 1..]
        );
        let formatted = format_toml(&messy).unwrap();
        assert!(formatted.starts_with("# My custom layout\n"));
        assert!(formatted.contains("] # Keep this note"));
        assert!(formatted.contains("# Keep inner note"));
        assert_eq!(CurrentKeymaps::from_toml(&formatted).unwrap(), raw);
        assert_eq!(format_toml(&formatted).unwrap(), formatted);
    }
}
