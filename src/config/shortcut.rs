//! HHKB shortcuts: a key byte plus the USB modifier bit mask in its companion map.
use anyhow::{Result, bail, ensure};

use super::{action_byte, action_name};

const MODIFIERS: [&str; 8] = [
    "LCtrl", "LShift", "LAlt", "LMeta", "RCtrl", "RShift", "RAlt", "RMeta",
];

/// Parse one key or simultaneous modifiers plus at most one key.
/// A modifier-only chord such as Ctrl+Shift uses the zero action byte.
pub fn parse_assignment(name: &str) -> Result<(u8, u8)> {
    if name.to_ascii_lowercase().starts_with("raw:") && name.contains('/') {
        let (key, mask) = name.split_once('/').unwrap();
        ensure!(
            mask.to_ascii_lowercase().starts_with("0x"),
            "Raw modifier mask must use 0xNN"
        );
        return Ok((action_byte(key)?, action_byte(&format!("raw:{mask}"))?));
    }
    // Try complete labels first: the official label "Keypad +" contains '+'.
    if let Ok(key) = action_byte(name) {
        return Ok((key, 0));
    }
    let mut remainder = name;
    let mut modifiers = 0;
    loop {
        let (part, tail) = match remainder.split_once('+') {
            Some(parts) => parts,
            None => (remainder, ""),
        };
        let byte = action_byte(part.trim())?;
        ensure!(
            (0xe0..=0xe7).contains(&byte),
            "Shortcut prefixes must be modifiers; found {part:?}. Only one ordinary key is allowed"
        );
        let bit = 1 << (byte - 0xe0);
        ensure!(modifiers & bit == 0, "Duplicate shortcut modifier {part:?}");
        modifiers |= bit;
        if tail.is_empty() {
            ensure!(!remainder.contains('+'), "Shortcut has an empty final key");
            return Ok((0, modifiers));
        }
        if tail.trim().eq_ignore_ascii_case("(M)") {
            return Ok((0, modifiers));
        }
        // A modifier at the end belongs in the mask, rather than the key byte.
        if let Ok(key) = action_byte(tail.trim())
            && !(0xe0..=0xe7).contains(&key)
        {
            ensure!(key != 1, "Fn cannot be combined with shortcut modifiers");
            return Ok((key, modifiers));
        }
        remainder = tail;
        if remainder.trim().is_empty() {
            bail!("Shortcut has an empty final key");
        }
    }
}

/// Format both bytes losslessly, including unusual combinations read from a device.
pub fn assignment_name(key: u8, modifiers: u8) -> String {
    if modifiers == 0 {
        return action_name(key);
    }
    let mut parts: Vec<String> = MODIFIERS
        .iter()
        .enumerate()
        .filter(|(index, _)| modifiers & (1 << index) != 0)
        .map(|(_, name)| (*name).to_owned())
        .collect();
    // A raw pair is unambiguous for Fn/modifier key bytes and bypasses chord
    // restrictions when preserving unusual assignments from device snapshots.
    if key == 1 || (0xe0..=0xe7).contains(&key) {
        return format!("raw:0x{key:02X}/0x{modifiers:02X}");
    }
    parts.push(if key == 0 {
        "(M)".into()
    } else {
        action_name(key)
    });
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shortcuts_with_left_right_modifiers_and_one_key() {
        for (name, key, mask) in [
            ("Ctrl+Shift+C", 0x06, 0x03),
            ("RCtrl+RAlt+Delete", 0x4c, 0x50),
            ("Command + Alt + T", 0x17, 0x0c),
            ("LAlt+Keypad +", 0x57, 0x04),
            ("Ctrl+Shift", 0, 0x03),
            ("Ctrl+(M)", 0, 0x01),
            ("LCtrl+LShift+LAlt+LMeta+RCtrl+RShift+RAlt+RMeta+A", 4, 0xff),
            ("Ctrl+BrightnessUp", 0xec, 1),
            ("Shift", 0xe1, 0),
        ] {
            assert_eq!(parse_assignment(name).unwrap(), (key, mask), "{name}");
        }
        assert_eq!(assignment_name(0x17, 0x0c), "LAlt+LMeta+T");
        assert_eq!(assignment_name(0, 3), "LCtrl+LShift+(M)");
    }

    #[test]
    fn rejects_multiple_keys_duplicate_modifiers_and_malformed_shortcuts() {
        for name in [
            "Ctrl+A+S",
            "A+Ctrl",
            "Ctrl+LCtrl+C",
            "Ctrl+",
            "+C",
            "Ctrl++C",
            "Ctrl+Fn",
            "Ctrl+Unknown",
            "raw:0x01/0x100",
            "raw:0x01/0xGG",
            "raw:0x01/01",
            "raw:0x01/0x01/0x02",
        ] {
            assert!(parse_assignment(name).is_err(), "{name}");
        }
    }

    #[test]
    fn all_action_and_modifier_pairs_round_trip_losslessly() {
        for key in 0..=255 {
            for mask in 0..=255 {
                assert_eq!(
                    parse_assignment(&assignment_name(key, mask)).unwrap(),
                    (key, mask)
                );
            }
        }
    }
}
