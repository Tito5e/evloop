use core::fmt;
use std::{
	collections::{HashSet, VecDeque, hash_set::IntoIter},
	hash::Hash,
	io,
	iter::Empty,
	mem,
	num::{NonZeroU16, NonZeroU32},
	ptr,
};

use evloop_core::monitor::VideoMode;
use geometry_types::{PhysicalPosition, PhysicalRect, PhysicalSize};
use windows_sys::{
	Win32::{
		Foundation::{HWND, LPARAM, POINT, RECT},
		Graphics::Gdi::{
			DEVMODEW, DM_BITSPERPEL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
			ENUM_CURRENT_SETTINGS, EnumDisplayMonitors, EnumDisplaySettingsExW, GetMonitorInfoW,
			HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
			MONITORINFOEXW, MonitorFromPoint, MonitorFromWindow,
		},
	},
	core::BOOL,
};

use crate::sys::{
	bits::has_flag,
	dpi::{dpi_to_scale_factor, get_monitor_dpi},
	wide::decode_wide,
};

#[derive(Clone)]
pub struct VideoModeHandle {
	mode: VideoMode,
	native_video_mode: Box<DEVMODEW>,
}

impl PartialEq for VideoModeHandle {
	fn eq(&self, other: &Self) -> bool {
		self.mode == other.mode
	}
}

impl Eq for VideoModeHandle {}

impl Hash for VideoModeHandle {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.mode.hash(state)
	}
}

impl fmt::Debug for VideoModeHandle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("VideoMode").field("mode", &self.mode).finish()
	}
}

impl VideoModeHandle {
	fn new(native_video_mode: DEVMODEW) -> Self {
		const REQUIRED_FIELDS: u32 =
			DM_BITSPERPEL | DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
		assert!(has_flag(native_video_mode.dmFields, REQUIRED_FIELDS));

		let mode = VideoMode::new(
			PhysicalSize {
				width: native_video_mode.dmPelsWidth,
				height: native_video_mode.dmPelsHeight,
			},
			NonZeroU16::new(native_video_mode.dmBitsPerPel as u16),
			NonZeroU32::new(native_video_mode.dmDisplayFrequency * 1000),
		);

		VideoModeHandle { mode, native_video_mode: Box::new(native_video_mode) }
	}
}

unsafe extern "system" fn monitor_enum_proc(
	hmonitor: HMONITOR,
	_hdc: HDC,
	_place: *mut RECT,
	data: LPARAM,
) -> BOOL {
	let monitors = data as *mut VecDeque<MonitorHandle>;
	unsafe { (*monitors).push_back(MonitorHandle::new(hmonitor)) };
	true.into() // continue enumeration
}

pub fn available_monitors() -> VecDeque<MonitorHandle> {
	let mut monitors: VecDeque<MonitorHandle> = VecDeque::new();
	unsafe {
		EnumDisplayMonitors(
			ptr::null_mut(),
			ptr::null(),
			Some(monitor_enum_proc),
			&mut monitors as *mut _ as LPARAM,
		);
	}
	monitors
}

pub fn primary_monitor() -> MonitorHandle {
	const ORIGIN: POINT = POINT { x: 0, y: 0 };
	let hmonitor = unsafe { MonitorFromPoint(ORIGIN, MONITOR_DEFAULTTOPRIMARY) };
	MonitorHandle::new(hmonitor)
}

pub fn current_monitor(hwnd: HWND) -> MonitorHandle {
	let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
	MonitorHandle::new(hmonitor)
}

pub(crate) fn get_monitor_info(hmonitor: HMONITOR) -> Result<MONITORINFOEXW, io::Error> {
	let mut monitor_info: MONITORINFOEXW = unsafe { mem::zeroed() };
	monitor_info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
	let status = unsafe {
		GetMonitorInfoW(hmonitor, &mut monitor_info as *mut MONITORINFOEXW as *mut MONITORINFO)
	};
	if status == false.into() { Err(io::Error::last_os_error()) } else { Ok(monitor_info) }
}

