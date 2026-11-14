use std::{ffi::c_void, mem, ptr, sync::MutexGuard};

use evloop_core::{
	event::ElementState::{Pressed, Released},
	pointer::{
		ButtonSource, MouseButton,
		MouseScrollDelta::{LineDelta, PixelDelta},
		PointerKind::{self, Mouse},
		PointerSource, ScrollSource, TouchPhase,
	},
};
use geometry_types::PhysicalPosition;
use windows_sys::Win32::{
	Foundation::{HWND, RECT},
	UI::{
		Controls::HOVER_DEFAULT,
		Input::KeyboardAndMouse::{
			ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
		},
		WindowsAndMessaging::{
			GetClientRect, HTCLIENT, LoadCursorW, SPI_GETWHEELSCROLLCHARS, SPI_GETWHEELSCROLLLINES,
			SetCursor, SystemParametersInfoW, WHEEL_DELTA, WM_LBUTTONDOWN, WM_LBUTTONUP,
			WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
		},
	},
};

use crate::{
	event::WindowEvent,
	event_loop::window_proc::{keyboard::update_modifiers, message::WindowMessage},
	resource::cursor::{SelectedCursor, to_windows_cursor},
	sys::{
		msg_params::{get_x_lparam, get_xbutton_wparam, get_y_lparam, loword},
		proc_result::ProcResult,
	},
	window::{CursorFlags, WindowState},
};

pub(crate) fn on_wm_mousemove(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, lparam, userdata, .. } = *m;

	let x = get_x_lparam(lparam as u32) as i32;
	let y = get_y_lparam(lparam as u32) as i32;
	let position = PhysicalPosition::new(x, y);

	let cursor_moved;
	{
		let mut w = userdata.window_state_lock();
		let mouse_was_inside_window = w.mouse.cursor_flags().contains(CursorFlags::IN_WINDOW);

		match get_pointer_move_kind(window, mouse_was_inside_window, x, y) {
			PointerMoveKind::Enter => {
				w.mouse.set_cursor_flags(window, |f| f.set(CursorFlags::IN_WINDOW, true)).ok();

				drop(w);
				userdata.send_window_event(
					window,
					WindowEvent::PointerEntered {
						device_id: None,
						primary: true,
						position,
						kind: PointerKind::Mouse,
					},
				);

				// Calling TrackMouseEvent in order to receive mouse leave events.
				unsafe {
					TrackMouseEvent(&mut TRACKMOUSEEVENT {
						cbSize: mem::size_of::<TRACKMOUSEEVENT>() as u32,
						dwFlags: TME_LEAVE,
						hwndTrack: window,
						dwHoverTime: HOVER_DEFAULT,
					})
				};
			}
			PointerMoveKind::Leave => {
				w.mouse.set_cursor_flags(window, |f| f.set(CursorFlags::IN_WINDOW, false)).ok();

				drop(w);
				userdata.send_window_event(
					window,
					WindowEvent::PointerLeft {
						device_id: None,
						primary: true,
						position: Some(position),
						kind: PointerKind::Mouse,
					},
				);
			}
			PointerMoveKind::None => drop(w),
		}

		// handle spurious WM_MOUSEMOVE messages
		// see https://devblogs.microsoft.com/oldnewthing/20031001-00/?p=42343
		// and http://debugandconquer.blogspot.com/2015/08/the-cause-of-spurious-mouse-move.html
		let mut w = userdata.window_state_lock();
		cursor_moved = w.mouse.last_position != Some(position);
		w.mouse.last_position = Some(position);
	}

	if cursor_moved {
		update_modifiers(window, userdata);

		userdata.send_window_event(
			window,
			WindowEvent::PointerMoved {
				device_id: None,
				primary: true,
				position,
				source: PointerSource::Mouse,
			},
		);
	}

	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_mouseleave(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, userdata, .. } = *m;

	{
		let mut w = userdata.window_state_lock();
		w.mouse.set_cursor_flags(window, |f| f.set(CursorFlags::IN_WINDOW, false)).ok();
	}

	userdata.send_window_event(
		window,
		WindowEvent::PointerLeft { device_id: None, primary: true, position: None, kind: Mouse },
	);

	*result = ProcResult::Value(0);
}

