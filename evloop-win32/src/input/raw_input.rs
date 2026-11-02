use std::{mem, ptr};

use evloop_core::{
	event::ElementState,
	keyboard::{KeyCode, PhysicalKey},
};
use windows_sys::Win32::{
	Devices::HumanInterfaceDevice::{
		HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_GENERIC_MOUSE, HID_USAGE_PAGE_GENERIC,
	},
	Foundation::{HANDLE, HWND},
	UI::{
		Input::{
			GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList, HRAWINPUT,
			KeyboardAndMouse::{MAPVK_VK_TO_VSC_EX, MapVirtualKeyW, VK_NUMLOCK, VK_SHIFT},
			RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST, RAWINPUTHEADER, RAWKEYBOARD,
			RID_DEVICE_INFO, RID_DEVICE_INFO_HID, RID_DEVICE_INFO_KEYBOARD, RID_DEVICE_INFO_MOUSE,
			RID_INPUT, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDEV_REMOVE, RIDI_DEVICEINFO,
			RIDI_DEVICENAME, RIM_TYPEHID, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE, RegisterRawInputDevices,
		},
		WindowsAndMessaging::{
			RI_KEY_E0, RI_KEY_E1, RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_1_UP,
			RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP, RI_MOUSE_BUTTON_3_DOWN,
			RI_MOUSE_BUTTON_3_UP, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP,
			RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP,
		},
	},
};

use crate::{
	input::{DeviceEvents, keyboard::scancode_to_physicalkey},
	sys::{bits::has_flag, wide::decode_wide},
};

pub fn get_raw_input_device_list() -> Option<Vec<RAWINPUTDEVICELIST>> {
	let list_size = size_of::<RAWINPUTDEVICELIST>() as u32;

	let mut num_devices = 0;
	let status = unsafe { GetRawInputDeviceList(ptr::null_mut(), &mut num_devices, list_size) };

	// TODO: u32::MAXでいいのか確認
	if status == u32::MAX {
		return None;
	}

	let mut buffer = Vec::with_capacity(num_devices as _);

	let num_stored =
		unsafe { GetRawInputDeviceList(buffer.as_mut_ptr(), &mut num_devices, list_size) };

	if num_stored == u32::MAX {
		return None;
	}

	debug_assert_eq!(num_devices, num_stored);

	unsafe { buffer.set_len(num_devices as _) };

	Some(buffer)
}

pub enum RawDeviceInfo {
	Mouse(RID_DEVICE_INFO_MOUSE),
	Keyboard(RID_DEVICE_INFO_KEYBOARD),
	Hid(RID_DEVICE_INFO_HID),
}

impl From<RID_DEVICE_INFO> for RawDeviceInfo {
	fn from(value: RID_DEVICE_INFO) -> Self {
		unsafe {
			match value.dwType {
				RIM_TYPEMOUSE => RawDeviceInfo::Mouse(value.Anonymous.mouse),
				RIM_TYPEKEYBOARD => RawDeviceInfo::Keyboard(value.Anonymous.keyboard),
				RIM_TYPEHID => RawDeviceInfo::Hid(value.Anonymous.hid),
				_ => unreachable!(),
			}
		}
	}
}

pub fn get_raw_input_device_info(handle: HANDLE) -> Option<RawDeviceInfo> {
	let mut info: RID_DEVICE_INFO = unsafe { mem::zeroed() };
	let info_size = mem::size_of::<RID_DEVICE_INFO>() as u32;

	info.cbSize = info_size;

	let mut minimum_size = 0;
	let status = unsafe {
		GetRawInputDeviceInfoW(handle, RIDI_DEVICEINFO, &mut info as *mut _ as _, &mut minimum_size)
	};

	// TODO: Check
	if status == u32::MAX || status == 0 {
		return None;
	}

	debug_assert_eq!(info_size, status);

	Some(info.into())
}

pub fn get_raw_input_device_name(handle: HANDLE) -> Option<String> {
	let mut minimum_size = 0;
	let status = unsafe {
		GetRawInputDeviceInfoW(handle, RIDI_DEVICENAME, ptr::null_mut(), &mut minimum_size)
	};

	if status != 0 {
		return None;
	}

	let mut name: Vec<u16> = Vec::with_capacity(minimum_size as _);

	let status = unsafe {
		GetRawInputDeviceInfoW(handle, RIDI_DEVICENAME, name.as_ptr() as _, &mut minimum_size)
	};

	if status == u32::MAX || status == 0 {
		return None;
	}

	debug_assert_eq!(minimum_size, status);

	unsafe { name.set_len(minimum_size as _) };

	decode_wide(&name).into_string().ok()
}

