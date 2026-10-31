use core::fmt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(usize);

impl WindowId {
	pub const fn into_raw(self) -> usize {
		self.0
	}

	pub const fn from_raw(id: usize) -> Self {
		Self(id)
	}
}

impl fmt::Debug for WindowId {
	fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(fmtr)
	}
}
