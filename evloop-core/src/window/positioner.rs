use geometry_types::{LogicalPosition, LogicalSize};

use crate::dpi::{Position, Size};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPositioner {
	pub anchor: WindowAnchor,
	pub anchor_rect: (Position, Size),
	pub offset: Position,
	pub gravity: WindowGravity,
	pub constraint_adjustment: WindowConstraintAdjustment,
}

impl WindowPositioner {
	pub fn new(
		anchor: WindowAnchor,
		anchor_rect: (Position, Size),
		offset: Position,
		gravity: WindowGravity,
		constraint_adjustment: WindowConstraintAdjustment,
	) -> Self {
		WindowPositioner { anchor, anchor_rect, offset, gravity, constraint_adjustment }
	}
}

impl Default for WindowPositioner {
	fn default() -> Self {
		Self {
			anchor: WindowAnchor::default(),
			anchor_rect: (
				Position::Logical(LogicalPosition::new(0.0, 0.0)),
				Size::Logical(LogicalSize::new(1.0, 1.0)),
			),
			offset: Position::Logical(LogicalPosition::new(0.0, 0.0)),
			gravity: WindowGravity::default(),
			constraint_adjustment: WindowConstraintAdjustment::empty(),
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[non_exhaustive]
pub enum WindowAnchor {
	#[default]
	Center,
	Top,
	Bottom,
	Left,
	Right,
	TopLeft,
	BottomLeft,
	TopRight,
	BottomRight,
}

/// Defines in what direction a surface should be positioned
/// See: <https://wayland.app/protocols/xdg-shell#xdg_positioner:request:set_gravity>
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[non_exhaustive]
pub enum WindowGravity {
	#[default]
	Center,
	Top,
	Bottom,
	Left,
	Right,
	TopLeft,
	BottomLeft,
	TopRight,
	BottomRight,
}

bitflags::bitflags! {
	/// Specify how the window should be positioned if the originally intended position caused the
	/// surface to be constrained. See: <https://wayland.app/protocols/xdg-shell#xdg_positioner:request:set_constraint_adjustment>
	/// For all other platforms than wayland the behaviour is simulated on the winit side
	#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
	pub struct WindowConstraintAdjustment: u32 {
		const SLIDE_X = 1 << 0;
		const SLIDE_Y = 1 << 1;
		const FLIP_X = 1 << 2;
		const FLIP_Y = 1 << 3;
		const RESIZE_X = 1 << 4;
		const RESIZE_Y = 1 << 5;
	}
}

fn anchor_fraction(anchor: WindowAnchor) -> (f32, f32) {
	match anchor {
		WindowAnchor::Center => (0.5, 0.5),
		WindowAnchor::Top => (0.5, 0.0),
		WindowAnchor::Bottom => (0.5, 1.0),
		WindowAnchor::Left => (0.0, 0.5),
		WindowAnchor::Right => (1.0, 0.5),
		WindowAnchor::TopLeft => (0.0, 0.0),
		WindowAnchor::BottomLeft => (0.0, 1.0),
		WindowAnchor::TopRight => (1.0, 0.0),
		WindowAnchor::BottomRight => (1.0, 1.0),
		// `WindowAnchor` is `#[non_exhaustive]`; fall back to `Center` for variants added after
		// this crate was built against an older `winit-core`.
		_ => (0.5, 0.5),
	}
}

/// Returns, as fractions of the popup's own width/height, the offset from the anchor point to
/// the popup's origin (top-left corner). For example a gravity of `BottomRight` places the
/// popup's top-left corner at the anchor point, so the popup grows down and to the right.
fn gravity_fraction(gravity: WindowGravity) -> (f32, f32) {
	match gravity {
		WindowGravity::Center => (-0.5, -0.5),
		WindowGravity::Top => (-0.5, -1.0),
		WindowGravity::Bottom => (-0.5, 0.0),
		WindowGravity::Left => (-1.0, -0.5),
		WindowGravity::Right => (0.0, -0.5),
		WindowGravity::TopLeft => (-1.0, -1.0),
		WindowGravity::BottomLeft => (-1.0, 0.0),
		WindowGravity::TopRight => (0.0, -1.0),
		WindowGravity::BottomRight => (0.0, 0.0),
		// `WindowGravity` is `#[non_exhaustive]`; fall back to `Center` for variants added after
		// this crate was built against an older `winit-core`.
		_ => (-0.5, -0.5),
	}
}

/// Adjusts a single axis of the popup's placement to stay within `[clip_min, clip_max]`, applying
/// `flip`/`slide`/`resize` in the order the `xdg_positioner` protocol suggests: flip, then slide,
/// then resize. `flipped_origin` is the alternate origin obtained by mirroring both the anchor
/// edge and the gravity on this axis; it is only used when `flip` is set and it actually results
/// in a better fit than the original origin.
fn constrain_axis(
	origin: f32,
	extent: f32,
	(clip_min, clip_max): (f32, f32),
	flipped_origin: f32,
	(flip, slide, resize): (bool, bool, bool),
) -> (f32, f32) {
	let fits = |o: f32| o >= clip_min && o + extent <= clip_max;

	let mut origin = origin;
	if !fits(origin) && flip && fits(flipped_origin) {
		origin = flipped_origin;
	}

	if !fits(origin) && slide {
		// `extent` may exceed the clip size, in which case `clip_max - extent` would be less
		// than `clip_min`; `.max(clip_min)` keeps the clamp range valid in that case.
		let slide_max = (clip_max - extent).max(clip_min);
		origin = origin.clamp(clip_min, slide_max);
	}

	let mut extent = extent;
	if resize && !fits(origin) {
		let clamped_origin = origin.max(clip_min);
		// Intersect with the clip rectangle on both ends, not just `clip_max`: if only the
		// leading edge overflows (`origin < clip_min`) while the trailing edge already fits,
		// this must shrink down to the original trailing edge rather than growing all the way
		// out to `clip_max`.
		extent = (clip_max.min(origin + extent) - clamped_origin).max(0.0);
		origin = clamped_origin;
	}

	(origin, extent)
}

pub fn place_window(
	positioner: &WindowPositioner,
	scale_factor: f64,
	window_size: LogicalSize,
	(clip_position, clip_size): (LogicalPosition, LogicalSize),
) -> (LogicalPosition, LogicalSize) {
	let anchor = positioner.anchor;
	let gravity = positioner.gravity;
	let constraint_adjustment = positioner.constraint_adjustment;
	let (anchor_position, anchor_size) = positioner.anchor_rect;
	let anchor_position = anchor_position.to_logical(scale_factor);
	let anchor_size = anchor_size.to_logical(scale_factor);
	let offset = positioner.offset.to_logical(scale_factor);

	let (anchor_fx, anchor_fy) = anchor_fraction(anchor);
	let (gravity_fx, gravity_fy) = gravity_fraction(gravity);

	let anchor_point_x = anchor_position.x + anchor_size.width * anchor_fx;
	let anchor_point_y = anchor_position.y + anchor_size.height * anchor_fy;

	let origin_x = anchor_point_x + window_size.width * gravity_fx + offset.x;
	let origin_y = anchor_point_y + window_size.height * gravity_fy + offset.y;

	// Flipping mirrors both the anchor edge and the gravity on that axis, effectively placing the
	// popup on the opposite side of the anchor rectangle.
	let flipped_anchor_point_x = anchor_position.x + anchor_size.width * (1.0 - anchor_fx);
	let flipped_anchor_point_y = anchor_position.y + anchor_size.height * (1.0 - anchor_fy);
	let flipped_x = flipped_anchor_point_x + window_size.width * (-1.0 - gravity_fx) + offset.x;
	let flipped_y = flipped_anchor_point_y + window_size.height * (-1.0 - gravity_fy) + offset.y;

	let clip_min_x = clip_position.x;
	let clip_max_x = clip_position.x + clip_size.width;
	let clip_min_y = clip_position.y;
	let clip_max_y = clip_position.y + clip_size.height;

	let (x, width) = constrain_axis(
		origin_x,
		window_size.width,
		(clip_min_x, clip_max_x),
		flipped_x,
		(
			constraint_adjustment.contains(WindowConstraintAdjustment::FLIP_X),
			constraint_adjustment.contains(WindowConstraintAdjustment::SLIDE_X),
			constraint_adjustment.contains(WindowConstraintAdjustment::RESIZE_X),
		),
	);
	let (y, height) = constrain_axis(
		origin_y,
		window_size.height,
		(clip_min_y, clip_max_y),
		flipped_y,
		(
			constraint_adjustment.contains(WindowConstraintAdjustment::FLIP_Y),
			constraint_adjustment.contains(WindowConstraintAdjustment::SLIDE_Y),
			constraint_adjustment.contains(WindowConstraintAdjustment::RESIZE_Y),
		),
	);

	(LogicalPosition::new(x, y), LogicalSize::new(width, height))
}
