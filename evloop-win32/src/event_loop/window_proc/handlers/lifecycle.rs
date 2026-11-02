use std::ptr;

use evloop_core::window::WindowId;
use windows_sys::Win32::{
	Graphics::Gdi::{RDW_INTERNALPAINT, RedrawWindow, SC_SCREENSAVE},
	System::Ole::RevokeDragDrop,
	UI::WindowsAndMessaging::{DefWindowProcW, GWL_USERDATA, SC_MINIMIZE, SC_RESTORE},
};

use crate::{
	event::WindowEvent,
	event_loop::window_proc::message::WindowMessage,
	sys::{proc_result::ProcResult, window_long::set_window_long},
	window::WindowFlags,
};

pub(crate) fn on_wm_close(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, userdata, .. } = *m;
	userdata.send_window_event(window, WindowEvent::CloseRequested);
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_destroy(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, userdata, .. } = *m;
	unsafe { RevokeDragDrop(window) };
	userdata.send_window_event(window, WindowEvent::Destroyed);
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_ncdestroy(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, userdata, .. } = *m;
	unsafe { set_window_long(window, GWL_USERDATA, 0) };
	userdata.event_loop_runner.unregister_window(WindowId::from_raw(window as usize));
	userdata.userdata_removed.set(true);
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_paint(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, msg, wparam, lparam, userdata, .. } = *m;
	/*if userdata.event_loop_runner.source_drag.get().is_some() {
		*result = ProcResult::Value(unsafe { DefWindowProcW(window, msg, wparam, lparam) });
		return;
	}*/

	userdata.window_state_lock().redraw_requested = userdata.event_loop_runner.should_buffer();

	if !userdata.event_loop_runner.should_buffer() {
		userdata.send_window_event(window, WindowEvent::RedrawRequested);
	}

	*result = ProcResult::Value(unsafe { DefWindowProcW(window, msg, wparam, lparam) });
	if std::mem::take(&mut userdata.window_state_lock().redraw_requested) {
		unsafe { RedrawWindow(window, ptr::null(), ptr::null_mut(), RDW_INTERNALPAINT) };
	}
}

pub(crate) fn on_wm_syscommand(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { wparam, userdata, .. } = *m;
	if wparam == SC_RESTORE as usize {
		let mut w = userdata.window_state_lock();
		w.set_window_flags_in_place(|f| f.set(WindowFlags::MINIMIZED, false));
	}
	if wparam == SC_MINIMIZE as usize {
		let mut w = userdata.window_state_lock();
		w.set_window_flags_in_place(|f| f.set(WindowFlags::MINIMIZED, true));
	}

	if wparam == SC_SCREENSAVE as usize {
		let window_state = userdata.window_state_lock();
		if window_state.fullscreen.is_some() {
			*result = ProcResult::Value(0);
			return;
		}
	}

	*result = ProcResult::DefWindowProc(wparam);
}
