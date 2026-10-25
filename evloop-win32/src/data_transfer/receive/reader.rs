use std::{ffi::OsString, os::windows::ffi::OsStringExt as _, path::PathBuf};

use windows_sys::{
	Win32::{
		System::{
			DataExchange::RegisterClipboardFormatW,
			Memory::{GlobalLock, GlobalSize, GlobalUnlock},
			Ole::{CF_HDROP, CF_UNICODETEXT},
		},
		UI::Shell::{DragQueryFileW, HDROP},
	},
	core::HRESULT,
};

use crate::{
	data_transfer::uri::file_path_to_uri,
	sys::{
		com::{ole::IDataObject, stg_medium::OwnedStgMedium},
		wide::encode_wide,
	},
};

pub(crate) struct DataObjectReader(pub(crate) *const IDataObject);

impl DataObjectReader {
	pub(crate) unsafe fn read_unicode_text(&self) -> Result<Option<String>, HRESULT> {
		let data_obj = self.0;
		let Some(medium) = unsafe { OwnedStgMedium::get(data_obj, CF_UNICODETEXT) }? else {
			return Ok(None);
		};
		let hglobal = medium.hglobal();

		let ptr = unsafe { GlobalLock(hglobal) };
		if ptr.is_null() {
			return Ok(None);
		}

		// `CF_UNICODETEXT` is a NUL-terminated UTF-16 string. Cap the scan by the allocation size in
		// case the buffer isn't terminated.
		let max_units = unsafe { GlobalSize(hglobal) } / std::mem::size_of::<u16>();
		let wide = unsafe { std::slice::from_raw_parts(ptr.cast::<u16>(), max_units) };
		let len = wide.iter().position(|&c| c == 0).unwrap_or(max_units);
		let text = String::from_utf16_lossy(&wide[..len]);

		unsafe { GlobalUnlock(hglobal) };

		Ok(Some(text))
	}

	pub(crate) unsafe fn read_uri_list(&self) -> Result<Option<Vec<String>>, HRESULT> {
		let data_obj = self.0;
		let Some(medium) = unsafe { OwnedStgMedium::get(data_obj, CF_HDROP) }? else {
			return Ok(None);
		};
		let hdrop = medium.hglobal() as HDROP;

		let item_count = unsafe { DragQueryFileW(hdrop, 0xffff_ffff, std::ptr::null_mut(), 0) };

		let mut paths = Vec::with_capacity(item_count as usize);
		for i in 0..item_count {
			let character_count =
				unsafe { DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0) } as usize;

			let mut path_buf = Vec::<u16>::with_capacity(character_count + 1);
			let copied = unsafe {
				DragQueryFileW(hdrop, i, path_buf.as_mut_ptr(), character_count as u32 + 1)
			} as usize;
			unsafe { path_buf.set_len(copied) };

			let path = PathBuf::from(OsString::from_wide(&path_buf));

			paths.push(
				file_path_to_uri(&path).unwrap_or_else(|| path.to_string_lossy().into_owned()),
			);
		}

		Ok(Some(paths))
	}

	pub(crate) unsafe fn read_png(&self) -> Result<Option<Vec<u8>>, HRESULT> {
		let data_obj = self.0;
		let format_name = encode_wide("PNG");
		let format = unsafe { RegisterClipboardFormatW(format_name.as_ptr()) };
		if format == 0 {
			return Ok(None);
		}

		let Some(medium) = unsafe { OwnedStgMedium::get(data_obj, format as u16) }? else {
			return Ok(None);
		};
		let hglobal = medium.hglobal();

		let ptr = unsafe { GlobalLock(hglobal) };
		if ptr.is_null() {
			return Ok(None);
		}

		let len = unsafe { GlobalSize(hglobal) };
		let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) }.to_vec();

		unsafe { GlobalUnlock(hglobal) };

		Ok(Some(bytes))
	}
}