/// Handles `WM_MOUSEWHEEL`.
pub(crate) fn on_wm_mousewheel(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;

	let value = (wparam >> 16) as i16;
	let value = value as f32 / WHEEL_DELTA as f32;

	update_modifiers(window, userdata);

	let delta = if userdata.window_state_lock().use_system_wheel_speed {
		let mut scroll_lines: u32 = DEFAULT_SCROLL_LINES_PER_WHEEL_DELTA as u32;
		let _ = unsafe {
			SystemParametersInfoW(
				SPI_GETWHEELSCROLLLINES,
				0,
				&mut scroll_lines as *mut u32 as *mut c_void,
				0,
			)
		};
		if scroll_lines == WHEEL_PAGESCROLL {
			// "One screen at a time": one notch scrolls by the height of the surface.
			let page_height = userdata.window_state_lock().surface_size.height as f32;
			PixelDelta(PhysicalPosition::new(0, (value * page_height).round() as i32))
		} else {
			LineDelta(0.0, value * scroll_lines as f32)
		}
	} else {
		LineDelta(0.0, value)
	};

	userdata.send_window_event(
		window,
		WindowEvent::MouseWheel {
			device_id: None,
			delta,
			phase: TouchPhase::Moved,
			source: ScrollSource::Unknown,
		},
	);

	*result = ProcResult::Value(0);
}

/// Handles `WM_MOUSEHWHEEL`.
pub(crate) fn on_wm_mousehwheel(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;

	let value = (wparam >> 16) as i16;
	let value = inverted_wheel_delta(value); // NOTE: inverted! See https://github.com/rust-windowing/winit/pull/2105/

	update_modifiers(window, userdata);

	let scroll_characters_multiplier = if userdata.window_state_lock().use_system_wheel_speed {
		let mut scroll_characters = DEFAULT_SCROLL_CHARACTERS_PER_WHEEL_DELTA;
		let _ = unsafe {
			SystemParametersInfoW(
				SPI_GETWHEELSCROLLCHARS,
				0,
				&mut scroll_characters as *mut isize as *mut c_void,
				0,
			)
		};
		scroll_characters
	} else {
		1
	};

	userdata.send_window_event(
		window,
		WindowEvent::MouseWheel {
			device_id: None,
			delta: LineDelta(value * scroll_characters_multiplier as f32, 0.0),
			phase: TouchPhase::Moved,
			source: ScrollSource::Unknown,
		},
	);

	*result = ProcResult::Value(0);
}

/// Handles `WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN`.
pub(crate) fn on_mouse_button_down(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, msg, lparam, userdata, .. } = *m;
	unsafe { capture_mouse(window, &mut userdata.window_state_lock()) };

	update_modifiers(window, userdata);

	let x = get_x_lparam(lparam as u32) as i32;
	let y = get_y_lparam(lparam as u32) as i32;
	let position = PhysicalPosition::new(x, y);

	userdata.send_window_event(
		window,
		WindowEvent::PointerButton {
			device_id: None,
			primary: true,
			state: Pressed,
			position,
			button: ButtonSource::Mouse(match msg {
				WM_LBUTTONDOWN => MouseButton::Left,
				WM_RBUTTONDOWN => MouseButton::Right,
				WM_MBUTTONDOWN => MouseButton::Middle,
				_ => unreachable!(),
			}),
		},
	);
	*result = ProcResult::Value(0);
}

/// Handles `WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP`.
pub(crate) fn on_mouse_button_up(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, msg, lparam, userdata, .. } = *m;
	unsafe { release_mouse(userdata.window_state_lock()) };

	update_modifiers(window, userdata);

	let x = get_x_lparam(lparam as u32) as i32;
	let y = get_y_lparam(lparam as u32) as i32;
	let position = PhysicalPosition::new(x, y);

	userdata.send_window_event(
		window,
		WindowEvent::PointerButton {
			device_id: None,
			primary: true,
			state: Released,
			position,
			button: ButtonSource::Mouse(match msg {
				WM_LBUTTONUP => MouseButton::Left,
				WM_RBUTTONUP => MouseButton::Right,
				WM_MBUTTONUP => MouseButton::Middle,
				_ => unreachable!(),
			}),
		},
	);
	*result = ProcResult::Value(0);
}

