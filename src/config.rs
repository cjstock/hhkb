use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyboardConfig {
    pub profile: Profile,
}

impl KeyboardConfig {
    pub fn new(_hhkb_base_layer: &[u8; 60]) -> Self {
        todo!()
    }
}

/// A complete raw layer. Zeroes are preserved without interpreting their meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap(pub [u8; 128]);

impl TryFrom<Vec<u8>> for Keymap {
    type Error = anyhow::Error;

    fn try_from(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let len = bytes.len();
        Ok(Self(bytes.try_into().map_err(|_| {
            anyhow::anyhow!("Keymap must contain exactly 128 bytes, got {len}")
        })?))
    }
}

impl Serialize for Keymap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Keymap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::try_from(Vec::<u8>::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Version 1 exports only the verified current-mode base and Fn selectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentKeymaps {
    pub schema_version: u32,
    pub model: String,
    pub serial: String,
    pub mode: u8,
    pub base: Keymap,
    #[serde(rename = "fn")]
    pub fn_layer: Keymap,
}

impl CurrentKeymaps {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.schema_version == 1,
            "Unsupported schema version {}",
            self.schema_version
        );
        anyhow::ensure!(
            self.mode == 0,
            "Unsupported mode {}; only mode 0 is verified",
            self.mode
        );
        anyhow::ensure!(
            !self.model.is_empty() && !self.serial.is_empty(),
            "Device model and serial must be nonempty"
        );
        Ok(())
    }

    pub fn from_toml(text: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        self.validate()?;
        Ok(toml::to_string_pretty(self)?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    hhk: Layout,
    mac: Layout,
    windows: Layout,
}

impl Profile {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Layout {
    pub base: Layer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layer {
    pub row0: [Key; 15],
    pub row1: [Key; 14],
    pub row2: [Key; 13],
    pub row3: [Key; 12], // far right 'fn' key is hardcoded
    pub row4: [Key; 5],
}

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Key {
    Fn,
    Meta,
    Space,
    Alt,
    LeftShift,
    Z,
    X,
    C,
    V,
    B,
    N,
    M,
    Comma,
    Period,
    ForwardSlash,
    RightShift,
    Ctrl,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    SemiColon,
    Quote,
    Enter,
    Tab,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    LeftParen,
    RightParen,
    Backspace,
    Escape,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Zero,
    Dash,
    Equal,
    BackSlash,
    BackTick,
}

#[cfg(test)]
mod raw_tests {
    use super::*;
    fn config() -> CurrentKeymaps {
        CurrentKeymaps {
            schema_version: 1,
            model: "test".into(),
            serial: "serial".into(),
            mode: 0,
            base: Keymap(std::array::from_fn(|i| i as u8)),
            fn_layer: Keymap([0; 128]),
        }
    }
    #[test]
    fn raw_toml_round_trip_preserves_every_byte_and_zero() {
        let original = config();
        let text = original.to_toml().unwrap();
        assert!(text.contains("fn = ["));
        assert_eq!(CurrentKeymaps::from_toml(&text).unwrap(), original);
    }
    #[test]
    fn rejects_invalid_versions_modes_identity_lengths_bytes_and_fields() {
        let text = config().to_toml().unwrap();
        for changed in [
            text.replace("schema_version = 1", "schema_version = 2"),
            text.replace("mode = 0", "mode = 1"),
            text.replace("serial = \"serial\"", "serial = \"\""),
            format!("{text}\nunknown = 1\n"),
        ] {
            assert!(CurrentKeymaps::from_toml(&changed).is_err());
        }
        for values in [
            vec!["0"; 127],
            vec!["0"; 129],
            vec!["256"; 128],
            vec!["-1"; 128],
            vec!["1.5"; 128],
            vec!["\"0\""; 128],
        ] {
            let input = format!(
                "schema_version = 1\nmodel = \"test\"\nserial = \"test\"\nmode = 0\nbase = [{}]\nfn = [{}]\n",
                values.join(","),
                vec!["0"; 128].join(",")
            );
            assert!(CurrentKeymaps::from_toml(&input).is_err());
        }
        assert!(Keymap::try_from(vec![0; 127]).is_err());
    }
}
