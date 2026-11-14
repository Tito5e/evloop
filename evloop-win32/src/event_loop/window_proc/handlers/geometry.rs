use geometry_types::{PhysicalPosition, PhysicalSize};
use windows_sys::{
	Win32::{
		Foundation::{HWND, LPARAM, POINT, RECT},
		Graphics::Gdi::{MONITOR_DEFAULTTONULL, MonitorFromRect},
		System::Threading::GetCurrentThreadId,
		UI::WindowsAndMessaging::{
			EnumThreadWindows, GW_OWNER, GWL_USERDATA, GetWindow, HTCAPTION, MINMAXINFO,
			NCCALCSIZE_PARAMS, PostMessageW, SIZE_MAXIMIZED, SIZE_MINIMIZED, SWP_NOMOVE,
			SWP_NOSIZE, WINDOWPOS, WM_LBUTTONUP, WM_MOUSEMOVE, WMSZ_BOTTOM, WMSZ_BOTTOMLEFT,
			WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
		},
	},
	core::BOOL,
};

use crate::{
	event::WindowEvent,
	event_loop::{
		public_window_callback,
		window_proc::{message::WindowMessage, user_data::WindowUserData},
	},
	monitor::{Fullscreen, MonitorHandle, get_monitor_info},
	sys::{
		msg_params::{hiword, loword},
		proc_result::ProcResult,
		window_long::get_window_long,
		window_query::is_maximized,
	},
	window::{WindowFlags, get_outer_rect, reposition_owned_popup},
};

