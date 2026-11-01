use std::num::NonZeroIsize;

use raw_window_handle::{
	DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
	RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::GWLP_HINSTANCE};

use crate::{sys::window_long::get_window_long, window::Window};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct SyncWindowHandle(HWND);

impl SyncWindowHandle {
	pub(crate) fn new(hwnd: HWND) -> Self {
		Self(hwnd)
	}

	pub(crate) fn hwnd(&self) -> HWND {
		self.0
	}
}

unsafe impl Send for SyncWindowHandle {}

unsafe impl Sync for SyncWindowHandle {}

impl Window {
	pub(crate) unsafe fn raw_window_handle_unchecked(&self) -> RawWindowHandle {
		let mut window_handle = Win32WindowHandle::new(unsafe {
			NonZeroIsize::new_unchecked(self.window.hwnd() as isize)
		});
		let hinstance = unsafe { get_window_long(self.hwnd(), GWLP_HINSTANCE) };
		window_handle.hinstance = NonZeroIsize::new(hinstance);
		RawWindowHandle::Win32(window_handle)
	}

	pub fn window_handle_any_thread(&self) -> Result<WindowHandle<'_>, HandleError> {
		unsafe {
			let handle = self.raw_window_handle_unchecked();

			Ok(WindowHandle::borrow_raw(handle))
		}
	}
}

impl HasDisplayHandle for Window {
	fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
		let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
		unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
	}
}

impl HasWindowHandle for Window {
	fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
		if !self.thread_executor.in_event_loop_thread() {
			tracing::error!("tried to access window handle outside of the main thread");
			return Err(HandleError::Unavailable);
		}

		let raw = unsafe { self.raw_window_handle_unchecked() };
		unsafe { Ok(WindowHandle::borrow_raw(raw)) }
	}
}