/// Handles `WM_XBUTTONDOWN`.
pub(crate) fn on_wm_xbuttondown(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;
	let xbutton = get_xbutton_wparam(wparam as u32);

	unsafe { capture_mouse(window, &mut userdata.window_state_lock()) };

	update_modifiers(window, userdata);

	let x = get_x_lparam(lparam as u32) as i32;
	let y = get_y_lparam(lparam as u32) as i32;
	let position = PhysicalPosition::new(x, y);

	// 1 is defined as back, 2 as forward; other codes are unexpected.
	let b = xbutton as u8 + MouseButton::Back as u8 - 1;

	userdata.send_window_event(
		window,
		WindowEvent::PointerButton {
			device_id: None,
			primary: true,
			state: Pressed,
			position,

			button: ButtonSource::Mouse(MouseButton::try_from_u8(b).unwrap().into()),
		},
	);
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_xbuttonup(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;

	let xbutton = get_xbutton_wparam(wparam as u32);

	unsafe { release_mouse(userdata.window_state_lock()) };

	update_modifiers(window, userdata);

	let x = get_x_lparam(lparam as u32) as i32;
	let y = get_y_lparam(lparam as u32) as i32;
	let position = PhysicalPosition::new(x, y);

	let b = xbutton as u8 + MouseButton::Back as u8 - 1;

	userdata.send_window_event(
		window,
		WindowEvent::PointerButton {
			device_id: None,
			primary: true,
			state: Released,
			position,
			// 1 is defined as back, 2 as forward; other codes are unexpected.
			button: ButtonSource::Mouse(MouseButton::try_from_u8(b).unwrap()),
		},
	);
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_capturechanged(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, lparam, userdata, .. } = *m;

	if lparam != window as isize {
		userdata.window_state_lock().mouse.capture_count = 0;
	}
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_setcursor(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { wparam, lparam, userdata, .. } = *m;
	let set_cursor_to = {
		let window_state = userdata.window_state_lock();
		let in_client_area = loword(lparam as u32) as u32 == HTCLIENT;
		if in_client_area { Some(window_state.mouse.selected_cursor.clone()) } else { None }
	};

	match set_cursor_to {
		Some(selected_cursor) => {
			let hcursor = match selected_cursor {
				SelectedCursor::Named(cursor_icon) => unsafe {
					LoadCursorW(ptr::null_mut(), to_windows_cursor(cursor_icon))
				},
				SelectedCursor::Custom(cursor) => cursor.as_raw_handle(),
			};
			unsafe { SetCursor(hcursor) };
			*result = ProcResult::Value(0);
		}
		None => *result = ProcResult::DefWindowProc(wparam),
	}
}

const WHEEL_PAGESCROLL: u32 = u32::MAX;

// https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfoa#:~:text=SPI_GETWHEELSCROLLLINES
const DEFAULT_SCROLL_LINES_PER_WHEEL_DELTA: isize = 3;

// https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfoa#:~:text=SPI_GETWHEELSCROLLCHARS
const DEFAULT_SCROLL_CHARACTERS_PER_WHEEL_DELTA: isize = 3;

unsafe fn capture_mouse(window: HWND, window_state: &mut WindowState) {
	window_state.mouse.capture_count += 1;
	unsafe { SetCapture(window) };
}

unsafe fn release_mouse(mut window_state: MutexGuard<'_, WindowState>) {
	window_state.mouse.capture_count = window_state.mouse.capture_count.saturating_sub(1);
	if window_state.mouse.capture_count == 0 {
		drop(window_state);
		unsafe { ReleaseCapture() };
	}
}

enum PointerMoveKind {
	/// Pointer entered the window.
	Enter,
	/// Pointer leaved the window client area.
	Leave,
	/// Pointer is inside the window or `GetClientRect` failed.
	None,
}

fn get_pointer_move_kind(
	window: HWND,
	mouse_was_inside_window: bool,
	x: i32,
	y: i32,
) -> PointerMoveKind {
	let rect: RECT = unsafe {
		let mut rect: RECT = mem::zeroed();
		if GetClientRect(window, &mut rect) == false.into() {
			return PointerMoveKind::None;
		}
		rect
	};

	let x = (rect.left..rect.right).contains(&x);
	let y = (rect.top..rect.bottom).contains(&y);

	if !mouse_was_inside_window && x && y {
		PointerMoveKind::Enter
	} else if mouse_was_inside_window && !(x && y) {
		PointerMoveKind::Leave
	} else {
		PointerMoveKind::None
	}
}

pub(crate) fn inverted_wheel_delta(value: i16) -> f32 {
	-(value as f32) / WHEEL_DELTA as f32
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn inverted_wheel_delta_handles_minimum_value() {
		assert_eq!(inverted_wheel_delta(i16::MIN), 32768.0 / WHEEL_DELTA as f32);
		assert_eq!(inverted_wheel_delta(120), -1.0);
		assert_eq!(inverted_wheel_delta(-120), 1.0);
	}
}
