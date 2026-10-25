use std::ptr;

use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

struct ComInitialized(#[allow(dead_code)] *mut ());

impl Drop for ComInitialized {
	fn drop(&mut self) {
		unsafe { CoUninitialize() };
	}
}

thread_local! {
	static COM_INITIALIZED: ComInitialized = {
		unsafe {
			CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED as u32);
			ComInitialized(ptr::null_mut())
		}
	};
}

pub fn com_initialized() {
	COM_INITIALIZED.with(|_| {});
}
