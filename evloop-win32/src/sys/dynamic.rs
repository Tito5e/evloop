use std::{ffi::c_void, sync::LazyLock};

use windows_sys::{
	Win32::{
		Foundation::{HANDLE, HMODULE, HWND, NTSTATUS, RECT},
		Graphics::Gdi::HMONITOR,
		System::{
			LibraryLoader::{GetProcAddress, LoadLibraryA},
			SystemInformation::OSVERSIONINFOW,
			SystemServices::IMAGE_DOS_HEADER,
		},
		UI::{
			HiDpi::{DPI_AWARENESS_CONTEXT, MONITOR_DPI_TYPE, PROCESS_DPI_AWARENESS},
			Input::Pointer::{POINTER_INFO, POINTER_PEN_INFO, POINTER_TOUCH_INFO},
		},
	},
	core::{BOOL, HRESULT},
};

pub(crate) fn get_function_impl(library: &str, function: &str) -> Option<*const c_void> {
	assert_eq!(library.chars().last(), Some('\0'));
	assert_eq!(function.chars().last(), Some('\0'));

	// 非ASCII文字を名前に持つライブラリを使用しないので、文字列の変換を含まないA版の関数を使用する
	let module = unsafe { LoadLibraryA(library.as_ptr()) };
	if module.is_null() {
		return None;
	}

	unsafe { GetProcAddress(module, function.as_ptr()) }.map(|function_ptr| function_ptr as _)
}

macro_rules! get_function {
	($lib:expr, $func:ident) => {
		$crate::sys::dynamic::get_function_impl(
			concat!($lib, '\0'),
			concat!(stringify!($func), '\0'),
		)
		.map(|f| unsafe { std::mem::transmute::<*const _, $func>(f) })
	};
}

pub type SetProcessDPIAware = unsafe extern "system" fn() -> BOOL;
pub type SetProcessDpiAwareness =
	unsafe extern "system" fn(value: PROCESS_DPI_AWARENESS) -> HRESULT;
pub type SetProcessDpiAwarenessContext =
	unsafe extern "system" fn(value: DPI_AWARENESS_CONTEXT) -> BOOL;
pub type GetDpiForWindow = unsafe extern "system" fn(hwnd: HWND) -> u32;
pub type GetDpiForMonitor = unsafe extern "system" fn(
	hmonitor: HMONITOR,
	dpi_type: MONITOR_DPI_TYPE,
	dpi_x: *mut u32,
	dpi_y: *mut u32,
) -> HRESULT;
pub type EnableNonClientDpiScaling = unsafe extern "system" fn(hwnd: HWND) -> BOOL;
pub type AdjustWindowRectExForDpi = unsafe extern "system" fn(
	rect: *mut RECT,
	dw_style: u32,
	b_menu: BOOL,
	dw_ex_style: u32,
	dpi: u32,
) -> BOOL;

pub type GetPointerFrameInfoHistory = unsafe extern "system" fn(
	pointer_id: u32,
	entries_count: *mut u32,
	pointer_count: *mut u32,
	pointer_info: *mut POINTER_INFO,
) -> BOOL;

pub type SkipPointerFrameMessages = unsafe extern "system" fn(pointer_id: u32) -> BOOL;
pub type GetPointerDeviceRects = unsafe extern "system" fn(
	device: HANDLE,
	pointer_device_rect: *mut RECT,
	display_rect: *mut RECT,
) -> BOOL;

pub type GetPointerTouchInfo =
	unsafe extern "system" fn(pointer_id: u32, touch_info: *mut POINTER_TOUCH_INFO) -> BOOL;
pub type GetPointerPenInfo =
	unsafe extern "system" fn(pointer_id: u32, pen_info: *mut POINTER_PEN_INFO) -> BOOL;

pub(crate) static WIN10_BUILD_VERSION: LazyLock<Option<u32>> = LazyLock::new(|| {
	type RtlGetVersion = unsafe extern "system" fn(*mut OSVERSIONINFOW) -> NTSTATUS;
	let handle = get_function!("ntdll.dll", RtlGetVersion);

	if let Some(rtl_get_version) = handle {
		unsafe {
			let mut vi = std::mem::zeroed();

			let status = (rtl_get_version)(&mut vi);

			if status >= 0 && vi.dwMajorVersion == 10 && vi.dwMinorVersion == 0 {
				Some(vi.dwBuildNumber)
			} else {
				None
			}
		}
	} else {
		None
	}
});

pub(crate) static GET_DPI_FOR_WINDOW: LazyLock<Option<GetDpiForWindow>> =
	LazyLock::new(|| get_function!("user32.dll", GetDpiForWindow));
pub(crate) static ADJUST_WINDOW_RECT_EX_FOR_DPI: LazyLock<Option<AdjustWindowRectExForDpi>> =
	LazyLock::new(|| get_function!("user32.dll", AdjustWindowRectExForDpi));
pub(crate) static GET_DPI_FOR_MONITOR: LazyLock<Option<GetDpiForMonitor>> =
	LazyLock::new(|| get_function!("shcore.dll", GetDpiForMonitor));
pub(crate) static ENABLE_NON_CLIENT_DPI_SCALING: LazyLock<Option<EnableNonClientDpiScaling>> =
	LazyLock::new(|| get_function!("user32.dll", EnableNonClientDpiScaling));
pub(crate) static SET_PROCESS_DPI_AWARENESS_CONTEXT: LazyLock<
	Option<SetProcessDpiAwarenessContext>,
> = LazyLock::new(|| get_function!("user32.dll", SetProcessDpiAwarenessContext));
pub(crate) static SET_PROCESS_DPI_AWARENESS: LazyLock<Option<SetProcessDpiAwareness>> =
	LazyLock::new(|| get_function!("shcore.dll", SetProcessDpiAwareness));
pub(crate) static SET_PROCESS_DPI_AWARE: LazyLock<Option<SetProcessDPIAware>> =
	LazyLock::new(|| get_function!("user32.dll", SetProcessDPIAware));
pub(crate) static GET_POINTER_FRAME_INFO_HISTORY: LazyLock<Option<GetPointerFrameInfoHistory>> =
	LazyLock::new(|| get_function!("user32.dll", GetPointerFrameInfoHistory));
pub(crate) static SKIP_POINTER_FRAME_MESSAGES: LazyLock<Option<SkipPointerFrameMessages>> =
	LazyLock::new(|| get_function!("user32.dll", SkipPointerFrameMessages));
pub(crate) static GET_POINTER_DEVICE_RECTS: LazyLock<Option<GetPointerDeviceRects>> =
	LazyLock::new(|| get_function!("user32.dll", GetPointerDeviceRects));
pub(crate) static GET_POINTER_TOUCH_INFO: LazyLock<Option<GetPointerTouchInfo>> =
	LazyLock::new(|| get_function!("user32.dll", GetPointerTouchInfo));
pub(crate) static GET_POINTER_PEN_INFO: LazyLock<Option<GetPointerPenInfo>> =
	LazyLock::new(|| get_function!("user32.dll", GetPointerPenInfo));

// Quoted from `winit`
pub fn get_instance_handle() -> HMODULE {
	// Gets the instance handle by taking the address of the
	// pseudo-variable created by the microsoft linker:
	// https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483

	// This is preferred over GetModuleHandle(NULL) because it also works in DLLs:
	// https://stackoverflow.com/questions/21718027/getmodulehandlenull-vs-hinstance
	unsafe extern "C" {
		static __ImageBase: IMAGE_DOS_HEADER;
	}

	unsafe { &__ImageBase as *const _ as _ }
}
