use std::{collections::HashMap, ffi::c_void, ptr};

use evloop_core::{
	data_transfer::{SendData, TypeHint},
	error::TransferError,
};
use windows_sys::{
	Win32::{Foundation::S_OK, System::Ole::OleGetClipboard},
	core::HRESULT,
};

use crate::{
	data_transfer::{ole::ensure_ole_initialized, receive::reader::DataObjectReader},
	sys::com::{
		IUnknown,
		ole::{IDataObject, IDataObjectVTable},
	},
};

#[derive(Debug, Default)]
pub(crate) struct DataSnapshot {
	data: HashMap<TypeHint, SendData>,
}

impl DataSnapshot {
	pub(crate) unsafe fn from_idataobject(data_obj: *const IDataObject) -> Result<Self, HRESULT> {
		let reader = DataObjectReader(data_obj);
		let mut data = HashMap::new();

		if let Some(text) = unsafe { reader.read_unicode_text() }? {
			data.insert(TypeHint::PlainText, SendData::String(text));
		}

		if let Some(uris) = unsafe { reader.read_uri_list() }? {
			if !uris.is_empty() {
				data.insert(TypeHint::UriList, SendData::Uris(uris));
			}
		}

		if let Some(png) = unsafe { reader.read_png() }? {
			if !png.is_empty() {
				data.insert(TypeHint::Image { extension_hint: Some("png") }, SendData::Bytes(png));
			}
		}

		Ok(Self { data })
	}

	pub(crate) fn resolve(&self, requested: TypeHint) -> Option<TypeHint> {
		self.data.keys().copied().find(|stored| stored.matches(&requested))
	}

	pub(crate) fn from_clipboard() -> Result<Self, TransferError> {
		ensure_ole_initialized().map_err(|err| os_error!(err))?;

		let mut ptr: *mut c_void = ptr::null_mut();
		let hr = unsafe { OleGetClipboard(&mut ptr) };
		if hr != S_OK || ptr.is_null() {
			return Err(os_error!(format!("OleGetClipboard failed: 0x{hr:08x}")).into());
		}
		let data_obj = ptr as *mut IDataObject;

		let data = unsafe { DataSnapshot::from_idataobject(data_obj) };

		let release = unsafe { (*(*data_obj).cast::<IDataObjectVTable>()).parent.Release };
		unsafe { release(data_obj as *mut IUnknown) };

		data.map_err(|hr| os_error!(format!("Clipboard read failed: 0x{hr:08x}")).into())
	}
}
