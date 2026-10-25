#![allow(non_snake_case, non_upper_case_globals)]

pub(crate) mod apartment;
pub(crate) mod ole;
pub(crate) mod shell;
pub(crate) mod stg_medium;

use std::ffi::c_void;

use windows_sys::core::{GUID, HRESULT};

pub type IUnknown = *mut c_void;

#[repr(C)]
pub struct IUnknownVTable {
	pub QueryInterface: unsafe extern "system" fn(
		This: *mut IUnknown,
		riid: *const GUID,
		ppvObject: *mut *mut c_void,
	) -> HRESULT,
	pub AddRef: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
	pub Release: unsafe extern "system" fn(This: *mut IUnknown) -> u32,
}

pub(super) fn guids_eq(a: &GUID, b: &GUID) -> bool {
	a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

macro_rules! com_iunknown_impl {
	($ty:ty, $extra_iid:expr) => {
		#[allow(non_snake_case)]
		impl $ty {
			unsafe fn from_interface<'a, I>(this: *mut I) -> &'a mut Self {
				unsafe { &mut *(this as *mut _) }
			}

			unsafe extern "system" fn QueryInterface(
				this: *mut $crate::sys::com::IUnknown,
				riid: *const ::windows_sys::core::GUID,
				ppv: *mut *mut ::std::ffi::c_void,
			) -> ::windows_sys::core::HRESULT {
				let riid = unsafe { &*riid };
				if $crate::sys::com::guids_eq(riid, &::windows_sys::core::IID_IUnknown)
					|| $crate::sys::com::guids_eq(riid, $extra_iid)
				{
					unsafe { *ppv = this as *mut ::std::ffi::c_void };
					unsafe { Self::AddRef(this) };
					::windows_sys::Win32::Foundation::S_OK
				} else {
					unsafe { *ppv = std::ptr::null_mut() };
					::windows_sys::Win32::Foundation::E_NOINTERFACE
				}
			}

			unsafe extern "system" fn AddRef(this: *mut $crate::sys::com::IUnknown) -> u32 {
				let me = unsafe { Self::from_interface(this) };
				me.refcount.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed) as u32 + 1
			}

			unsafe extern "system" fn Release(this: *mut $crate::sys::com::IUnknown) -> u32 {
				let me = unsafe { Self::from_interface(this) };
				let count = me.refcount.fetch_sub(1, ::std::sync::atomic::Ordering::Release) - 1;
				if count == 0 {
					::std::sync::atomic::fence(::std::sync::atomic::Ordering::Acquire);
					drop(unsafe { Box::from_raw(me as *mut Self) });
				}
				count as u32
			}
		}
	};
}
