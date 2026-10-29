pub(crate) mod action;
pub(crate) mod data;
pub(crate) mod hint;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DataTransferId(i64);

impl DataTransferId {
	pub const fn into_raw(&self) -> i64 {
		self.0
	}

	pub const fn from_raw(id: i64) -> Self {
		Self(id)
	}
}
