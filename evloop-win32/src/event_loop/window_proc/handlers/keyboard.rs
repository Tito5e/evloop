use windows_sys::Win32::{
	Foundation::HWND,
	UI::WindowsAndMessaging::{GetMenu, MNC_CLOSE, WM_SYSKEYDOWN, WM_SYSKEYUP},
};

use crate::{
	event::WindowEvent::ModifiersChanged,
	event_loop::window_proc::{message::WindowMessage, user_data::WindowUserData},
	sys::proc_result::ProcResult,
};

pub(crate) fn on_wm_menuchar(_m: &WindowMessage<'_>, result: &mut ProcResult) {
	*result = ProcResult::Value((MNC_CLOSE << 16) as isize);
}

pub(crate) fn on_wm_inputlangchange(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { wparam, .. } = *m;

	{
		let mut layouts = LAYOUT_CACHE.lock().unwrap();
		layouts.get_current_layout();
	}

	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_key_down(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { msg, wparam, .. } = *m;

	if msg == WM_SYSKEYDOWN {
		*result = ProcResult::DefWindowProc(wparam);
	}
}

pub(crate) fn on_key_up(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, msg, wparam, .. } = *m;
	if msg == WM_SYSKEYUP && unsafe { !GetMenu(window).is_null() } {
		*result = ProcResult::DefWindowProc(wparam);
	}
}

pub(crate) fn update_modifiers(window: HWND, userdata: &WindowUserData) {
	let modifiers = {
		let mut layouts = LAYOUT_CACHE.lock().unwrap();
		layouts.get_agnostic_mods()
	};

	let mut window_state = userdata.window_state.lock().unwrap();
	if window_state.modifiers_state != modifiers {
		window_state.modifiers_state = modifiers;

		drop(window_state);

		userdata.send_window_event(window, ModifiersChanged(modifiers.into()));
	}
}
