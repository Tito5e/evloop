use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};

use crate::event_loop::window_proc::user_data::WindowUserData;

#[derive(Clone, Copy)]
pub(crate) struct WindowMessage<'a> {
	pub window: HWND,
	pub msg: u32,
	pub wparam: WPARAM,
	pub lparam: LPARAM,
	pub userdata: &'a WindowUserData,
}
