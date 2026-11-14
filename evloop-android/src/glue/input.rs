//! Owned copies of GameActivity input events.
//!
//! GameActivity delivers input on the Java main thread; the glue copies each event into these
//! types and hands them to the application thread.

pub use ndk::event::Keycode;

use crate::sys::{GameActivityKeyEvent, GameActivityMotionEvent, GameActivityPointerAxes};

/// `MotionEvent.AXIS_*` values used by evloop. Only enabled axes carry values (see
/// [`ENABLED_AXES`]).
pub mod axis {
	pub const X: i32 = 0;
	pub const Y: i32 = 1;
	pub const PRESSURE: i32 = 2;
	pub const ORIENTATION: i32 = 8;
	pub const VSCROLL: i32 = 9;
	pub const HSCROLL: i32 = 10;
	pub const TILT: i32 = 25;
}

/// Axes enabled in addition to X/Y (which GameActivity always reads).
pub(crate) const ENABLED_AXES: &[i32] =
	&[axis::PRESSURE, axis::ORIENTATION, axis::VSCROLL, axis::HSCROLL, axis::TILT];

/// `MotionEvent.ACTION_*`, without the pointer index bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionAction {
	Down,
	Up,
	Move,
	Cancel,
	Outside,
	PointerDown,
	PointerUp,
	HoverMove,
	Scroll,
	HoverEnter,
	HoverExit,
	ButtonPress,
	ButtonRelease,
	Unknown(i32),
}

impl From<i32> for MotionAction {
	fn from(action: i32) -> Self {
		match action & 0xff {
			0 => Self::Down,
			1 => Self::Up,
			2 => Self::Move,
			3 => Self::Cancel,
			4 => Self::Outside,
			5 => Self::PointerDown,
			6 => Self::PointerUp,
			7 => Self::HoverMove,
			8 => Self::Scroll,
			9 => Self::HoverEnter,
			10 => Self::HoverExit,
			11 => Self::ButtonPress,
			12 => Self::ButtonRelease,
			other => Self::Unknown(other),
		}
	}
}

/// `MotionEvent.TOOL_TYPE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolType {
	Unknown,
	Finger,
	Stylus,
	Mouse,
	Eraser,
	Palm,
}

impl From<i32> for ToolType {
	fn from(tool: i32) -> Self {
		match tool {
			1 => Self::Finger,
			2 => Self::Stylus,
			3 => Self::Mouse,
			4 => Self::Eraser,
			5 => Self::Palm,
			_ => Self::Unknown,
		}
	}
}

/// One pointer of a [`MotionEvent`].
#[derive(Debug, Clone, Copy)]
pub struct Pointer {
	axes: GameActivityPointerAxes,
}

impl Pointer {
	#[inline]
	pub fn pointer_id(&self) -> i32 {
		self.axes.id
	}

	#[inline]
	pub fn tool_type(&self) -> ToolType {
		self.axes.toolType.into()
	}

	#[inline]
	pub fn x(&self) -> f32 {
		self.axis_value(axis::X)
	}

	#[inline]
	pub fn y(&self) -> f32 {
		self.axis_value(axis::Y)
	}

	#[inline]
	pub fn pressure(&self) -> f32 {
		self.axis_value(axis::PRESSURE)
	}

	/// The value of a `MotionEvent.AXIS_*`. `0.0` for axes that are not enabled.
	#[inline]
	pub fn axis_value(&self, axis: i32) -> f32 {
		usize::try_from(axis).ok().and_then(|i| self.axes.axisValues.get(i)).copied().unwrap_or(0.0)
	}
}

impl std::fmt::Debug for GameActivityPointerAxes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PointerAxes")
			.field("id", &self.id)
			.field("toolType", &self.toolType)
			.finish()
	}
}

/// A copy of an Android `MotionEvent` (without history).
#[derive(Debug, Clone)]
pub struct MotionEvent {
	device_id: i32,
	source: i32,
	action: i32,
	meta_state: i32,
	button_state: i32,
	action_button: i32,
	pointers: Vec<Pointer>,
}

impl MotionEvent {
	/// Copies the event. The history arrays are not copied.
	pub(crate) unsafe fn from_raw(event: &GameActivityMotionEvent) -> Self {
		let count = (event.pointerCount as usize).min(event.pointers.len());
		Self {
			device_id: event.deviceId,
			source: event.source,
			action: event.action,
			meta_state: event.metaState,
			button_state: event.buttonState,
			action_button: event.actionButton,
			pointers: event.pointers[..count].iter().map(|&axes| Pointer { axes }).collect(),
		}
	}

