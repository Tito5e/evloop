use windows_sys::{
	Win32::{
		Foundation::{CLIPBRD_E_CANT_OPEN, HGLOBAL},
		System::{
			Com::{DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED_HGLOBAL},
			Ole::ReleaseStgMedium,
		},
	},
	core::HRESULT,
};

use crate::sys::com::ole::{IDataObject, IDataObjectVTable};

pub(crate) struct OwnedStgMedium(pub(crate) STGMEDIUM);

impl OwnedStgMedium {
	pub(crate) unsafe fn get(
		data_obj: *const IDataObject,
		cf_format: u16,
	) -> Result<Option<Self>, HRESULT> {
		let format = FORMATETC {
			cfFormat: cf_format,
			ptd: std::ptr::null_mut(),
			dwAspect: DVASPECT_CONTENT,
			lindex: -1,
			tymed: TYMED_HGLOBAL as u32,
		};

		let mut medium = unsafe { std::mem::zeroed::<STGMEDIUM>() };
		let get_data = unsafe { (*(*data_obj).cast::<IDataObjectVTable>()).GetData };
		match unsafe { get_data(data_obj as *mut _, &format, &mut medium) } {
			CLIPBRD_E_CANT_OPEN => Err(CLIPBRD_E_CANT_OPEN),
			hr if hr < 0 => Ok(None),
			_ => Ok(Some(Self(medium))),
		}
	}

	pub(crate) fn hglobal(&self) -> HGLOBAL {
		unsafe { self.0.u.hGlobal }
	}
}

impl Drop for OwnedStgMedium {
	fn drop(&mut self) {
		unsafe { ReleaseStgMedium(&mut self.0) };
	}
}
