use std::ffi::c_void;

use windows_sys::{
	Win32::{
		Foundation::{HWND, POINT, POINTL},
		System::Com::{FORMATETC, STGMEDIUM},
		UI::Shell::SHDRAGIMAGE,
	},
	core::{BOOL, GUID, HRESULT},
};

use crate::sys::com::IUnknownVTable;

pub type IAdviseSink = *mut c_void;
pub type IDataObject = *mut c_void;
pub type IEnumFORMATETC = *mut c_void;
pub type IEnumSTATDATA = *mut c_void;

#[repr(C)]
pub struct IDataObjectVTable {
	pub parent: IUnknownVTable,
	pub GetData: unsafe extern "system" fn(
		This: *mut IDataObject,
		pformatetcIn: *const FORMATETC,
		pmedium: *mut STGMEDIUM,
	) -> HRESULT,
	pub GetDataHere: unsafe extern "system" fn(
		This: *mut IDataObject,
		pformatetc: *const FORMATETC,
		pmedium: *mut STGMEDIUM,
	) -> HRESULT,
	pub QueryGetData:
		unsafe extern "system" fn(This: *mut IDataObject, pformatetc: *const FORMATETC) -> HRESULT,
	pub GetCanonicalFormatEtc: unsafe extern "system" fn(
		This: *mut IDataObject,
		pformatetcIn: *const FORMATETC,
		pformatetcOut: *mut FORMATETC,
	) -> HRESULT,
	pub SetData: unsafe extern "system" fn(
		This: *mut IDataObject,
		pformatetc: *const FORMATETC,
		pmedium: *const STGMEDIUM,
		fRelease: BOOL,
	) -> HRESULT,
	pub EnumFormatEtc: unsafe extern "system" fn(
		This: *mut IDataObject,
		dwDirection: u32,
		ppenumFormatEtc: *mut *mut IEnumFORMATETC,
	) -> HRESULT,
	pub DAdvise: unsafe extern "system" fn(
		This: *mut IDataObject,
		pformatetc: *const FORMATETC,
		advf: u32,
		pAdvSInk: *const IAdviseSink,
		pdwConnection: *mut u32,
	) -> HRESULT,
	pub DUnadvise: unsafe extern "system" fn(This: *mut IDataObject, dwConnection: u32) -> HRESULT,
	pub EnumDAdvise: unsafe extern "system" fn(
		This: *mut IDataObject,
		ppenumAdvise: *const *const IEnumSTATDATA,
	) -> HRESULT,
}

#[repr(C)]
pub struct IEnumFORMATETCVTable {
	pub parent: IUnknownVTable,
	pub Next: unsafe extern "system" fn(
		This: *mut IEnumFORMATETC,
		celt: u32,
		rgelt: *mut FORMATETC,
		pceltFetched: *mut u32,
	) -> HRESULT,
	pub Skip: unsafe extern "system" fn(This: *mut IEnumFORMATETC, celt: u32) -> HRESULT,
	pub Reset: unsafe extern "system" fn(This: *mut IEnumFORMATETC) -> HRESULT,
	pub Clone: unsafe extern "system" fn(
		This: *mut IEnumFORMATETC,
		ppenum: *mut *mut IEnumFORMATETC,
	) -> HRESULT,
}

pub type IDragSourceHelper = *mut c_void;

#[repr(C)]
pub struct IDragSourceHelperVTable {
	pub parent: IUnknownVTable,
	pub InitializeFromBitmap: unsafe extern "system" fn(
		This: *mut IDragSourceHelper,
		pshdi: *const SHDRAGIMAGE,
		pDataObject: *mut IDataObject,
	) -> HRESULT,
	pub InitializeFromWindow: unsafe extern "system" fn(
		This: *mut IDragSourceHelper,
		hwnd: HWND,
		ppt: *const POINT,
		pDataObject: *mut IDataObject,
	) -> HRESULT,
}

pub type IDropTargetHelper = *mut c_void;

#[repr(C)]
pub struct IDropTargetHelperVTable {
	pub parent: IUnknownVTable,
	pub DragEnter: unsafe extern "system" fn(
		This: *mut IDropTargetHelper,
		hwndTarget: HWND,
		pDataObject: *mut IDataObject,
		ppt: *const POINT,
		dwEffect: u32,
	) -> HRESULT,
	pub DragLeave: unsafe extern "system" fn(This: *mut IDropTargetHelper) -> HRESULT,
	pub DragOver: unsafe extern "system" fn(
		This: *mut IDropTargetHelper,
		ppt: *const POINT,
		dwEffect: u32,
	) -> HRESULT,
	pub Drop: unsafe extern "system" fn(
		This: *mut IDropTargetHelper,
		pDataObject: *mut IDataObject,
		ppt: *const POINT,
		dwEffect: u32,
	) -> HRESULT,
	pub Show: unsafe extern "system" fn(This: *mut IDropTargetHelper, fShow: BOOL) -> HRESULT,
}

pub type IDropSource = *mut c_void;

#[repr(C)]
pub struct IDropSourceVTable {
	pub parent: IUnknownVTable,
	pub QueryContinueDrag: unsafe extern "system" fn(
		This: *mut IDropSource,
		fEscapePressed: BOOL,
		grfKeyState: u32,
	) -> HRESULT,
	pub GiveFeedback: unsafe extern "system" fn(This: *mut IDropSource, dwEffect: u32) -> HRESULT,
}

#[repr(C)]
pub struct IDropTargetVTable {
	pub parent: IUnknownVTable,
	pub DragEnter: unsafe extern "system" fn(
		This: *mut IDropTarget,
		pDataObj: *const IDataObject,
		grfKeyState: u32,
		pt: POINTL,
		pdwEffect: *mut u32,
	) -> HRESULT,
	pub DragOver: unsafe extern "system" fn(
		This: *mut IDropTarget,
		grfKeyState: u32,
		pt: POINTL,
		pdwEffect: *mut u32,
	) -> HRESULT,
	pub DragLeave: unsafe extern "system" fn(This: *mut IDropTarget) -> HRESULT,
	pub Drop: unsafe extern "system" fn(
		This: *mut IDropTarget,
		pDataObj: *const IDataObject,
		grfKeyState: u32,
		pt: POINTL,
		pdwEffect: *mut u32,
	) -> HRESULT,
}

#[repr(C)]
pub struct IDropTarget {
	pub lpVtbl: *const IDropTargetVTable,
}

/// Defined in `objidl.h`.
pub const IID_IDataObject: GUID = GUID::from_u128(0x0000010e_0000_0000_c000_000000000046);

/// Defined in `oleidl.h`.
pub const IID_IDropSource: GUID = GUID::from_u128(0x00000121_0000_0000_c000_000000000046);

/// Defined in `objidl.h`.
pub const IID_IEnumFORMATETC: GUID = GUID::from_u128(0x00000103_0000_0000_c000_000000000046);

/// Defined in `shobjidl_core.h`.
pub const IID_IDragSourceHelper: GUID = GUID::from_u128(0xde5bf786_477a_11d2_839d_00c04fd918d0);

/// Defined in `shobjidl_core.h`.
pub const IID_IDropTargetHelper: GUID = GUID::from_u128(0x4657278b_411b_11d2_839a_00c04fd918d0);
