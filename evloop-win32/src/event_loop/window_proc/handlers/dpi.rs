use std::{
	ptr,
	sync::{Arc, Mutex},
};

use evloop_core::window::WindowId;
use geometry_types::PhysicalSize;
use windows_sys::Win32::{
	Foundation::{RECT, SIZE},
	UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos},
};

use crate::{
	event::{SurfaceSizeWriter, WindowEvent},
	event_loop::{Event, window_proc::message::WindowMessage},
	sys::{dpi::dpi_to_scale_factor, msg_params::loword, proc_result::ProcResult},
	window::{WindowFlags, WindowState, get_inner_rect},
};

/// Handles `WM_GETDPISCALEDSIZE` (Per-Monitor V2 only), sent before `WM_DPICHANGED`.
///
/// Emits `WindowEvent::ScaleFactorChanging` synchronously so the application can choose the
/// surface size it gets at the new DPI. If the application keeps the proposed (linearly
/// scaled) size, or cannot be called right now, `FALSE` is returned and the OS scales the window
/// itself.
pub(crate) fn on_wm_getdpiscaledsize(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;
	*result = ProcResult::Value(false.into());

	let new_dpi = wparam as u32;
	let new_scale_factor = dpi_to_scale_factor(new_dpi);

	let (old_scale_factor, surface_size, window_flags) = {
		let state = userdata.window_state_lock();
		let resizable_by_dpi =
			state.fullscreen.is_none() && !state.window_flags().contains(WindowFlags::MAXIMIZED);
		if !resizable_by_dpi || state.scale_factor == new_scale_factor {
			return;
		}
		(state.scale_factor, state.surface_size, state.window_flags())
	};

	let proposed = surface_size.to_logical(old_scale_factor).to_physical(new_scale_factor);
	let new_size = Arc::new(Mutex::new(proposed));

	let event = Event::Window {
		window_id: WindowId::from_raw(window as usize),
		event: WindowEvent::ScaleFactorChanging {
			scale_factor: new_scale_factor,
			surface_size_writer: SurfaceSizeWriter::new(Arc::downgrade(&new_size)),
		},
	};
	if !userdata.event_loop_runner.dispatch_synchronously(event) {
		return;
	}

	let requested = *new_size.lock().unwrap();
	if requested == proposed {
		return;
	}

	let client =
		RECT { left: 0, top: 0, right: requested.width as i32, bottom: requested.height as i32 };
	let outer = window_flags.adjust_rect_for_dpi(window, client, Some(new_dpi)).unwrap_or(client);

	let size = unsafe { &mut *(lparam as *mut SIZE) };
	size.cx = outer.right - outer.left;
	size.cy = outer.bottom - outer.top;
	*result = ProcResult::Value(true.into());
}

/// Handles `WM_DPICHANGED`: emits `ScaleFactorChanged` and applies the rectangle suggested by
/// the OS (which already honors the size chosen in `ScaleFactorChanging`). The resulting
/// `WM_SIZE` emits `Resized`.
pub(crate) fn on_wm_dpichanged(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;

	let new_dpi_x = loword(wparam as u32) as u32;
	let new_scale_factor = dpi_to_scale_factor(new_dpi_x);
	let old_scale_factor: f64;

	let allow_resize = {
		let mut window_state = userdata.window_state_lock();
		old_scale_factor = window_state.scale_factor;
		window_state.scale_factor = new_scale_factor;

		if new_scale_factor == old_scale_factor {
			*result = ProcResult::Value(0);
			return;
		}

		let allow_resize = window_state.fullscreen.is_none()
			&& !window_state.window_flags().contains(WindowFlags::MAXIMIZED);

		allow_resize
	};

	let suggested_rect = unsafe { *(lparam as *const RECT) };

	let old_physical_inner_rect =
		get_inner_rect(window).expect("Failed to query (old) inner window rect");
	let old_physical_surface_size = PhysicalSize::new(
		(old_physical_inner_rect.right - old_physical_inner_rect.left) as u32,
		(old_physical_inner_rect.bottom - old_physical_inner_rect.top) as u32,
	);

	let new_physical_surface_size = match allow_resize {
		true => {
			old_physical_surface_size.to_logical(old_scale_factor).to_physical(new_scale_factor)
		}
		false => old_physical_surface_size,
	};

	userdata.send_window_event(window, WindowEvent::ScaleFactorChanged(new_scale_factor));

	{
		let window_state = userdata.window_state_lock();
		if new_physical_surface_size != old_physical_surface_size {
			WindowState::set_window_flags(window_state, window, |f| {
				f.set(WindowFlags::MAXIMIZED, false)
			});
		}
	}

	let new_outer_rect: RECT = suggested_rect;

	unsafe {
		SetWindowPos(
			window,
			ptr::null_mut(),
			new_outer_rect.left,
			new_outer_rect.top,
			new_outer_rect.right - new_outer_rect.left,
			new_outer_rect.bottom - new_outer_rect.top,
			SWP_NOZORDER | SWP_NOACTIVATE,
		)
	};

	*result = ProcResult::Value(0);
}