pub fn register_raw_input_devices(devices: &[RAWINPUTDEVICE]) -> bool {
	let device_size = size_of::<RAWINPUTDEVICE>() as u32;

	unsafe {
		RegisterRawInputDevices(devices.as_ptr(), devices.len() as u32, device_size) == true.into()
	}
}

pub fn register_all_mouse_and_keyboards_for_raw_input(
	mut window_handle: HWND,
	filter: DeviceEvents,
) -> bool {
	let flags = match filter {
		DeviceEvents::Never => {
			window_handle = ptr::null_mut();
			RIDEV_REMOVE
		}
		DeviceEvents::WhenFocused => RIDEV_DEVNOTIFY,
		DeviceEvents::Always => RIDEV_DEVNOTIFY | RIDEV_INPUTSINK,
	};

	let devices: [RAWINPUTDEVICE; 2] = [
		RAWINPUTDEVICE {
			usUsagePage: HID_USAGE_PAGE_GENERIC,
			usUsage: HID_USAGE_GENERIC_MOUSE,
			dwFlags: flags,
			hwndTarget: window_handle,
		},
		RAWINPUTDEVICE {
			usUsagePage: HID_USAGE_PAGE_GENERIC,
			usUsage: HID_USAGE_GENERIC_KEYBOARD,
			dwFlags: flags,
			hwndTarget: window_handle,
		},
	];

	register_raw_input_devices(&devices)
}

pub fn get_raw_input_data(handle: HRAWINPUT) -> Option<RAWINPUT> {
	let mut data: RAWINPUT = unsafe { mem::zeroed() };
	let mut data_size = size_of::<RAWINPUT>() as u32;
	let header_size = size_of::<RAWINPUTHEADER>() as u32;

	let status = unsafe {
		GetRawInputData(handle, RID_INPUT, &mut data as *mut _ as _, &mut data_size, header_size)
	};

	if status == u32::MAX || status == 0 {
		return None;
	}

	Some(data)
}

fn button_flags_to_element_state(
	button_flags: u32,
	down_flag: u32,
	up_flag: u32,
) -> Option<ElementState> {
	if has_flag(button_flags, down_flag) {
		Some(ElementState::Pressed)
	} else if has_flag(button_flags, up_flag) {
		Some(ElementState::Released)
	} else {
		None
	}
}

pub fn get_raw_mouse_button_state(button_flags: u32) -> [Option<ElementState>; 5] {
	[
		button_flags_to_element_state(button_flags, RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_1_UP),
		button_flags_to_element_state(button_flags, RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP),
		button_flags_to_element_state(button_flags, RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_3_UP),
		button_flags_to_element_state(button_flags, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP),
		button_flags_to_element_state(button_flags, RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP),
	]
}

pub fn get_keyboard_physical_key(keyboard: RAWKEYBOARD) -> Option<PhysicalKey> {
	let extension = {
		if has_flag(keyboard.Flags, RI_KEY_E0 as _) {
			0xe000
		} else if has_flag(keyboard.Flags, RI_KEY_E1 as _) {
			0xe100
		} else {
			0x0000
		}
	};
	let scancode = if keyboard.MakeCode == 0 {
		// In some cases (often with media keys) the device reports a scancode of 0 but a
		// valid virtual key. In these cases we obtain the scancode from the virtual key.
		unsafe { MapVirtualKeyW(keyboard.VKey as u32, MAPVK_VK_TO_VSC_EX) as u16 }
	} else {
		keyboard.MakeCode | extension
	};
	if scancode == 0xe11d || scancode == 0xe02a {
		return None;
	}
	let physical_key = if keyboard.VKey == VK_NUMLOCK {
		PhysicalKey::Code(KeyCode::NumLock)
	} else {
		scancode_to_physicalkey(scancode as u32)
	};
	if keyboard.VKey == VK_SHIFT {
		if let PhysicalKey::Code(
			KeyCode::NumpadDecimal
			| KeyCode::Numpad0
			| KeyCode::Numpad1
			| KeyCode::Numpad2
			| KeyCode::Numpad3
			| KeyCode::Numpad4
			| KeyCode::Numpad5
			| KeyCode::Numpad6
			| KeyCode::Numpad7
			| KeyCode::Numpad8
			| KeyCode::Numpad9,
		) = physical_key
		{
			return None;
		}
	}

	Some(physical_key)
}
