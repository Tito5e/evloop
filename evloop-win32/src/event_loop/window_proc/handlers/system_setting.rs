use crate::{
	event::WindowEvent, event_loop::window_proc::message::WindowMessage,
	sys::proc_result::ProcResult, window::try_theme,
};

pub(crate) fn on_wm_settingchange(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;

	let preferred_theme = userdata.window_state_lock().preferred_theme;

	if preferred_theme.is_none() {
		let new_theme = try_theme(window, preferred_theme, false);
		let mut window_state = userdata.window_state_lock();

		if window_state.current_theme != new_theme {
			window_state.current_theme = new_theme;
			drop(window_state);
			userdata.send_window_event(window, WindowEvent::ThemeChanged(new_theme));
		}
	}
	*result = ProcResult::DefWindowProc(wparam);
}
