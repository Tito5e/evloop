use core::fmt;

use evloop_core::{
	event::ElementState,
	keyboard::{KeyLocation, LogicalKey, PhysicalKey},
};
use smol_str::SmolStr;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyEvent {
	pub raw_key: PhysicalRawKey,
	pub physical_key: PhysicalKey,
	pub logical_key: LogicalKey,
	pub text: Option<SmolStr>,
	pub location: KeyLocation,
	pub state: ElementState,
	pub repeat: bool,
	pub text_with_all_modifiers: Option<SmolStr>,
	pub key_without_modifiers: LogicalKey,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalRawKey(u16);

impl fmt::Debug for LogicalRawKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let raw = self.0;
		f.debug_tuple("LogicalRawKey").field(&format_args!("0x{raw:04X}")).finish()
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalRawKey(u16);

impl PhysicalRawKey {
	pub fn new(raw: u16) -> Self {
		Self(raw)
	}
}

impl fmt::Debug for PhysicalRawKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let raw = self.0;
		f.debug_tuple("PhysicalRawKey").field(&format_args!("0x{raw:04X}")).finish()
	}
}
