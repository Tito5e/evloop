pub use keyboard_types::{Code as KeyCode, Location as KeyLocation, NamedKey};
use smol_str::SmolStr;

pub enum NativeKeyCode {
    Unidentified,
    Android(u32),
    MacOS(u16),
    Windows(u16),
    Xkb(u32),
    Ohos(u32),
}

pub enum NativeKey {
    Unidentified,
    Android(u32),
    MacOS(u16),
    Windows(u16),
    Xkb(u32),
    Web(SmolStr),
    Ohos(u32),
}

pub enum PhysicalKey {
    Code(KeyCode),
    Unidentified(NativeKeyCode),
}

pub enum Key<Str = SmolStr> {
    Named(NamedKey),

    Character(Str),

    Unidentified(NativeKey),

    Dead(Option<char>),
}

impl Key {
    pub fn to_text(&self) -> Option<&str> {
        match self {
            Key::Named(action) => match action {
                NamedKey::Enter => Some("\r"),
                NamedKey::Backspace => Some("\x08"),
                NamedKey::Tab => Some("\t"),
                NamedKey::Escape => Some("\x1b"),
                _ => None,
            },
            Key::Character(ch) => Some(ch.as_str()),
            _ => None,
        }
    }
}
