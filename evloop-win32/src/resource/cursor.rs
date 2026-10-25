use std::{ffi::c_void, io, ptr, sync::Arc};

use cursor_icon::CursorIcon;
use evloop_core::{cursor::CursorImage, error::CustomCursorError};
use windows_sys::{
	Win32::{
		Graphics::Gdi::{
			CreateBitmap, CreateCompatibleBitmap, DeleteObject, GetDC, ReleaseDC, SetBitmapBits,
		},
		UI::WindowsAndMessaging::{
			CreateIconIndirect, DestroyCursor, HCURSOR, HICON, ICONINFO, IDC_APPSTARTING,
			IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL, IDC_SIZENESW,
			IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT,
		},
	},
	core::PCWSTR,
};

#[derive(Debug, Clone)]
pub enum SelectedCursor {
	Named(CursorIcon),
	Custom(Arc<RaiiCursor>),
}

impl Default for SelectedCursor {
	fn default() -> Self {
		Self::Named(Default::default())
	}
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Cursor(pub(crate) Arc<RaiiCursor>);

impl Cursor {
	pub(crate) fn new(image: &CursorImage) -> Result<Self, CustomCursorError> {
		let mut bgra = Vec::from(image.buffer());
		bgra.chunks_exact_mut(4).for_each(|chunk| chunk.swap(0, 2));

		let w = image.width() as i32;
		let h = image.height() as i32;

		unsafe {
			let hdc_screen = GetDC(ptr::null_mut());
			if hdc_screen.is_null() {
				return Err(os_error!(io::Error::last_os_error()).into());
			}
			let hbm_color = CreateCompatibleBitmap(hdc_screen, w, h);
			ReleaseDC(ptr::null_mut(), hdc_screen);
			if hbm_color.is_null() {
				return Err(os_error!(io::Error::last_os_error()).into());
			}
			if SetBitmapBits(hbm_color, bgra.len() as u32, bgra.as_ptr() as *const c_void) == 0 {
				DeleteObject(hbm_color);
				return Err(os_error!(io::Error::last_os_error()).into());
			};

			// Mask created according to https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createbitmap#parameters
			let mask_bits: Vec<u8> = vec![0xff; ((((w + 15) >> 4) << 1) * h) as usize];
			let hbm_mask = CreateBitmap(w, h, 1, 1, mask_bits.as_ptr() as *const _);
			if hbm_mask.is_null() {
				DeleteObject(hbm_color);
				return Err(os_error!(io::Error::last_os_error()).into());
			}

			let icon_info = ICONINFO {
				fIcon: 0,
				xHotspot: image.hotspot_x() as u32,
				yHotspot: image.hotspot_y() as u32,
				hbmMask: hbm_mask,
				hbmColor: hbm_color,
			};

			let handle = CreateIconIndirect(&icon_info as *const _);
			DeleteObject(hbm_color);
			DeleteObject(hbm_mask);
			if handle.is_null() {
				return Err(os_error!(io::Error::last_os_error()).into());
			}

			Ok(Self(Arc::new(RaiiCursor { handle })))
		}
	}

	pub fn is_animated(&self) -> bool {
		false
	}
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct RaiiCursor {
	handle: HCURSOR,
}

unsafe impl Send for RaiiCursor {}

unsafe impl Sync for RaiiCursor {}

impl Drop for RaiiCursor {
	fn drop(&mut self) {
		unsafe { DestroyCursor(self.handle) };
	}
}

impl RaiiCursor {
	pub fn as_raw_handle(&self) -> HICON {
		self.handle
	}
}

pub(crate) fn to_windows_cursor(cursor: CursorIcon) -> PCWSTR {
	match cursor {
		CursorIcon::Default => IDC_ARROW,
		CursorIcon::Pointer => IDC_HAND,
		CursorIcon::Crosshair => IDC_CROSS,
		CursorIcon::Text | CursorIcon::VerticalText => IDC_IBEAM,
		CursorIcon::NotAllowed | CursorIcon::NoDrop => IDC_NO,
		CursorIcon::Grab | CursorIcon::Grabbing | CursorIcon::Move | CursorIcon::AllScroll => {
			IDC_SIZEALL
		}
		CursorIcon::EResize
		| CursorIcon::WResize
		| CursorIcon::EwResize
		| CursorIcon::ColResize => IDC_SIZEWE,
		CursorIcon::NResize
		| CursorIcon::SResize
		| CursorIcon::NsResize
		| CursorIcon::RowResize => IDC_SIZENS,
		CursorIcon::NeResize | CursorIcon::SwResize | CursorIcon::NeswResize => IDC_SIZENESW,
		CursorIcon::NwResize | CursorIcon::SeResize | CursorIcon::NwseResize => IDC_SIZENWSE,
		CursorIcon::Wait => IDC_WAIT,
		CursorIcon::Progress => IDC_APPSTARTING,
		CursorIcon::Help => IDC_HELP,
		_ => IDC_ARROW, // use arrow for the missing cases.
	}
}
