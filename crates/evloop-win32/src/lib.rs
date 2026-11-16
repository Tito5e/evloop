#![cfg(target_os = "windows")] // FIXME(Tito5e): Hack for allow compiling on all platforms.

use windows_sys::Win32::{
    Foundation::{HMODULE, HWND},
    System::SystemServices::IMAGE_DOS_HEADER,
    UI::WindowsAndMessaging::WINDOW_LONG_PTR_INDEX,
};

mod encoding;
pub mod event_loop;

pub(crate) fn get_instance_handle() -> HMODULE {
    unsafe extern "C" {
        static __ImageBase: IMAGE_DOS_HEADER;
    }

    unsafe { &__ImageBase as *const _ as _ }
}

#[inline]
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
        use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongW;

        SetWindowLongW(hwnd, nindex, dwnewlong as i32) as isize
    };
}

#[inline]
pub(crate) unsafe fn get_window_long(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX) -> isize {
    #[cfg(target_pointer_width = "64")]
    return unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;

        GetWindowLongPtrW(hwnd, nindex)
    };

    #[cfg(target_pointer_width = "32")]
    return unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongW;

        GetWindowLongW(hwnd, nindex) as isize
    };
}