pub enum VideoModeHandleIter {
	Empty(Empty<VideoModeHandle>),
	Handles(IntoIter<VideoModeHandle>),
}

impl Iterator for VideoModeHandleIter {
	type Item = VideoModeHandle;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Empty(iter) => iter.next(),
			Self::Handles(iter) => iter.next(),
		}
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MonitorHandle(HMONITOR);

unsafe impl Send for MonitorHandle {}
unsafe impl Sync for MonitorHandle {}

impl MonitorHandle {
	pub(crate) const fn new(hmonitor: HMONITOR) -> Self {
		Self(hmonitor)
	}

	pub(crate) fn size(&self) -> PhysicalSize {
		let rc_monitor = get_monitor_info(self.0).unwrap().monitorInfo.rcMonitor;
		PhysicalSize::new(
			(rc_monitor.right - rc_monitor.left) as u32,
			(rc_monitor.bottom - rc_monitor.top) as u32,
		)
	}

	pub(crate) fn work_area(&self) -> Option<PhysicalRect> {
		let rc_work = get_monitor_info(self.0).ok()?.monitorInfo.rcWork;
		Some(PhysicalRect {
			x: rc_work.left,
			y: rc_work.top,
			width: (rc_work.right - rc_work.left) as u32,
			height: (rc_work.bottom - rc_work.top) as u32,
		})
	}

	pub(crate) fn video_mode_handles(&self) -> VideoModeHandleIter {
		let mut modes = HashSet::<VideoModeHandle>::new();

		let monitor_info = match get_monitor_info(self.0) {
			Ok(monitor_info) => monitor_info,
			Err(error) => {
				tracing::warn!("Error from get_monitor_info: {error}");
				return VideoModeHandleIter::Empty(std::iter::empty());
			}
		};

		let device_name = monitor_info.szDevice.as_ptr();
		let mut i = 0;
		loop {
			let mut mode: DEVMODEW = unsafe { mem::zeroed() };
			mode.dmSize = mem::size_of_val(&mode) as u16;
			if unsafe { EnumDisplaySettingsExW(device_name, i, &mut mode, 0) } == false.into() {
				break;
			}

			modes.insert(VideoModeHandle::new(mode));

			i += 1;
		}

		VideoModeHandleIter::Handles(modes.into_iter())
	}

	pub fn id(&self) -> u128 {
		self.native_id() as _
	}

	pub fn native_id(&self) -> u64 {
		self.0 as _
	}

	pub fn name(&self) -> Option<std::borrow::Cow<'_, str>> {
		let monitor_info = get_monitor_info(self.0).ok()?;
		Some(decode_wide(&monitor_info.szDevice).to_string_lossy().to_string().into())
	}

	pub fn position(&self) -> Option<PhysicalPosition> {
		get_monitor_info(self.0)
			.map(|info| {
				let rc_monitor = info.monitorInfo.rcMonitor;
				PhysicalPosition { x: rc_monitor.left, y: rc_monitor.top }
			})
			.ok()
	}

	pub fn scale_factor(&self) -> f64 {
		dpi_to_scale_factor(get_monitor_dpi(self.0).unwrap_or(96))
	}

	pub fn current_video_mode(&self) -> Option<VideoMode> {
		let monitor_info = get_monitor_info(self.0).ok()?;
		let device_name = monitor_info.szDevice.as_ptr();
		unsafe {
			let mut mode: DEVMODEW = mem::zeroed();
			mode.dmSize = mem::size_of_val(&mode) as u16;
			if EnumDisplaySettingsExW(device_name, ENUM_CURRENT_SETTINGS, &mut mode, 0)
				== false.into()
			{
				None
			} else {
				Some(VideoModeHandle::new(mode).mode)
			}
		}
	}

	pub fn video_modes(&self) -> impl Iterator<Item = VideoMode> {
		self.video_mode_handles().map(|mode| mode.mode)
	}
}
