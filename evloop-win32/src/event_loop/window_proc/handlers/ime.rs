use evloop_core::ime::Ime;
use windows_sys::Win32::UI::{
	Input::Ime::{GCS_COMPSTR, GCS_RESULTSTR, ISC_SHOWUICOMPOSITIONWINDOW},
	WindowsAndMessaging::DefWindowProcW,
};

use crate::{
	event::WindowEvent, event_loop::window_proc::message::WindowMessage, input::ImeContext,
	sys::proc_result::ProcResult, window::ImeState,
};

// `Ime::Enabled` / `Ime::Disabled` report the IME *session* (see `evloop_core::ime`), which is
// started and ended by `Window::request_ime_update` (`on_ime_session`). A composition only
// produces `Preedit` / `Commit` events.

pub(crate) fn on_wm_ime_startcomposition(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { wparam, userdata, .. } = *m;
	let mut state = userdata.window_state_lock();
	if state.ime_capabilities.is_some() {
		state.ime_state = ImeState::Preedit;
	}

	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_wm_ime_composition(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, lparam, userdata, .. } = *m;
	let ime_allowed_and_composing = {
		let w = userdata.window_state_lock();
		w.ime_capabilities.is_some() && w.ime_state != ImeState::Disabled
	};

	if ime_allowed_and_composing {
		let ime_context = unsafe { ImeContext::current(window) };

		if lparam == 0 {
			userdata.send_window_event(window, WindowEvent::Ime(Ime::Preedit(String::new(), None)));
		}

		if (lparam as u32 & GCS_RESULTSTR) != 0 {
			if let Some(text) = unsafe { ime_context.get_composed_text() } {
				userdata.window_state_lock().ime_state = ImeState::Enabled;
				userdata
					.send_window_event(window, WindowEvent::Ime(Ime::Preedit(String::new(), None)));
				userdata.send_window_event(window, WindowEvent::Ime(Ime::Commit(text)));
			}
		}

		if (lparam as u32 & GCS_COMPSTR) != 0 {
			if let Some((text, first, last)) =
				unsafe { ime_context.get_composing_text_and_cursor() }
			{
				userdata.window_state_lock().ime_state = ImeState::Preedit;
				let cursor_range = first.map(|f| (f, last.unwrap_or(f)));

				userdata
					.send_window_event(window, WindowEvent::Ime(Ime::Preedit(text, cursor_range)));
			}
		}
	}

	*result = ProcResult::Value(0);
}

pub(crate) fn on_wm_ime_endcomposition(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;
	let was_composing = {
		let w = userdata.window_state_lock();
		w.ime_capabilities.is_some() && w.ime_state == ImeState::Preedit
	};
	if was_composing {
		let ime_context = unsafe { ImeContext::current(window) };
		userdata.send_window_event(window, WindowEvent::Ime(Ime::Preedit(String::new(), None)));
		if let Some(text) = unsafe { ime_context.get_composed_text() }.filter(|t| !t.is_empty()) {
			userdata.send_window_event(window, WindowEvent::Ime(Ime::Commit(text)));
		}
		userdata.window_state_lock().ime_state = ImeState::Enabled;
	}

	*result = ProcResult::DefWindowProc(wparam);
}

pub(crate) fn on_wm_ime_setcontext(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, msg, wparam, lparam, .. } = *m;
	let lparam = lparam & !(ISC_SHOWUICOMPOSITIONWINDOW as isize);
	*result = ProcResult::Value(unsafe { DefWindowProcW(window, msg, wparam, lparam) });
}

/// Handles `IME_SESSION_MSG_ID`, posted by `Window::request_ime_update`.
pub(crate) fn on_ime_session(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;
	if wparam != 0 {
		userdata.send_window_event(window, WindowEvent::Ime(Ime::Enabled));
	} else {
		let was_composing = {
			let mut state = userdata.window_state_lock();
			let composing = state.ime_state == ImeState::Preedit;
			state.ime_state = ImeState::Disabled;
			composing
		};
		if was_composing {
			userdata.send_window_event(window, WindowEvent::Ime(Ime::Preedit(String::new(), None)));
		}
		userdata.send_window_event(window, WindowEvent::Ime(Ime::Disabled));
	}
	*result = ProcResult::Value(0);
}
