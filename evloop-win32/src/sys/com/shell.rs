use windows_sys::{
	Win32::Foundation::HWND,
	core::{BOOL, GUID, HRESULT},
};

use crate::sys::com::IUnknownVTable;

#[repr(C)]
pub struct ITaskbarListVTable {
	pub parent: IUnknownVTable,
	pub HrInit: unsafe extern "system" fn(This: *mut ITaskbarList) -> HRESULT,
	pub AddTab: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
	pub DeleteTab: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
	pub ActivateTab: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
	pub SetActiveAlt: unsafe extern "system" fn(This: *mut ITaskbarList, hwnd: HWND) -> HRESULT,
}

#[repr(C)]
pub struct ITaskbarList {
	pub lpVtbl: *const ITaskbarListVTable,
}

#[repr(C)]
pub struct ITaskbarList2VTable {
	pub parent: ITaskbarListVTable,
	pub MarkFullscreenWindow: unsafe extern "system" fn(
		This: *mut ITaskbarList2,
		hwnd: HWND,
		fFullscreen: BOOL,
	) -> HRESULT,
}

#[repr(C)]
pub struct ITaskbarList2 {
	pub lpVtbl: *const ITaskbarList2VTable,
}

/// Defined in `shobjidl_core.h`.
pub const CLSID_TaskbarList: GUID = GUID {
	data1: 0x56fdf344,
	data2: 0xfd6d,
	data3: 0x11d0,
	data4: [0x95, 0x8a, 0x00, 0x60, 0x97, 0xc9, 0xa0, 0x90],
};

/// Defined in `shobjidl_core.h`.
pub const IID_ITaskbarList: GUID = GUID {
	data1: 0x56fdf342,
	data2: 0xfd6d,
	data3: 0x11d0,
	data4: [0x95, 0x8a, 0x00, 0x60, 0x97, 0xc9, 0xa0, 0x90],
};

/// Defined in `shobjidl_core.h`.
pub const IID_ITaskbarList2: GUID = GUID {
	data1: 0x602d4995,
	data2: 0xb13a,
	data3: 0x429b,
	data4: [0xa6, 0x6e, 0x19, 0x35, 0xe4, 0x4f, 0x43, 0x17],
};
