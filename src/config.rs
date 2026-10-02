use serde::{Deserialize, Serialize};

mod shortcut;
mod visual;
pub use shortcut::{assignment_name, parse_assignment};
pub use visual::{action_byte, action_name, diff_keymaps, format_toml};

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

/// Current-mode key and modifier maps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentKeymaps {
    pub schema_version: u32,
    pub model: String,
    pub serial: String,
    pub mode: u8,
    pub base: Keymap,
    pub fn_layer: Keymap,
    pub modifiers: ModifierKeymaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierKeymaps {
    pub base: Keymap,
    pub fn_layer: Keymap,
}

// Physical positions whose Fn-layer entries cannot be changed by the official tool.
pub(crate) const FN_RESERVED: [(usize, &str); 3] = [(44, "Q"), (31, "Ctrl"), (7, "RShift")];

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
        for (index, physical) in FN_RESERVED {
            anyhow::ensure!(
                self.fn_layer.0[index] == 0 && self.modifiers.fn_layer.0[index] == 0,
                "Fn physical {physical} (byte {index}) is Reserved and must have zero key and modifier bytes"
            );
        }
        Ok(())
    }

    pub fn from_toml(text: &str) -> anyhow::Result<Self> {
        visual::from_toml(text)
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        visual::to_toml(self)
    }
}
