use core::fmt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(usize);

impl WindowId {
	/// Convert the `WindowId` into the underlying integer.
	///
	/// This is useful if you need to pass the ID across an FFI boundary, or store it in an atomic.
	pub const fn into_raw(self) -> usize {
		self.0
	}

	/// Construct a `WindowId` from the underlying integer.
	///
	/// This should only be called with integers returned from [`WindowId::into_raw`].
	pub const fn from_raw(id: usize) -> Self {
		Self(id)
	}
}

impl fmt::Debug for WindowId {
	fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(fmtr)
	}
}