pub(crate) fn on_wm_nccalcsize(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;
	let window_flags = userdata.window_state_lock().window_flags;
	if wparam == 0 || window_flags.contains(WindowFlags::MARKER_DECORATIONS) {
		*result = ProcResult::DefWindowProc(wparam);
		return;
	}

	let params = unsafe { &mut *(lparam as *mut NCCALCSIZE_PARAMS) };

	if is_maximized(window) {
		let monitor = unsafe { MonitorFromRect(&params.rgrc[0], MONITOR_DEFAULTTONULL) };
		if let Ok(monitor_info) = get_monitor_info(monitor) {
			params.rgrc[0] = monitor_info.monitorInfo.rcWork;
		}
	} else if window_flags.contains(WindowFlags::MARKER_UNDECORATED_SHADOW) {
		params.rgrc[0].top += 1;
		params.rgrc[0].bottom += 1;
	}

	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_entersizemove(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { userdata, .. } = *m;
	userdata
		.window_state_lock()
		.set_window_flags_in_place(|f| f.insert(WindowFlags::MARKER_IN_SIZE_MOVE));
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_exitsizemove(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, lparam, userdata, .. } = *m;
	let mut state = userdata.window_state_lock();
	if state.dragging {
		state.dragging = false;
		unsafe { PostMessageW(window, WM_LBUTTONUP, 0, lparam) };
	}

	state.set_window_flags_in_place(|f| f.remove(WindowFlags::MARKER_IN_SIZE_MOVE));
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_nclbuttondown(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, .. } = *m;
	if wparam == HTCAPTION as _ {
		let lparam = 0;
		unsafe { PostMessageW(window, WM_MOUSEMOVE, 0, lparam) };
	}
	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_wm_windowposchanging(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, lparam, userdata, .. } = *m;
	let mut window_state = userdata.window_state_lock();
	if let Some(ref mut fullscreen) = window_state.fullscreen {
		let window_pos = unsafe { &mut *(lparam as *mut WINDOWPOS) };
		let new_rect = RECT {
			left: window_pos.x,
			top: window_pos.y,
			right: window_pos.x + window_pos.cx,
			bottom: window_pos.y + window_pos.cy,
		};

		const NOMOVE_OR_NOSIZE: u32 = SWP_NOMOVE | SWP_NOSIZE;

		let new_rect = if window_pos.flags & NOMOVE_OR_NOSIZE != 0 {
			let cur_rect = get_outer_rect(window).expect("Unexpected GetWindowRect failure");

			match window_pos.flags & NOMOVE_OR_NOSIZE {
				NOMOVE_OR_NOSIZE => None,

				SWP_NOMOVE => Some(RECT {
					left: cur_rect.left,
					top: cur_rect.top,
					right: cur_rect.left + window_pos.cx,
					bottom: cur_rect.top + window_pos.cy,
				}),

				SWP_NOSIZE => Some(RECT {
					left: window_pos.x,
					top: window_pos.y,
					right: window_pos.x - cur_rect.left + cur_rect.right,
					bottom: window_pos.y - cur_rect.top + cur_rect.bottom,
				}),

				_ => unreachable!(),
			}
		} else {
			Some(new_rect)
		};

		if let Some(new_rect) = new_rect {
			let new_monitor = unsafe { MonitorFromRect(&new_rect, MONITOR_DEFAULTTONULL) };
			match fullscreen {
				Fullscreen::Borderless(fullscreen_monitor) => {
					if !new_monitor.is_null()
						&& fullscreen_monitor
							.as_ref()
							.map(|monitor| new_monitor != monitor.native_id() as _)
							.unwrap_or(true)
					{
						if let Ok(new_monitor_info) = get_monitor_info(new_monitor) {
							let new_monitor_rect = new_monitor_info.monitorInfo.rcMonitor;
							window_pos.x = new_monitor_rect.left;
							window_pos.y = new_monitor_rect.top;
							window_pos.cx = new_monitor_rect.right - new_monitor_rect.left;
							window_pos.cy = new_monitor_rect.bottom - new_monitor_rect.top;
						}
						*fullscreen_monitor = Some(MonitorHandle::new(new_monitor));
					}
				}
				Fullscreen::Exclusive(monitor, _) => {
					if let Ok(old_monitor_info) = get_monitor_info(monitor.native_id() as _) {
						let old_monitor_rect = old_monitor_info.monitorInfo.rcMonitor;
						window_pos.x = old_monitor_rect.left;
						window_pos.y = old_monitor_rect.top;
						window_pos.cx = old_monitor_rect.right - old_monitor_rect.left;
						window_pos.cy = old_monitor_rect.bottom - old_monitor_rect.top;
					}
				}
			}
		}
	}

	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_windowposchanged(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;

	let windowpos = lparam as *const WINDOWPOS;
	if unsafe { (*windowpos).flags & SWP_NOMOVE != SWP_NOMOVE } {
		let physical_position = unsafe { PhysicalPosition::new((*windowpos).x, (*windowpos).y) };
		userdata.send_window_event(window, WindowEvent::Moved(physical_position));

		unsafe { reposition_owned_windows(window) };
	}

	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_wm_size(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;
	let w = loword(lparam as u32) as u32;
	let h = hiword(lparam as u32) as u32;
	let physical_size = PhysicalSize::new(w, h);

	{
		let mut w = userdata.window_state_lock();
		if !w.window_flags().contains(WindowFlags::MARKER_RETAIN_STATE_ON_SIZE) {
			let maximized = wparam == SIZE_MAXIMIZED as usize;
			w.set_window_flags_in_place(|f| f.set(WindowFlags::MAXIMIZED, maximized));
		}
	}

	let minimized = wparam == SIZE_MINIMIZED as usize;
	let occlusion_changed = {
		let mut state = userdata.window_state_lock();
		let changed = state.occluded != minimized;
		state.occluded = minimized;
		changed
	};
	// Report un-occlusion before the resize that follows a restore, so the application resumes
	// rendering before it sees the new size.
	if occlusion_changed && !minimized {
		userdata.send_window_event(window, WindowEvent::Occluded(false));
	}

	let mut state = userdata.window_state_lock();
	if (w, h) != (0, 0) && physical_size != state.surface_size {
		state.surface_size = physical_size;
		drop(state);
		userdata.send_window_event(window, WindowEvent::Resized(physical_size));
	} else {
		drop(state);
	}

	if occlusion_changed && minimized {
		userdata.send_window_event(window, WindowEvent::Occluded(true));
	}
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_sizing(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, lparam, userdata, .. } = *m;
	fn snap_to_nearest_increment_delta(value: i32, increment: i32) -> i32 {
		let half_one = increment / 2;
		let half_two = increment - half_one;
		half_one - (value - half_two) % increment
	}

	let scale_factor = userdata.window_state_lock().scale_factor;
	let Some(inc) = userdata
		.window_state_lock()
		.resize_increments
		.map(|inc| inc.to_physical(scale_factor))
		.filter(|inc| inc.width > 0 && inc.height > 0)
	else {
		*result = ProcResult::Value(0);
		return;
	};

	let side = wparam as u32;
	let rect = unsafe { &mut *(lparam as *mut RECT) };

	let adj_rect =
		userdata.window_state_lock().window_flags.adjust_rect(window, *rect).unwrap_or(*rect);
	let deco_width = rect.left - adj_rect.left + adj_rect.right - rect.right;
	let deco_height = rect.top - adj_rect.top + adj_rect.bottom - rect.bottom;

	let width = rect.right - rect.left - deco_width;
	let height = rect.bottom - rect.top - deco_height;

	let mut width_delta = snap_to_nearest_increment_delta(width, inc.width as i32);
	let mut height_delta = snap_to_nearest_increment_delta(height, inc.height as i32);

	let min_size =
		userdata.window_state_lock().min_surface_size.map(|size| size.to_physical(scale_factor));
	let max_size =
		userdata.window_state_lock().max_surface_size.map(|size| size.to_physical(scale_factor));
	let final_width = width + width_delta;
	let final_height = height + height_delta;
	if let Some(min_size) = min_size {
		if final_width < min_size.width as i32 {
			width_delta += min_size.width as i32 - final_width;
		}
		if final_height < min_size.height as i32 {
			height_delta += min_size.height as i32 - final_height;
		}
	}
	if let Some(max_size) = max_size {
		if final_width > max_size.width as i32 {
			width_delta -= final_width - max_size.width as i32;
		}
		if final_height > max_size.height as i32 {
			height_delta -= final_height - max_size.height as i32;
		}
	}

	match side {
		WMSZ_LEFT | WMSZ_BOTTOMLEFT | WMSZ_TOPLEFT => {
			rect.left -= width_delta;
		}
		WMSZ_RIGHT | WMSZ_BOTTOMRIGHT | WMSZ_TOPRIGHT => {
			rect.right += width_delta;
		}
		_ => {}
	}

	match side {
		WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT => {
			rect.top -= height_delta;
		}
		WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT => {
			rect.bottom += height_delta;
		}
		_ => {}
	}

	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_wm_getminmaxinfo(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, lparam, userdata, .. } = *m;
	let mmi = lparam as *mut MINMAXINFO;

	let window_state = userdata.window_state_lock();
	let window_flags = window_state.window_flags;

	if window_state.min_surface_size.is_some() || window_state.max_surface_size.is_some() {
		if let Some(min_size) = window_state.min_surface_size {
			let min_size = min_size.to_physical(window_state.scale_factor);
			let size = window_flags.adjust_size(window, min_size);
			unsafe {
				(*mmi).ptMinTrackSize = POINT { x: size.width as i32, y: size.height as i32 }
			};
		}
		if let Some(max_size) = window_state.max_surface_size {
			let max_size = max_size.to_physical(window_state.scale_factor);
			let size = window_flags.adjust_size(window, max_size);
			unsafe {
				(*mmi).ptMaxTrackSize = POINT { x: size.width as i32, y: size.height as i32 }
			};
		}
	}

	*result = ProcResult::Value(0);
}

unsafe fn reposition_owned_windows(parent: HWND) {
	unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
		use windows_sys::Win32::UI::WindowsAndMessaging::GWL_WNDPROC;

		if unsafe { GetWindow(hwnd, GW_OWNER) } != lparam as HWND {
			return true.into();
		}

		let window_proc = unsafe { get_window_long(hwnd, GWL_WNDPROC) };
		if window_proc != public_window_callback as *const () as isize {
			return true.into();
		}

		let userdata_ptr = unsafe { get_window_long(hwnd, GWL_USERDATA) } as *mut WindowUserData;
		if !userdata_ptr.is_null() {
			let userdata = unsafe { &*userdata_ptr };
			if userdata.window_state_lock().window_flags.contains(WindowFlags::ANCHORED) {
				reposition_owned_popup(hwnd, &userdata.window_state);
			}
		}

		true.into()
	}

	unsafe {
		EnumThreadWindows(GetCurrentThreadId(), Some(enum_proc), parent as LPARAM);
	}
}
