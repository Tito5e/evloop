use core::fmt;

use bitflags::bitflags;
pub use keyboard_types::{Code as KeyCode, Location as KeyLocation, NamedKey};
use smol_str::SmolStr;

use crate::{event::ElementState, keyboard};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModifiersKeyState {
	Pressed,
	#[default]
	Unknown,
}

bitflags! {
	#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct ModifiersKeys: u8 {
		const LSHIFT   = 0b0000_0001;
		const RSHIFT   = 0b0000_0010;
		const LCONTROL = 0b0000_0100;
		const RCONTROL = 0b0000_1000;
		const LALT     = 0b0001_0000;
		const RALT     = 0b0010_0000;
		const LMETA    = 0b0100_0000;
		const RMETA    = 0b1000_0000;
	}
}

// TODO: move to each backend crate
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEvent {
	// pub raw_key: NativeKey,
	pub physical_key: PhysicalKey,
	pub logical_key: keyboard::LogicalKey,
	pub text: Option<SmolStr>,
	pub location: keyboard::KeyLocation,
	pub state: ElementState,
	pub repeat: bool,
	pub text_with_all_modifiers: Option<SmolStr>,
	pub key_without_modifiers: keyboard::LogicalKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PhysicalKey {
	Code(KeyCode),
	Unidentified,
}

impl From<KeyCode> for PhysicalKey {
	#[inline]
	fn from(value: KeyCode) -> Self {
		PhysicalKey::Code(value)
	}
}

impl From<PhysicalKey> for KeyCode {
	#[inline]
	fn from(value: PhysicalKey) -> Self {
		match value {
			PhysicalKey::Code(code) => code,
			PhysicalKey::Unidentified => KeyCode::Unidentified,
		}
	}
}

impl PartialEq<KeyCode> for PhysicalKey {
	#[inline]
	fn eq(&self, other: &KeyCode) -> bool {
		match self {
			Self::Code(code) => code == other,
			_ => false,
		}
	}
}

impl PartialEq<PhysicalKey> for KeyCode {
	#[inline]
	fn eq(&self, other: &PhysicalKey) -> bool {
		other == self
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LogicalKey<Str = SmolStr> {
	Named(NamedKey),

	Character(Str),

	Unidentified,

	Dead(Option<char>),
}

impl LogicalKey<SmolStr> {
	pub fn as_ref(&self) -> LogicalKey<&str> {
		match self {
			LogicalKey::Named(a) => LogicalKey::Named(*a),
			LogicalKey::Character(ch) => LogicalKey::Character(ch.as_str()),
			LogicalKey::Dead(d) => LogicalKey::Dead(*d),
			LogicalKey::Unidentified => LogicalKey::Unidentified,
		}
	}
}

impl LogicalKey {
	pub fn to_text(&self) -> Option<&str> {
		match self {
			LogicalKey::Named(action) => match action {
				NamedKey::Enter => Some("\r"),
				NamedKey::Backspace => Some("\x08"),
				NamedKey::Tab => Some("\t"),
				NamedKey::Escape => Some("\x1b"),
				_ => None,
			},
			LogicalKey::Character(ch) => Some(ch.as_str()),
			_ => None,
		}
	}
}

impl From<NamedKey> for LogicalKey {
	#[inline]
	fn from(value: NamedKey) -> Self {
		LogicalKey::Named(value)
	}
}

impl<Str> PartialEq<NamedKey> for LogicalKey<Str> {
	#[inline]
	fn eq(&self, other: &NamedKey) -> bool {
		match self {
			LogicalKey::Named(a) => a == other,
			_ => false,
		}
	}
}

impl<Str: PartialEq<str>> PartialEq<str> for LogicalKey<Str> {
	#[inline]
	fn eq(&self, other: &str) -> bool {
		match self {
			LogicalKey::Character(s) => s == other,
			_ => false,
		}
	}
}

impl<Str: PartialEq<str>> PartialEq<&str> for LogicalKey<Str> {
	#[inline]
	fn eq(&self, rhs: &&str) -> bool {
		self == *rhs
	}
}

bitflags! {
	#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
	pub struct ModifiersState: u32 {
		const SHIFT = 0b100;
		const CONTROL = 0b100 << 3;
		const ALT = 0b100 << 6;
		const META = 0b100 << 9;
	}
}

impl ModifiersState {
	#[inline]
	#[must_use]
	pub fn shift_key(&self) -> bool {
		self.intersects(Self::SHIFT)
	}

	#[inline]
	#[must_use]
	pub fn control_key(&self) -> bool {
		self.intersects(Self::CONTROL)
	}

	#[inline]
	#[must_use]
	pub fn alt_key(&self) -> bool {
		self.intersects(Self::ALT)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
	pub(crate) state: ModifiersState,
	pub(crate) pressed_mods: ModifiersKeys,
}

impl Modifiers {
	pub fn new(state: ModifiersState, pressed_mods: ModifiersKeys) -> Self {
		Self { state, pressed_mods }
	}

	pub fn state(&self) -> ModifiersState {
		self.state
	}

	pub fn lshift_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::LSHIFT)
	}

	pub fn rshift_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::RSHIFT)
	}

	pub fn lalt_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::LALT)
	}

	pub fn ralt_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::RALT)
	}

	pub fn lcontrol_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::LCONTROL)
	}

	pub fn rcontrol_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::RCONTROL)
	}

	pub fn lmeta_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::LMETA)
	}

	pub fn rmeta_state(&self) -> ModifiersKeyState {
		self.mod_state(ModifiersKeys::RMETA)
	}

	fn mod_state(&self, modifier: ModifiersKeys) -> ModifiersKeyState {
		if self.pressed_mods.contains(modifier) {
			ModifiersKeyState::Pressed
		} else {
			ModifiersKeyState::Unknown
		}
	}
}

impl From<ModifiersState> for Modifiers {
	fn from(value: ModifiersState) -> Self {
		Self { state: value, pressed_mods: Default::default() }
	}
}
