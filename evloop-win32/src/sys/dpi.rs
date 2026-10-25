#![allow(non_snake_case)] // 通常の関数とポインタ関数を区別するためにあえて命名規則を変更する

use std::sync::Once;

use windows_sys::Win32::{
	Foundation::{HWND, S_OK},
	Graphics::Gdi::{
		GetDC, GetDeviceCaps, HMONITOR, LOGPIXELSX, MONITOR_DEFAULTTONEAREST, MonitorFromWindow,
		ReleaseDC,
	},
	UI::{
		HiDpi::{
			DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
			MDT_EFFECTIVE_DPI, PROCESS_PER_MONITOR_DPI_AWARE,
		},
		WindowsAndMessaging::IsProcessDPIAware,
	},
};

use crate::sys::dynamic::{
	ENABLE_NON_CLIENT_DPI_SCALING, GET_DPI_FOR_MONITOR, GET_DPI_FOR_WINDOW, SET_PROCESS_DPI_AWARE,
	SET_PROCESS_DPI_AWARENESS, SET_PROCESS_DPI_AWARENESS_CONTEXT,
};

pub fn become_dpi_aware() {
	static ENABLE_DPI_AWARENESS: Once = Once::new();
	ENABLE_DPI_AWARENESS.call_once(|| unsafe {
		if let Some(SetProcessDpiArarenessContext) = *SET_PROCESS_DPI_AWARENESS_CONTEXT {
			// Windows 10 Anniv Update (1607)以降
			if SetProcessDpiArarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
				== false.into()
			{
				// V2はWindows 10 Creators Update (1703)以降でのみ動作するので、V1にフォールバックする
				SetProcessDpiArarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
			}
		} else if let Some(SetProcessDpiAwareness) = *SET_PROCESS_DPI_AWARENESS {
			// Windows 8.1以降
			SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
		} else if let Some(SetProcessDPIAware) = *SET_PROCESS_DPI_AWARE {
			// Vista以降
			SetProcessDPIAware();
		}
	});
}

pub fn enable_non_client_dpi_scaling(hwnd: HWND) {
	unsafe {
		if let Some(EnableNonClientDpiScaling) = *ENABLE_NON_CLIENT_DPI_SCALING {
			EnableNonClientDpiScaling(hwnd);
		}
	}
}

pub fn get_monitor_dpi(hmonitor: HMONITOR) -> Option<u32> {
	unsafe {
		if let Some(GetDpiForMonitor) = *GET_DPI_FOR_MONITOR {
			// Windows 8.1以降
			let mut dpi_x = 0;
			let mut dpi_y = 0;
			if GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == S_OK {
				// MSDNによるとこの２つの値は同一になるらしい
				// https://msdn.microsoft.com/en-us/library/windows/desktop/dn280510(v=vs.85).aspx
				return Some(dpi_x);
			}
		}
	}
	None
}

pub const BASE_DPI: u32 = 96;

pub fn dpi_to_scale_factor(dpi: u32) -> f64 {
	dpi as f64 / BASE_DPI as f64
}

pub unsafe fn get_window_dpi(hwnd: HWND) -> u32 {
	if let Some(GetDpiForWindow) = *GET_DPI_FOR_WINDOW {
		// Windows 10 Anniv Update (1607)以降
		//
		// GetDpiForWindowを使用すればウィンドウの存在するモニターの正確なDPIを知ることができる
		match unsafe { GetDpiForWindow(hwnd) } {
			0 => BASE_DPI, // hwndが不正なのでBASE_DPIを返す
			dpi => dpi,
		}
	} else if let Some(GetDpiForMonitor) = *GET_DPI_FOR_MONITOR {
		// Windows 8.1以降
		//
		// GetDpiForWindowは使用できないので、DPI_AWARENESSに影響される古いAPIにフォールバックする
		let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
		if monitor.is_null() {
			return BASE_DPI;
		}

		let mut dpi_x = 0;
		let mut dpi_y = 0;
		if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } == S_OK {
			dpi_x
		} else {
			BASE_DPI
		}
	} else {
		// Vista以降
		if unsafe { IsProcessDPIAware() } != false.into() {
			let hdc = unsafe { GetDC(hwnd) };
			if hdc.is_null() {
				// TODO: より良いハンドリングが無いか検討する
				panic!("[evloop] `GetDC` returned null!");
			}
			let dpi = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) as u32 };
			unsafe { ReleaseDC(hwnd, hdc) };
			dpi
		} else {
			// プロセスがDPI_UNAWAREなので、基準となるDPIを返す
			BASE_DPI
		}
	}
}
