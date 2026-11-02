use evloop_core::event::DeviceId;
use windows_sys::Win32::Foundation::HANDLE;

use crate::input::raw_input::get_raw_input_device_name;

pub fn device_persistent_identifier(device_id: &DeviceId) -> Option<String> {
	let raw_id = device_id.into_raw();
	if raw_id != 0 { get_raw_input_device_name(raw_id as HANDLE) } else { None }
}