	#[inline]
	pub fn device_id(&self) -> i32 {
		self.device_id
	}

	/// `InputDevice.SOURCE_*` bits.
	#[inline]
	pub fn source(&self) -> i32 {
		self.source
	}

	#[inline]
	pub fn action(&self) -> MotionAction {
		self.action.into()
	}

	/// The index of the pointer that changed, for `PointerDown` / `PointerUp`.
	#[inline]
	pub fn pointer_index(&self) -> usize {
		((self.action as u32 & 0xff00) >> 8) as usize
	}

	#[inline]
	pub fn pointer_count(&self) -> usize {
		self.pointers.len()
	}

	/// # Panics
	///
	/// If `index >= pointer_count()`.
	#[inline]
	pub fn pointer_at_index(&self, index: usize) -> Pointer {
		self.pointers[index]
	}

	#[inline]
	pub fn pointers(&self) -> impl Iterator<Item = Pointer> + '_ {
		self.pointers.iter().copied()
	}

	#[inline]
	pub fn meta_state(&self) -> i32 {
		self.meta_state
	}

	/// `MotionEvent.BUTTON_*` bits currently pressed.
	#[inline]
	pub fn button_state(&self) -> i32 {
		self.button_state
	}

	/// The button that changed, for `ButtonPress` / `ButtonRelease`.
	#[inline]
	pub fn action_button(&self) -> i32 {
		self.action_button
	}
}

/// `KeyEvent.ACTION_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
	Down,
	Up,
	Multiple,
	Unknown(i32),
}

/// A copy of an Android `KeyEvent`.
#[derive(Debug, Clone, Copy)]
pub struct KeyEvent {
	raw: GameActivityKeyEvent,
}

impl KeyEvent {
	pub(crate) fn from_raw(raw: GameActivityKeyEvent) -> Self {
		Self { raw }
	}

	#[inline]
	pub fn device_id(&self) -> i32 {
		self.raw.deviceId
	}

	#[inline]
	pub fn source(&self) -> i32 {
		self.raw.source
	}

	#[inline]
	pub fn action(&self) -> KeyAction {
		match self.raw.action {
			0 => KeyAction::Down,
			1 => KeyAction::Up,
			2 => KeyAction::Multiple,
			other => KeyAction::Unknown(other),
		}
	}

	#[inline]
	pub fn key_code(&self) -> Keycode {
		self.raw.keyCode.into()
	}

	#[inline]
	pub fn scan_code(&self) -> i32 {
		self.raw.scanCode
	}

	#[inline]
	pub fn meta_state(&self) -> i32 {
		self.raw.metaState
	}

	#[inline]
	pub fn repeat_count(&self) -> i32 {
		self.raw.repeatCount
	}
}

impl std::fmt::Debug for GameActivityKeyEvent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("KeyEvent")
			.field("keyCode", &self.keyCode)
			.field("action", &self.action)
			.field("deviceId", &self.deviceId)
			.finish_non_exhaustive()
	}
}

/// An input event queued by the glue.
#[derive(Debug, Clone)]
pub enum InputEvent {
	Motion(MotionEvent),
	Key(KeyEvent),
}

/// Keys left to the system (volume, camera, zoom) so that the hardware buttons keep working.
pub(crate) fn is_system_key(key_code: i32) -> bool {
	const VOLUME_UP: i32 = 24;
	const VOLUME_DOWN: i32 = 25;
	const CAMERA: i32 = 27;
	const ZOOM_IN: i32 = 168;
	const ZOOM_OUT: i32 = 169;
	const VOLUME_MUTE: i32 = 164;
	matches!(key_code, VOLUME_UP | VOLUME_DOWN | VOLUME_MUTE | CAMERA | ZOOM_IN | ZOOM_OUT)
}

/// Pointer-like sources handled by evloop: touchscreen, mouse, stylus (`SOURCE_CLASS_POINTER`)
/// and touchpads (`SOURCE_CLASS_POSITION`). Others (joysticks, ...) are left to the system.
pub(crate) fn is_pointer_source(source: i32) -> bool {
	const SOURCE_CLASS_POINTER: i32 = 0x0000_0002;
	const SOURCE_CLASS_POSITION: i32 = 0x0000_0008;
	source & (SOURCE_CLASS_POINTER | SOURCE_CLASS_POSITION) != 0
}
