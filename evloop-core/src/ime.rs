use bitflags::bitflags;

use crate::{
	dpi::{Position, Size},
	error::ImeSurroundingTextError,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub enum ImePurpose {
	#[default]
	Normal,
	Password,
	Terminal,
	Number,
	Phone,
	Url,
	Email,
	Pin,
	Date,
	Time,
	DateTime,
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
	pub struct ImeHint: u32 {
		const NONE = 0;
		const COMPLETION = 0x1;
		const SPELLCHECK = 0x2;
		const AUTO_CAPITALIZATION = 0x4;
		const LOWERCASE = 0x8;
		const UPPERCASE = 0x10;
		const TITLECASE = 0x20;
		const HIDDEN_TEXT = 0x40;
		const SENSITIVE_DATA = 0x80;
		const LATIN = 0x100;
		const MULTILINE = 0x200;
	}
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ImeSurroundingText {
	text: String,
	cursor: usize,
	anchor: usize,
}

impl ImeSurroundingText {
	pub const MAX_TEXT_BYTES: usize = 4000;

	pub fn new(
		text: String,
		cursor: usize,
		anchor: usize,
	) -> Result<Self, ImeSurroundingTextError> {
		let text = if text.len() < Self::MAX_TEXT_BYTES {
			text
		} else {
			return Err(ImeSurroundingTextError::TextTooLong);
		};

		let cursor = if text.is_char_boundary(cursor) && cursor <= text.len() {
			cursor
		} else {
			return Err(ImeSurroundingTextError::CursorBadPosition);
		};

		let anchor = if text.is_char_boundary(anchor) && anchor <= text.len() {
			anchor
		} else {
			return Err(ImeSurroundingTextError::AnchorBadPosition);
		};

		Ok(Self { text, cursor, anchor })
	}

	#[inline]
	#[must_use]
	pub fn into_text(self) -> String {
		self.text
	}

	#[inline]
	#[must_use]
	pub fn text(&self) -> &str {
		&self.text
	}

	#[inline]
	#[must_use]
	pub fn cursor(&self) -> usize {
		self.cursor
	}

	#[inline]
	#[must_use]
	pub fn anchor(&self) -> usize {
		self.anchor
	}
}

#[derive(Debug, PartialEq, Clone)]
pub enum ImeRequest {
	Enable(ImeEnableRequest),
	Update(ImeRequestData),
	Disable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImeEnableRequest {
	capabilities: ImeCapabilities,
	request_data: ImeRequestData,
}

impl ImeEnableRequest {
	pub fn new(capabilities: ImeCapabilities, request_data: ImeRequestData) -> Option<Self> {
		if capabilities.cursor_area() ^ request_data.cursor_area.is_some() {
			return None;
		}

		if capabilities.hint_and_purpose() ^ request_data.hint_and_purpose.is_some() {
			return None;
		}

		if capabilities.surrounding_text() ^ request_data.surrounding_text.is_some() {
			return None;
		}

		Some(Self { capabilities, request_data })
	}

	pub const fn capabilities(&self) -> &ImeCapabilities {
		&self.capabilities
	}

	pub const fn request_data(&self) -> &ImeRequestData {
		&self.request_data
	}

	pub fn into_raw(self) -> (ImeCapabilities, ImeRequestData) {
		(self.capabilities, self.request_data)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImeCapabilities(ImeCapabilitiesFlags);

impl ImeCapabilities {
	pub fn new() -> Self {
		Self::default()
	}

	pub const fn with_hint_and_purpose(self) -> Self {
		Self(self.0.union(ImeCapabilitiesFlags::HINT_AND_PURPOSE))
	}

	pub const fn without_hint_and_purpose(self) -> Self {
		Self(self.0.difference(ImeCapabilitiesFlags::HINT_AND_PURPOSE))
	}

	pub const fn hint_and_purpose(&self) -> bool {
		self.0.contains(ImeCapabilitiesFlags::HINT_AND_PURPOSE)
	}

	pub const fn with_cursor_area(self) -> Self {
		Self(self.0.union(ImeCapabilitiesFlags::CURSOR_AREA))
	}

	pub const fn without_cursor_area(self) -> Self {
		Self(self.0.difference(ImeCapabilitiesFlags::CURSOR_AREA))
	}

	pub const fn cursor_area(&self) -> bool {
		self.0.contains(ImeCapabilitiesFlags::CURSOR_AREA)
	}

	pub const fn with_surrounding_text(self) -> Self {
		Self(self.0.union(ImeCapabilitiesFlags::SURROUNDING_TEXT))
	}

	pub const fn without_surrounding_text(self) -> Self {
		Self(self.0.difference(ImeCapabilitiesFlags::SURROUNDING_TEXT))
	}

	pub const fn surrounding_text(&self) -> bool {
		self.0.contains(ImeCapabilitiesFlags::SURROUNDING_TEXT)
	}
}

bitflags! {
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
	pub(crate) struct ImeCapabilitiesFlags: u8 {
		const HINT_AND_PURPOSE = 1 << 0;
		const CURSOR_AREA = 1 << 1;
		const SURROUNDING_TEXT = 1 << 2;
	}
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct ImeRequestData {
	pub hint_and_purpose: Option<(ImeHint, ImePurpose)>,
	pub cursor_area: Option<(Position, Size)>,
	pub surrounding_text: Option<ImeSurroundingText>,
}

impl ImeRequestData {
	pub fn with_hint_and_purpose(self, hint: ImeHint, purpose: ImePurpose) -> Self {
		Self { hint_and_purpose: Some((hint, purpose)), ..self }
	}

	pub fn with_cursor_area(self, position: Position, size: Size) -> Self {
		Self { cursor_area: Some((position, size)), ..self }
	}

	pub fn with_surrounding_text(self, surrounding_text: ImeSurroundingText) -> Self {
		Self { surrounding_text: Some(surrounding_text), ..self }
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ime {
	Enabled,
	Preedit(String, Option<(usize, usize)>),
	Commit(String),
	DeleteSurrounding { before_bytes: usize, after_bytes: usize },
	Disabled,
}
