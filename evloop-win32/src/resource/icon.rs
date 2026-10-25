use core::fmt;
use std::{io, mem, path::Path, ptr, sync::Arc};

use evloop_core::icon::{BadIcon, RgbaIcon};
use geometry_types::PhysicalSize;
use windows_sys::{
	Win32::UI::WindowsAndMessaging::{
		CreateIcon, DestroyIcon, HICON, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTSIZE,
		LR_LOADFROMFILE, LoadImageW,
	},
	core::PCWSTR,
};

use crate::sys::{dynamic::get_instance_handle, wide::encode_wide};

pub(crate) const PIXEL_SIZE: usize = mem::size_of::<u32>();

/// Windows specific `Icon`.
///
/// Windows icons can be created from files, or from the [`embedded resources`](https://learn.microsoft.com/en-us/windows/win32/menurc/about-resource-files).
///
/// The `ICON` resource definition statement use the following syntax:
/// ```rc
/// nameID ICON filename
/// ```
/// `nameID` is a unique name or a 16-bit unsigned integer value identifying the resource,
/// `filename` is the name of the file that contains the resource.
///
/// More information about the `ICON` resource can be found at [`Microsoft Learn`](https://learn.microsoft.com/en-us/windows/win32/menurc/icon-resource) portal.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Icon {
	pub(crate) inner: Arc<RaiiIcon>,
}

unsafe impl Send for Icon {}

impl Icon {
	pub(crate) fn from_path_impl<P: AsRef<Path>>(
		path: P,
		size: Option<PhysicalSize>,
	) -> Result<Self, BadIcon> {
		// width / height of 0 along with LR_DEFAULTSIZE tells windows to load the default icon size
		let (width, height) = size.map(|size| (size.width, size.height)).unwrap_or((0, 0));

		let wide_path = encode_wide(path.as_ref());

		let handle = unsafe {
			LoadImageW(
				ptr::null_mut(),
				wide_path.as_ptr(),
				IMAGE_ICON,
				width as i32,
				height as i32,
				LR_DEFAULTSIZE | LR_LOADFROMFILE,
			)
		};
		if !handle.is_null() {
			Ok(Icon::from_handle(handle as HICON))
		} else {
			Err(BadIcon::OsError(io::Error::last_os_error()))
		}
	}

	pub(crate) fn from_resource_impl(
		resource_id: u16,
		size: Option<PhysicalSize>,
	) -> Result<Self, BadIcon> {
		Self::from_resource_ptr(resource_id as PCWSTR, size)
	}

	pub(crate) fn from_resource_name_impl(
		resource_name: &str,
		size: Option<PhysicalSize>,
	) -> Result<Self, BadIcon> {
		let wide_name = encode_wide(resource_name);
		Self::from_resource_ptr(wide_name.as_ptr(), size)
	}

	fn from_resource_ptr(resource: PCWSTR, size: Option<PhysicalSize>) -> Result<Self, BadIcon> {
		// width / height of 0 along with LR_DEFAULTSIZE tells windows to load the default icon size
		let (width, height) = size.map(|size| (size.width, size.height)).unwrap_or((0, 0));
		let handle = unsafe {
			LoadImageW(
				get_instance_handle(),
				resource,
				IMAGE_ICON,
				width as i32,
				height as i32,
				LR_DEFAULTSIZE,
			)
		};
		if !handle.is_null() {
			Ok(Icon::from_handle(handle as HICON))
		} else {
			Err(BadIcon::OsError(io::Error::last_os_error()))
		}
	}

	pub(crate) fn as_raw_handle(&self) -> HICON {
		self.inner.handle
	}

	pub(crate) fn from_rgba(rgba: &RgbaIcon) -> Result<Self, BadIcon> {
		let pixel_count = rgba.buffer().len() / PIXEL_SIZE;
		let mut and_mask = Vec::with_capacity(pixel_count);
		let mut bgra = Vec::with_capacity(rgba.buffer().len());
		for pixel in rgba.buffer().chunks_exact(PIXEL_SIZE) {
			and_mask.push(pixel[3].wrapping_sub(u8::MAX)); // invert alpha channel
			bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
		}
		let handle = unsafe {
			CreateIcon(
				ptr::null_mut(),
				rgba.width() as i32,
				rgba.height() as i32,
				1,
				(PIXEL_SIZE * 8) as u8,
				and_mask.as_ptr(),
				bgra.as_ptr(),
			)
		};
		if !handle.is_null() {
			Ok(Icon::from_handle(handle))
		} else {
			Err(BadIcon::OsError(io::Error::last_os_error()))
		}
	}

