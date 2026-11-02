use evloop_core::keyboard::ModifiersState;
use windows_sys::Win32::Foundation::HWND;

use crate::{
	event::WindowEvent,
	event_loop::window_proc::{
		keyboard::update_modifiers, message::WindowMessage, user_data::WindowUserData,
	},
	sys::proc_result::ProcResult,
};

pub(crate) fn on_wm_ncactivate(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;

	let is_active = wparam != false.into();
	let active_focus_changed = userdata.window_state_lock().set_active(is_active);
	if active_focus_changed {
		if is_active {
			unsafe { gain_active_focus(window, userdata) };
		} else {
			unsafe { lose_active_focus(window, userdata) };
		}
	}
	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_wm_setfocus(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, userdata, .. } = *m;
	let active_focus_changed = userdata.window_state_lock().set_focused(true);
	if active_focus_changed {
		unsafe { gain_active_focus(window, userdata) };
	}
	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_killfocus(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, userdata, .. } = *m;
	let active_focus_changed = userdata.window_state_lock().set_focused(false);
	if active_focus_changed {
		unsafe { lose_active_focus(window, userdata) };
	}
	*result = ProcResult::Value(0);
}

unsafe fn gain_active_focus(window: HWND, userdata: &WindowUserData) {
	update_modifiers(window, userdata);

	userdata.send_window_event(window, WindowEvent::Focused(true));
}

unsafe fn lose_active_focus(window: HWND, userdata: &WindowUserData) {
	userdata.window_state_lock().modifiers_state = ModifiersState::empty();
	userdata
		.send_window_event(window, WindowEvent::ModifiersChanged(ModifiersState::empty().into()));

	userdata.send_window_event(window, WindowEvent::Focused(false));
}
