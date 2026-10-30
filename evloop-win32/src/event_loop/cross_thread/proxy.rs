use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::PostMessageW};

use crate::sys::message_id::USER_EVENT_MSG_ID;

#[derive(Debug)]
pub struct EventLoopProxy {
	pub(crate) target_window: HWND,
}

unsafe impl Send for EventLoopProxy {}

unsafe impl Sync for EventLoopProxy {}

impl EventLoopProxy {
	pub fn wake_up(&self) {
		unsafe { PostMessageW(self.target_window, USER_EVENT_MSG_ID.get(), 0, 0) };
	}
}
