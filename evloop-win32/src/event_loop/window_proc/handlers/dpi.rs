use std::{
	ptr,
	sync::{Arc, Mutex},
};

use geometry_types::PhysicalSize;
use windows_sys::Win32::{
	Foundation::RECT,
	UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos},
};

use crate::{
	event::WindowEvent,
	event_loop::window_proc::message::WindowMessage,
	sys::{dpi::dpi_to_scale_factor, msg_params::loword, proc_result::ProcResult},
	window::{WindowFlags, WindowState, get_inner_rect},
};

// TODO: WM_GETDPISCALEDSIZEを使用した実装を追加する
// winit-win32(UPSTREAM)で使用されているワークアラウンドが必要なくなる可能性がある
// また、フルスクリーンに関するハンドリングも詳細に行う
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
