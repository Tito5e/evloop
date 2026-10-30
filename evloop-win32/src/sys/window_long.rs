use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX};

#[inline(always)]
pub(crate) unsafe fn get_window_long(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX) -> isize {
	#[cfg(target_pointer_width = "64")]
	return unsafe {
		use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;

		GetWindowLongPtrW(hwnd, nindex)
	};
	#[cfg(target_pointer_width = "32")]
	return unsafe {
		use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;

		GetWindowLongPtrW(hwnd, nindex) as isize
	};
}

#[inline(always)]
pub(crate) unsafe fn set_window_long(
	hwnd: HWND,
	nindex: WINDOW_LONG_PTR_INDEX,
	dwnewlong: isize,
) -> isize {
	#[cfg(target_pointer_width = "64")]
	return unsafe {
		use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;

		SetWindowLongPtrW(hwnd, nindex, dwnewlong)
	};
	#[cfg(target_pointer_width = "32")]
	return unsafe {
		use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;

		SetWindowLongPtrW(hwnd, nindex, dwnewlong as i32) as isize
	};
}
