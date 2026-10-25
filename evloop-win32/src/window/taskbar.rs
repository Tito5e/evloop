use std::{cell::Cell, ptr};

use windows_sys::Win32::{
	Foundation::{HWND, S_OK},
	System::Com::{CLSCTX_ALL, CoCreateInstance},
};

use crate::sys::com::{
	apartment::com_initialized,
	shell::{CLSID_TaskbarList, IID_ITaskbarList, IID_ITaskbarList2, ITaskbarList, ITaskbarList2},
};

thread_local! {
	static TASKBAR_LIST: Cell<*mut ITaskbarList> = const { Cell::new(ptr::null_mut()) };
	static TASKBAR_LIST2: Cell<*mut ITaskbarList2> = const { Cell::new(ptr::null_mut()) };
}

// Reference Implementation:
// https://github.com/chromium/chromium/blob/f18e79d901f56154f80eea1e2218544285e62623/ui/views/win/fullscreen_handler.cc
//
// As per MSDN marking the window as fullscreen should ensure that the
// taskbar is moved to the bottom of the Z-order when the fullscreen window
// is activated. If the window is not fullscreen, the Shell falls back to
// heuristics to determine how the window should be treated, which means
// that it could still consider the window as fullscreen. :(
pub(crate) unsafe fn taskbar_mark_fullscreen(handle: HWND, fullscreen: bool) {
	com_initialized();

	TASKBAR_LIST2.with(|task_bar_list2_ptr| {
		let mut task_bar_list2 = task_bar_list2_ptr.get();

		if task_bar_list2.is_null() {
			let hr = unsafe {
				CoCreateInstance(
					&CLSID_TaskbarList,
					ptr::null_mut(),
					CLSCTX_ALL,
					&IID_ITaskbarList2,
					&mut task_bar_list2 as *mut _ as *mut _,
				)
			};
			if hr != S_OK {
				// In visual studio retrieving the taskbar list fails
				return;
			}

			let hr_init = unsafe { (*(*task_bar_list2).lpVtbl).parent.HrInit };
			if unsafe { hr_init(task_bar_list2.cast()) } != S_OK {
				// In some old windows, the taskbar object could not be created, we just ignore it
				return;
			}
			task_bar_list2_ptr.set(task_bar_list2)
		}

		task_bar_list2 = task_bar_list2_ptr.get();
		let mark_fullscreen_window = unsafe { (*(*task_bar_list2).lpVtbl).MarkFullscreenWindow };
		unsafe { mark_fullscreen_window(task_bar_list2, handle, fullscreen.into()) };
	})
}

pub(crate) unsafe fn set_skip_taskbar(hwnd: HWND, skip: bool) {
	com_initialized();
	TASKBAR_LIST.with(|task_bar_list_ptr| {
		let mut task_bar_list = task_bar_list_ptr.get();

		if task_bar_list.is_null() {
			let hr = unsafe {
				CoCreateInstance(
					&CLSID_TaskbarList,
					ptr::null_mut(),
					CLSCTX_ALL,
					&IID_ITaskbarList,
					&mut task_bar_list as *mut _ as *mut _,
				)
			};
			if hr != S_OK {
				// In visual studio retrieving the taskbar list fails
				return;
			}

			let hr_init = unsafe { (*(*task_bar_list).lpVtbl).HrInit };
			if unsafe { hr_init(task_bar_list.cast()) } != S_OK {
				// In some old windows, the taskbar object could not be created, we just ignore it
				return;
			}
			task_bar_list_ptr.set(task_bar_list)
		}

		task_bar_list = task_bar_list_ptr.get();
		if skip {
			let delete_tab = unsafe { (*(*task_bar_list).lpVtbl).DeleteTab };
			unsafe { delete_tab(task_bar_list, hwnd) };
		} else {
			let add_tab = unsafe { (*(*task_bar_list).lpVtbl).AddTab };
			unsafe { add_tab(task_bar_list, hwnd) };
		}
	});
}
