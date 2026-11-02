use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;

use crate::{
	event_loop::window_proc::message::WindowMessage,
	sys::{
		message_id::{DESTROY_MSG_ID, SET_RETAIN_STATE_ON_SIZE_MSG_ID, TASKBAR_CREATED},
		proc_result::ProcResult,
	},
	window::{WindowFlags, set_skip_taskbar},
};

pub(crate) fn on_custom_message(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, msg, wparam, userdata, .. } = *m;
	if msg == DESTROY_MSG_ID.get() {
		unsafe { DestroyWindow(window) };
		*result = ProcResult::Value(0);
	} else if msg == SET_RETAIN_STATE_ON_SIZE_MSG_ID.get() {
		let mut window_state = userdata.window_state_lock();
		window_state.set_window_flags_in_place(|f| {
			f.set(WindowFlags::MARKER_RETAIN_STATE_ON_SIZE, wparam != 0)
		});
		*result = ProcResult::Value(0);
	} else if msg == TASKBAR_CREATED.get() {
		let skip_taskbar = userdata.window_state_lock().skip_taskbar;
		unsafe { set_skip_taskbar(window, skip_taskbar) };
		*result = ProcResult::DefWindowProc(wparam);
	} else {
		*result = ProcResult::DefWindowProc(wparam);
	}
}
