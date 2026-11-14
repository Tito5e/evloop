//! Android-specific types.

pub use ndk;
pub use raw_window_handle as rwh_06;

pub use crate::glue::{
	AndroidApp, AndroidAppWaker,
	input::{
		InputEvent, KeyAction, KeyEvent, Keycode, MotionAction, MotionEvent, Pointer, ToolType,
	},
	key_map::{KeyCharacterMap, KeyMapChar},
};

use std::ops::Range;

/// The editor state reported by the IME (`WindowEvent::TextInputStateChanged`), with UTF-8 byte
/// offsets.
///
/// This is Android's native IME model. Most applications should use the `Ime::*` events, which
/// evloop derives from this state; this event is for applications that prefer to mirror the
/// IME's editor state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextInputState {
	pub text: String,
	/// `start <= end`; empty for a cursor.
	pub selection: Range<usize>,
	/// The composing region, if any.
	pub compose: Option<Range<usize>>,
}

/// The action key pressed on the soft keyboard (`EditorInfo.IME_ACTION_*`),
/// `WindowEvent::ImeAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImeAction {
	Unspecified,
	None,
	Go,
	Search,
	Send,
	Next,
	Done,
	Previous,
	Unknown(i32),
}

impl From<i32> for ImeAction {
	fn from(action: i32) -> Self {
		match action {
			0 => Self::Unspecified,
			1 => Self::None,
			2 => Self::Go,
			3 => Self::Search,
			4 => Self::Send,
			5 => Self::Next,
			6 => Self::Done,
			7 => Self::Previous,
			other => Self::Unknown(other),
		}
	}
}

/// The kinds of window insets (`WindowInsetsCompat.Type`), for `Window::insets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsetType {
	CaptionBar,
	DisplayCutout,
	Ime,
	MandatorySystemGestures,
	NavigationBars,
	StatusBars,
	SystemBars,
	SystemGestures,
	TappableElement,
	Waterfall,
}

impl InsetType {
	pub(crate) fn to_raw(self) -> u8 {
		use crate::sys::*;
		match self {
			Self::CaptionBar => GAMECOMMON_INSETS_TYPE_CAPTION_BAR,
			Self::DisplayCutout => GAMECOMMON_INSETS_TYPE_DISPLAY_CUTOUT,
			Self::Ime => GAMECOMMON_INSETS_TYPE_IME,
			Self::MandatorySystemGestures => GAMECOMMON_INSETS_TYPE_MANDATORY_SYSTEM_GESTURES,
			Self::NavigationBars => GAMECOMMON_INSETS_TYPE_NAVIGATION_BARS,
			Self::StatusBars => GAMECOMMON_INSETS_TYPE_STATUS_BARS,
			Self::SystemBars => GAMECOMMON_INSETS_TYPE_SYSTEM_BARS,
			Self::SystemGestures => GAMECOMMON_INSETS_TYPE_SYSTEM_GESTURES,
			Self::TappableElement => GAMECOMMON_INSETS_TYPE_TAPABLE_ELEMENT,
			Self::Waterfall => GAMECOMMON_INSETS_TYPE_WATERFALL,
		}
	}
}
