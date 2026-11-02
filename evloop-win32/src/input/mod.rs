mod device_id;
mod ime;
mod keyboard;
mod raw_input;

pub use ime::*;
pub use keyboard::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum DeviceEvents {
	/// Report device events regardless of window focus.
	Always,
	/// Only capture device events while the window is focused.
	#[default]
	WhenFocused,
	/// Never capture device events.
	Never,
}
