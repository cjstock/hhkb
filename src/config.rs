use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyboardConfig {
    pub profile: Profile,
}

impl KeyboardConfig {
    pub fn new(hhkb_base_layer: &[u8; 60]) -> Self {
        todo!()
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
