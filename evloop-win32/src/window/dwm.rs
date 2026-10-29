/// Describes a system-drawn backdrop material of a window.
///
/// For a detailed explanation, see [`DWM_SYSTEMBACKDROP_TYPE docs`].
///
/// [`DWM_SYSTEMBACKDROP_TYPE docs`]: https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackdropType {
	#[default]
	Auto = 0,
	None = 1,
	MainWindow = 2,
	TransientWindow = 3,
	TabbedWindow = 4,
}

/// Describes a color used by Windows
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Color(u32);

impl Color {
	// Special constant only valid for the window border and therefore modeled using Option<Color>
	// for user facing code
	pub(crate) const NONE: Color = Color(0xfffffffe);
	/// Use the system's default color
	pub const SYSTEM_DEFAULT: Color = Color(0xffffffff);

	/// Create a new color from the given RGB values
	pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
		Self((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
	}
}

impl Default for Color {
	fn default() -> Self {
		Self::SYSTEM_DEFAULT
	}
}

/// Describes how the corners of a window should look like.
///
/// For a detailed explanation, see [`DWM_WINDOW_CORNER_PREFERENCE docs`].
///
/// [`DWM_WINDOW_CORNER_PREFERENCE docs`]: https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_window_corner_preference
#[repr(i32)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CornerPreference {
	#[default]
	Default = 0,
	DoNotRound = 1,
	Round = 2,
	RoundSmall = 3,
}
