use std::{ptr, sync::Mutex};

use evloop_core::{
	dpi::{Position, Size},
	window::{WindowPositioner, place_window},
};
use geometry_types::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use windows_sys::Win32::{
	Foundation::{HWND, POINT},
	Graphics::Gdi::{ClientToScreen, InvalidateRgn},
	UI::WindowsAndMessaging::{
		GetParent, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
	},
};

use crate::{
	monitor::current_monitor,
	window::{
		area::{get_inner_rect, get_outer_rect},
		flags::WindowFlags,
		state::WindowState,
	},
};

pub(crate) fn outer_size_of(hwnd: HWND) -> PhysicalSize {
	get_outer_rect(hwnd)
		.map(|rect| {
			PhysicalSize::new((rect.right - rect.left) as u32, (rect.bottom - rect.top) as u32)
		})
		.unwrap()
}

fn surface_size_of(hwnd: HWND) -> PhysicalSize {
	get_inner_rect(hwnd)
		.map(|rect| {
			PhysicalSize::new((rect.right - rect.left) as u32, (rect.bottom - rect.top) as u32)
		})
		.unwrap()
}

// If the window is anchored the position is relative to the parent window and not relative to
// the screen. Therefore we have to translate it from the parent coordinate system to the display
// coordinate system. Free function so it can be used both from `Window` and from an owned
// popup's `hwnd`/`WindowState` alone (see `reposition_owned_popup`).
pub(crate) fn translate_outer_position(
	hwnd: HWND,
	window_state: &Mutex<WindowState>,
	position: Position,
	scale_factor: f64,
) -> PhysicalPosition {
	let position = position.to_physical(scale_factor);
	let mut point = POINT { x: position.x, y: position.y };

	let window_flags = window_state.lock().unwrap().window_flags;
	if window_flags.contains(WindowFlags::ANCHORED) && !window_flags.contains(WindowFlags::CHILD) {
		let parent = unsafe { GetParent(hwnd) };
		if !parent.is_null() {
			unsafe {
				ClientToScreen(parent, &mut point);
			}
		}
	}

	PhysicalPosition::new(point.x, point.y)
}

/// Computes an anchored window's new outer position and surface size from its positioner state,
/// using [`winit_common::positioner::place_window`]. Returns `None` if the window isn't anchored,
/// or its monitor's work area can't be determined.
///
/// On success, returns `(origin, size, current_size)`:
/// - `origin`: the window's new outer position, in logical coordinates relative to the parent's
///   content area, or to the screen if this window has no parent
/// - `size`: the window's new surface size, as constrained by [`place_window`] and converted back
///   from the outer size it operates on (see below)
/// - `current_size`: the window's surface size *before* this placement, i.e. `hwnd`'s current
///   surface size
pub(crate) fn compute_anchored_placement(
	hwnd: HWND,
	anchored: bool,
	positioner: &WindowPositioner,
	scale_factor: f64,
) -> Option<(LogicalPosition, LogicalSize, LogicalSize)> {
	if !anchored {
		return None;
	}

	let monitor = current_monitor(hwnd);
	// Clip to the monitor's work area rather than its full bounds, so anchored popups don't get
	// placed underneath the taskbar.
	let work_area = monitor.work_area()?;

	// The anchor rect's position is relative to the parent's content area (see
	// `Popup::set_anchor_rect`), and `set_outer_position` re-adds the parent's screen origin for
	// anchored windows (see `translate_outer_position`). To stay in the same coordinate space,
	// the clip region also needs to be expressed relative to the parent's content area. Without a
	// parent, both the anchor rect and `set_outer_position` operate directly in screen
	// coordinates instead, so the origin is left at zero and the clip region stays in screen
	// space.
	let parent = unsafe { GetParent(hwnd) };
	let mut parent_origin = POINT { x: 0, y: 0 };
	if !parent.is_null() {
		unsafe { ClientToScreen(parent, &mut parent_origin) };
	}

	let clip_position =
		PhysicalPosition::new(work_area.x - parent_origin.x, work_area.y - parent_origin.y)
			.to_logical(scale_factor);
	let clip_size = PhysicalSize::new(work_area.width, work_area.height).to_logical(scale_factor);

	let current_outer_size = outer_size_of(hwnd).to_logical(scale_factor);
	let current_size = surface_size_of(hwnd).to_logical(scale_factor);

	let (origin, outer_size) =
		place_window(positioner, scale_factor, current_outer_size, (clip_position, clip_size));

	// The window's non-client insets (title bar, borders, ...), assumed constant regardless of
	// the window's size, to translate `outer_size` back into a surface size.
	let insets_width = current_outer_size.width - current_size.width;
	let insets_height = current_outer_size.height - current_size.height;
	let size = LogicalSize::new(
		(outer_size.width - insets_width).max(0.0),
		(outer_size.height - insets_height).max(0.0),
	);

	Some((origin, size, current_size))
}

/// Repositions (and, if constrained, resizes) the popup with the given `hwnd`, using its
/// positioner state stored in `window_state`
///
/// This is used to reposition a window when its parent moves, since Win32 does not do so
/// automatically for owned windows, this is only ever called from the event loop
/// thread (from a sibling window's message procedure while handling the parent's
/// `WM_WINDOWPOSCHANGED`), so it's safe to update the OS window and `WindowState` directly
/// instead of going through `EventLoopThreadExecutor`
pub(crate) fn reposition_owned_popup(hwnd: HWND, window_state: &Mutex<WindowState>) {
	let (anchored, positioner, scale_factor) = {
		let state = window_state.lock().unwrap();
		(state.anchored, state.positioner, state.scale_factor)
	};

	let Some((origin, size, current_size)) =
		compute_anchored_placement(hwnd, anchored, &positioner, scale_factor)
	else {
		return;
	};

	let position =
		translate_outer_position(hwnd, window_state, Position::Logical(origin), scale_factor);
	WindowState::set_window_flags(window_state.lock().unwrap(), hwnd, |f| {
		f.set(WindowFlags::MAXIMIZED, false)
	});
	unsafe {
		SetWindowPos(
			hwnd,
			ptr::null_mut(),
			position.x,
			position.y,
			0,
			0,
			SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOSIZE | SWP_NOACTIVATE,
		);
		InvalidateRgn(hwnd, ptr::null_mut(), false.into());
	}

	if size != current_size {
		let physical_size = Size::Logical(size).to_physical(scale_factor);
		let window_flags = window_state.lock().unwrap().window_flags;
		window_flags.set_size(hwnd, physical_size);
	}
}