	fn from_handle(handle: HICON) -> Self {
		Self { inner: Arc::new(RaiiIcon { handle }) }
	}
}

impl fmt::Debug for Icon {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		(*self.inner).fmt(formatter)
	}
}

impl Icon {
	/// Create an icon from a file path.
	///
	/// Specify `size` to load a specific icon size from the file, or `None` to load the default
	/// icon size from the file.
	///
	/// In cases where the specified size does not exist in the file, Windows may perform scaling
	/// to get an icon of the desired size.
	pub fn from_path<P: AsRef<Path>>(path: P, size: Option<PhysicalSize>) -> Result<Self, BadIcon> {
		Self::from_path_impl(path, size)
	}

	/// Create an icon from a resource embedded in this executable or library by its ordinal id.
	///
	/// The valid `ordinal` values range from 1 to [`u16::MAX`] (inclusive). The value `0` is an
	/// invalid ordinal id, but it can be used with [`from_resource_name`] as `"0"`.
	///
	/// [`from_resource_name`]: Self::from_resource_name
	///
	/// Specify `size` to load a specific icon size from the file, or `None` to load the default
	/// icon size from the file.
	///
	/// In cases where the specified size does not exist in the file, Windows may perform scaling
	/// to get an icon of the desired size.
	pub fn from_resource(resource_id: u16, size: Option<PhysicalSize>) -> Result<Self, BadIcon> {
		Self::from_resource_impl(resource_id, size)
	}

	/// Create an icon from a resource embedded in this executable or library by its name.
	///
	/// Specify `size` to load a specific icon size from the file, or `None` to load the default
	/// icon size from the file.
	///
	/// In cases where the specified size does not exist in the file, Windows may perform scaling
	/// to get an icon of the desired size.
	///
	/// # Notes
	///
	/// Consider the following resource definition statements:
	/// ```rc
	/// app     ICON "app.ico"
	/// 1       ICON "a.ico"
	/// 0027    ICON "custom.ico"
	/// 0       ICON "alt.ico"
	/// ```
	///
	/// Due to some internal implementation details of the resource embedding/loading process on
	/// Windows platform, strings that can be interpreted as 16-bit unsigned integers (`"1"`,
	/// `"002"`, etc.) cannot be used as valid resource names, and instead should be passed into
	/// [`from_resource`]:
	///
	/// [`from_resource`]: Self::from_resource
	///
	/// ```rust,no_run
	/// use winit::platform::windows::WinIcon;
	///
	/// assert!(WinIcon::from_resource_name("app", None).is_ok());
	/// assert!(WinIcon::from_resource(1, None).is_ok());
	/// assert!(WinIcon::from_resource(27, None).is_ok());
	/// assert!(WinIcon::from_resource_name("27", None).is_err());
	/// assert!(WinIcon::from_resource_name("0027", None).is_err());
	/// ```
	///
	/// While `0` cannot be used as an ordinal id (see [`from_resource`]), it can be used as a
	/// name:
	///
	/// [`from_resource`]: IconExtWindows::from_resource
	///
	/// ```rust,no_run
	/// # use winit::platform::windows::WinIcon;
	/// # use winit::icon::Icon;
	/// assert!(WinIcon::from_resource_name("0", None).is_ok());
	/// assert!(WinIcon::from_resource(0, None).is_err());
	/// ```
	pub fn from_resource_name(
		resource_name: &str,
		size: Option<PhysicalSize>,
	) -> Result<Self, BadIcon> {
		Self::from_resource_name_impl(resource_name, size)
	}
}

#[derive(Debug, Clone, Copy)]
pub enum IconType {
	Small = ICON_SMALL as isize,
	Big = ICON_BIG as isize,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct RaiiIcon {
	handle: HICON,
}

unsafe impl Send for RaiiIcon {}

unsafe impl Sync for RaiiIcon {}

impl Drop for RaiiIcon {
	fn drop(&mut self) {
		unsafe { DestroyIcon(self.handle) };
	}
}
