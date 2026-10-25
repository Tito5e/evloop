use geometry_types::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
	Logical(LogicalPosition),
	Physical(PhysicalPosition),
}

impl Position {
	#[inline]
	#[must_use]
	pub fn to_physical(&self, scale_factor: f64) -> PhysicalPosition {
		match self {
			Self::Logical(logical) => logical.to_physical(scale_factor),
			Self::Physical(physical) => *physical,
		}
	}

	#[inline]
	#[must_use]
	pub fn to_logical(&self, scale_factor: f64) -> LogicalPosition {
		match self {
			Self::Logical(logical) => *logical,
			Self::Physical(physical) => physical.to_logical(scale_factor),
		}
	}
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
	Logical(LogicalSize),
	Physical(PhysicalSize),
}

impl Size {
	#[inline]
	#[must_use]
	pub fn to_physical(&self, scale_factor: f64) -> PhysicalSize {
		match self {
			Self::Logical(logical) => logical.to_physical(scale_factor),
			Self::Physical(physical) => *physical,
		}
	}

	#[inline]
	#[must_use]
	pub fn to_logical(&self, scale_factor: f64) -> LogicalSize {
		match self {
			Self::Logical(logical) => *logical,
			Self::Physical(physical) => physical.to_logical(scale_factor),
		}
	}
}
