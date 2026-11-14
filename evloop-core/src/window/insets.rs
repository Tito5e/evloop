use geometry_types::{PhysicalBounds, PhysicalSize};

/// Distances, in physical pixels, by which something intrudes into a surface from each edge.
///
/// Used by `Window::safe_area` and `Window::ime_occlusion` on every backend.
///
/// # Semantics
///
/// - **Safe area** (`Window::safe_area`): the insets that must be removed from the surface to get
///   the region where interactive content can be placed without being covered or clipped by the
///   system (status bar, navigation bar, display cutout, ...). A backend that has no such system
///   UI returns [`PhysicalInsets::ZERO`]; this is the correct value, not a placeholder.
/// - **IME occlusion** (`Window::ime_occlusion`): the insets covered by an on-screen (soft)
///   keyboard. [`PhysicalInsets::ZERO`] when no soft keyboard covers the surface.
///
/// Insets are independent of the surface size: a resize does not change their meaning. Use
/// [`PhysicalInsets::inset_bounds`] to turn them into a rectangle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalInsets {
	pub top: u32,
	pub left: u32,
	pub bottom: u32,
	pub right: u32,
}

impl PhysicalInsets {
	pub const ZERO: Self = Self { top: 0, left: 0, bottom: 0, right: 0 };

	#[inline]
	#[must_use]
	pub const fn new(top: u32, left: u32, bottom: u32, right: u32) -> Self {
		Self { top, left, bottom, right }
	}

	#[inline]
	#[must_use]
	pub const fn is_zero(&self) -> bool {
		self.top == 0 && self.left == 0 && self.bottom == 0 && self.right == 0
	}

	/// The edge-wise maximum of both insets.
	#[inline]
	#[must_use]
	pub fn union(self, other: Self) -> Self {
		Self {
			top: self.top.max(other.top),
			left: self.left.max(other.left),
			bottom: self.bottom.max(other.bottom),
			right: self.right.max(other.right),
		}
	}

	/// The rectangle, in surface coordinates, that remains after removing these insets from a
	/// surface of `size`. Edges are clamped so the result is never inverted.
	#[must_use]
	pub fn inset_bounds(&self, size: PhysicalSize) -> PhysicalBounds {
		let width = size.width.min(i32::MAX as u32) as i32;
		let height = size.height.min(i32::MAX as u32) as i32;
		let clamp = |v: u32, max: i32| (v.min(i32::MAX as u32) as i32).min(max);

		let left = clamp(self.left, width);
		let top = clamp(self.top, height);
		let right = (width - clamp(self.right, width)).max(left);
		let bottom = (height - clamp(self.bottom, height)).max(top);

		PhysicalBounds::lrtb(left, right, top, bottom)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn inset_bounds_basic() {
		let insets = PhysicalInsets::new(10, 5, 20, 15);
		let b = insets.inset_bounds(PhysicalSize::new(100, 200));
		assert_eq!(b, PhysicalBounds::lrtb(5, 85, 10, 180));
	}

	#[test]
	fn inset_bounds_never_inverts() {
		let insets = PhysicalInsets::new(150, 80, 150, 80);
		let b = insets.inset_bounds(PhysicalSize::new(100, 200));
		assert!(b.left <= b.right && b.top <= b.bottom);
	}

	#[test]
	fn zero_is_whole_surface() {
		let b = PhysicalInsets::ZERO.inset_bounds(PhysicalSize::new(640, 480));
		assert_eq!(b, PhysicalBounds::lrtb(0, 640, 0, 480));
	}
}
