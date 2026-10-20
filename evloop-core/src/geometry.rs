#[derive(Debug, Copy, Clone)]
pub struct PhysicalInsets {
	pub bottom: u32,
	pub left: u32,
	pub top: u32,
	pub right: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct LogicalInsets {
	pub bottom: f64,
	pub left: f64,
	pub top: f64,
	pub right: f64,
}

#[derive(Debug, Copy, Clone)]
pub struct PhysicalSize {
	pub width: u32,
	pub height: u32,
}

impl PhysicalSize {
	#[inline]
	#[must_use]
	pub fn new(width: u32, height: u32) -> PhysicalSize {
		PhysicalSize { width, height }
	}
}

#[derive(Debug, Copy, Clone)]
pub struct LogicalSize {
	pub width: f64,
	pub height: f64,
}

#[derive(Debug, Copy, Clone)]
pub struct PhysicalPosition {
	pub x: i32,
	pub y: i32,
}
