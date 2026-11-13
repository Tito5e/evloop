mod appearance;
mod attributes;
mod dark_mode;
mod dwm;
mod flags;
mod fullscreen;
mod geometry;
mod init;
mod placement;
mod pointer;
mod rwh;
mod state;
mod status;
mod taskbar;
mod r#type;

use std::{
	ptr,
	sync::{Arc, Mutex, MutexGuard},
};

pub use attributes::*;
pub use dark_mode::*;
use evloop_core::{error::CreateWindowError, window::WindowId};
pub use flags::*;
pub use init::*;
pub use placement::*;
pub use state::*;
pub use status::*;
pub use taskbar::*;
use windows_sys::Win32::{
	Foundation::HWND,
	Graphics::Gdi::{RDW_INTERNALPAINT, RedrawWindow},
	UI::WindowsAndMessaging::PostMessageW,
};

use crate::{
	event_loop::{ActiveEventLoop, EventLoopThreadExecutor},
	monitor::Fullscreen,
	sys::message_id::DESTROY_MSG_ID,
	window::{rwh::SyncWindowHandle, r#type::WindowType},
};

#[derive(Debug)]
pub struct Window {
	window: SyncWindowHandle,
	window_state: Arc<Mutex<WindowState>>,
	thread_executor: EventLoopThreadExecutor,
}

// Core Implementation
impl Window {
	pub(crate) fn new(
		event_loop: &ActiveEventLoop,
		w_attr: WindowAttributes,
	) -> Result<Window, CreateWindowError> {
		let window = unsafe { init(w_attr, &event_loop.0) }?;
		window.reposition();
		Ok(window)
	}

	pub fn hwnd(&self) -> HWND {
		self.window.hwnd()
	}

	pub fn id(&self) -> WindowId {
		WindowId::from_raw(self.hwnd() as usize)
	}

	pub fn window_type(&self) -> WindowType {
		self.window_state_lock().window_type
	}

	fn window_state_lock(&self) -> MutexGuard<'_, WindowState> {
		self.window_state.lock().unwrap()
	}

	pub fn set_blur(&self, _blur: bool) {
		// TODO: [UPSTREAM] ?
	}

	pub fn pre_present_notify(&self) {}
}

impl Drop for Window {
	fn drop(&mut self) {
		if matches!(self.fullscreen(), Some(Fullscreen::Exclusive(_, _))) {
			self.set_fullscreen(None);
		}

		unsafe {
			PostMessageW(self.hwnd(), DESTROY_MSG_ID.get(), 0, 0);
		}
	}
}
