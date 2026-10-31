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
