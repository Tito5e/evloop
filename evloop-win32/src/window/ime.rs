use evloop_core::{
	error::ImeRequestError,
	ime::{ImeCapabilities, ImeRequest, ImeRequestData},
};
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::{
	input::ImeContext,
	sys::message_id::IME_SESSION_MSG_ID,
	window::{Window, state::ImeState},
};

impl Window {
	/// Starts, updates or ends the IME session of this window. See `evloop_core::ime` for the
	/// semantics shared by all backends.
	///
	/// On Windows (IMM32):
	/// - `cursor_area` places the composition and candidate windows.
	/// - `hint_and_purpose` and `surrounding_text` are accepted but have no effect.
	/// - `Ime::Enabled` / `Ime::Disabled` are delivered asynchronously, after this call returns.
	pub fn request_ime_update(&self, request: ImeRequest) -> Result<(), ImeRequestError> {
		match request {
			ImeRequest::Enable(enable) => {
				let (capabilities, data) = enable.into_raw();
				{
					let mut state = self.window_state_lock();
					if state.ime_capabilities.is_some() {
						return Err(ImeRequestError::AlreadyEnabled);
					}
					state.ime_capabilities = Some(capabilities);
					state.ime_state = ImeState::Enabled;
				}
				self.set_ime_session(true);
				self.apply_ime_request_data(capabilities, data);
				Ok(())
			}
			ImeRequest::Update(data) => {
				let capabilities =
					self.window_state_lock().ime_capabilities.ok_or(ImeRequestError::NotEnabled)?;
				self.apply_ime_request_data(capabilities, data);
				Ok(())
			}
			ImeRequest::Disable => {
				self.window_state_lock()
					.ime_capabilities
					.take()
					.ok_or(ImeRequestError::NotEnabled)?;
				self.set_ime_session(false);
				Ok(())
			}
		}
	}

	/// The capabilities of the current IME session, or `None` when no session is enabled.
	pub fn ime_capabilities(&self) -> Option<ImeCapabilities> {
		self.window_state_lock().ime_capabilities
	}

	fn set_ime_session(&self, enabled: bool) {
		let window = self.window;
		self.thread_executor.execute_in_thread(move || unsafe {
			ImeContext::set_ime_allowed(window.hwnd(), enabled);
			// Reported through the window procedure so the event goes through the runner.
			PostMessageW(window.hwnd(), IME_SESSION_MSG_ID.get(), enabled as usize, 0);
		});
	}

	fn apply_ime_request_data(&self, capabilities: ImeCapabilities, data: ImeRequestData) {
		let Some((position, size)) = data.cursor_area.filter(|_| capabilities.cursor_area()) else {
			return;
		};
		let window = self.window;
		let scale_factor = self.scale_factor();
		self.thread_executor.execute_in_thread(move || unsafe {
			ImeContext::current(window.hwnd()).set_ime_cursor_area(position, size, scale_factor);
		});
	}
}
