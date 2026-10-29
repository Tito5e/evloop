use std::time::Instant;

use geometry_types::{PhysicalBounds, PhysicalPosition, PhysicalSize};

use crate::{
	keyboard::KeyEvent,
	pointer::{ButtonSource, PointerKind, PointerSource},
};

// TODO: move to each backend crate
#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
	ContentRectChanged(PhysicalBounds),
	Focused(bool),
	ScaleFactorChanged(f64),
	Occluded(bool),
	Resized(PhysicalSize),
	Moved(PhysicalPosition),
	CloseRequested,
	PointerEntered {
		device_id: Option<DeviceId>,
		position: PhysicalPosition,
		primary: bool,
		kind: PointerKind,
	},
	PointerButton {
		device_id: Option<DeviceId>,
		state: ElementState,
		position: PhysicalPosition,
		primary: bool,
		button: ButtonSource,
		is_macos_activation_click: bool,
	},
	PointerMoved {
		device_id: Option<DeviceId>,
		position: PhysicalPosition,
		primary: bool,
		source: PointerSource,
	},
	PointerLeft {
		device_id: Option<DeviceId>,
		position: Option<PhysicalPosition>,
		primary: bool,
		kind: PointerKind,
	},
	KeyboardInput {
		device_id: Option<DeviceId>,
		event: KeyEvent,
		is_synthetic: bool,
	},
	RedrawRequested,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(i64);

impl DeviceId {
	pub const fn into_raw(self) -> i64 {
		self.0
	}

	pub const fn from_raw(id: i64) -> Self {
		Self(id)
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum ElementState {
	Pressed,
	Released,
}

impl ElementState {
	#[inline]
	#[must_use]
	pub const fn is_pressed(&self) -> bool {
		match *self {
			Self::Pressed => true,
			Self::Released => false,
		}
	}
}

pub enum EvloopEvent {
	ProxyWakeUp,
	MemoryWarning,
}
