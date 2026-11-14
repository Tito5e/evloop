use std::{mem, ptr};

use evloop_core::{
	event::ElementState::{Pressed, Released},
	pointer::{
		ButtonSource, FingerId, Force, PointerKind, PointerSource, TabletToolButton,
		TabletToolData, TabletToolKind, TabletToolOrientation, TabletToolTilt,
	},
};
use geometry_types::PhysicalPosition;
use windows_sys::Win32::{
	Foundation::POINT,
	Graphics::Gdi::ScreenToClient,
	UI::{
		Input::Pointer::{
			POINTER_FLAG_DOWN, POINTER_FLAG_PRIMARY, POINTER_FLAG_UP, POINTER_FLAG_UPDATE,
		},
		WindowsAndMessaging::{
			PEN_FLAG_BARREL, PEN_FLAG_ERASER, PEN_MASK_PRESSURE, PEN_MASK_ROTATION,
			PEN_MASK_TILT_X, PEN_MASK_TILT_Y, PT_PEN, PT_TOUCH,
		},
	},
};

use crate::{
	event::WindowEvent,
	event_loop::window_proc::message::WindowMessage,
	sys::{
		bits::has_flag,
		dynamic::{
			GET_POINTER_DEVICE_RECTS, GET_POINTER_FRAME_INFO_HISTORY, GET_POINTER_PEN_INFO,
			GET_POINTER_TOUCH_INFO, SKIP_POINTER_FRAME_MESSAGES,
		},
		msg_params::loword,
		proc_result::ProcResult,
	},
};

pub(crate) fn on_pointer_event(m: &WindowMessage<'_>, result: &mut ProcResult) {
	let WindowMessage { window, wparam, userdata, .. } = *m;

	if let (
		Some(GetPointerFrameInfoHistory),
		Some(SkipPointerFrameMessages),
		Some(GetPointerDeviceRects),
	) = (*GET_POINTER_FRAME_INFO_HISTORY, *SKIP_POINTER_FRAME_MESSAGES, *GET_POINTER_DEVICE_RECTS)
	{
		let pointer_id = loword(wparam as u32) as u32;
		let mut entries_count = 0u32;
		let mut pointers_count = 0u32;
		if unsafe {
			GetPointerFrameInfoHistory(
				pointer_id,
				&mut entries_count,
				&mut pointers_count,
				ptr::null_mut(),
			)
		} == false.into()
		{
			*result = ProcResult::Value(0);
			return;
		}

		let pointer_info_count = (entries_count * pointers_count) as usize;
		let mut pointer_infos = Vec::with_capacity(pointer_info_count);
		if unsafe {
			GetPointerFrameInfoHistory(
				pointer_id,
				&mut entries_count,
				&mut pointers_count,
				pointer_infos.as_mut_ptr(),
			)
		} == false.into()
		{
			*result = ProcResult::Value(0);
			return;
		}
		unsafe { pointer_infos.set_len(pointer_info_count) };

		// https://docs.microsoft.com/en-us/windows/desktop/api/winuser/nf-winuser-getpointerframeinfohistory
		// The information retrieved appears in reverse chronological order, with the most
		// recent entry in the first row of the returned array
		for pointer_info in pointer_infos.iter().rev() {
			let mut device_rect = mem::MaybeUninit::uninit();
			let mut display_rect = mem::MaybeUninit::uninit();

			if unsafe {
				GetPointerDeviceRects(
					pointer_info.sourceDevice,
					device_rect.as_mut_ptr(),
					display_rect.as_mut_ptr(),
				)
			} == false.into()
			{
				continue;
			}

			let device_rect = unsafe { device_rect.assume_init() };
			let display_rect = unsafe { display_rect.assume_init() };

			// For the most precise himetric to pixel conversion we calculate the ratio
			// between the resolution of the display device (pixel) and
			// the touch device (himetric).
			let himetric_to_pixel_ratio_x = (display_rect.right - display_rect.left) as f64
				/ (device_rect.right - device_rect.left) as f64;
			let himetric_to_pixel_ratio_y = (display_rect.bottom - display_rect.top) as f64
				/ (device_rect.bottom - device_rect.top) as f64;

			// ptHimetricLocation's origin is 0,0 even on multi-monitor setups.
			// On multi-monitor setups we need to translate the himetric location to the
			// rect of the display device it's attached to.
			let x = display_rect.left as f64
				+ pointer_info.ptHimetricLocation.x as f64 * himetric_to_pixel_ratio_x;
			let y = display_rect.top as f64
				+ pointer_info.ptHimetricLocation.y as f64 * himetric_to_pixel_ratio_y;

			let mut location = POINT { x: x.floor() as i32, y: y.floor() as i32 };

			if unsafe { ScreenToClient(window, &mut location) } == false.into() {
				continue;
			}

			let x = location.x as f64 + x.fract();
			let y = location.y as f64 + y.fract();
			let position = PhysicalPosition::new(x as i32, y as i32);

			let finger_id = FingerId::from_raw(pointer_info.pointerId as usize);
			let primary = has_flag(pointer_info.pointerFlags, POINTER_FLAG_PRIMARY);

			let is_down = has_flag(pointer_info.pointerFlags, POINTER_FLAG_DOWN);
			if is_down || has_flag(pointer_info.pointerFlags, POINTER_FLAG_UP) {
				let (kind, button) = match pointer_info.pointerType {
					PT_TOUCH => (
						PointerKind::Touch(finger_id),
						ButtonSource::Touch {
							finger_id,
							force: force_for_touch(pointer_info.pointerId),
						},
					),
					PT_PEN => {
						let kind = PointerKind::TabletTool(TabletToolKind::Pen);
						let (mut pen_flags, data) =
							tablet_tool_info_for_pen(pointer_info.pointerId);
						let old_pen_flags =
							userdata.last_tablet_down_button_state.replace(pen_flags);
						// For release, use a diff.
						if !is_down {
							pen_flags ^= old_pen_flags;
						};

						let button = ButtonSource::TabletTool {
							kind: TabletToolKind::Pen,
							button: pen_flags_to_button(pen_flags),
							data,
						};

						(kind, button)
					}
					_ => (PointerKind::Unknown, ButtonSource::Unknown(0)),
				};

				if is_down {
					userdata.send_window_event(
						window,
						WindowEvent::PointerEntered { device_id: None, primary, position, kind },
					);

					userdata.send_window_event(
						window,
						WindowEvent::PointerButton {
							device_id: None,
							primary,
							state: Pressed,
							position,
							button,
						},
					);
				} else {
					userdata.send_window_event(
						window,
						WindowEvent::PointerButton {
							device_id: None,
							primary,
							state: Released,
							position,
							button,
						},
					);
					userdata.send_window_event(
						window,
						WindowEvent::PointerLeft {
							device_id: None,
							primary,
							position: Some(position),
							kind,
						},
					);
				}
			} else if has_flag(pointer_info.pointerFlags, POINTER_FLAG_UPDATE) {
				let source = match pointer_info.pointerType {
					PT_TOUCH => PointerSource::Touch {
						finger_id,
						force: force_for_touch(pointer_info.pointerId),
					},
					PT_PEN => PointerSource::TabletTool {
						kind: TabletToolKind::Pen,
						data: tablet_tool_info_for_pen(pointer_info.pointerId).1,
					},
					_ => PointerSource::Unknown,
				};

				userdata.send_window_event(
					window,
					WindowEvent::PointerMoved { device_id: None, primary, position, source },
				);
			} else {
				continue;
			}
		}

		unsafe { SkipPointerFrameMessages(pointer_id) };
	}
	*result = ProcResult::Value(0);
}

fn normalize_pointer_pressure(pressure: u32) -> Option<Force> {
	match pressure {
		1..=1024 => Some(Force::Normalized(pressure as f64 / 1024.0)),
		_ => None,
	}
}

fn force_for_touch(pointer_id: u32) -> Option<Force> {
	let mut touch_info = mem::MaybeUninit::uninit();
	GET_POINTER_TOUCH_INFO.and_then(|GetPointerTouchInfo| {
		match unsafe { GetPointerTouchInfo(pointer_id, touch_info.as_mut_ptr()) } {
			0 => None,
			_ => normalize_pointer_pressure(unsafe { touch_info.assume_init().pressure }),
		}
	})
}

// Information is stored on the same thing, so we don't save anything by
// splitting the functions.
fn tablet_tool_info_for_pen(pointer_id: u32) -> (u32, TabletToolData) {
	let mut tool_data = TabletToolData::default();
	let mut tool_button = 0;

	let mut tablet_info = mem::MaybeUninit::uninit();
	GET_POINTER_PEN_INFO.map(|GetPointerPenInfo| {
		if unsafe { GetPointerPenInfo(pointer_id, tablet_info.as_mut_ptr()) } == 0 {
			return;
		}

		let pen_info = unsafe { tablet_info.assume_init() };
		if pen_info.penMask & PEN_MASK_PRESSURE != 0 {
			tool_data.force = normalize_pointer_pressure(pen_info.pressure);
		}
		if pen_info.penMask & (PEN_MASK_TILT_X | PEN_MASK_TILT_Y) != 0 {
			tool_data.tilt = Some(TabletToolOrientation::Tilt(TabletToolTilt {
				x: pen_info.tiltX as i8,
				y: pen_info.tiltY as i8,
			}));
		}
		if pen_info.penMask & PEN_MASK_ROTATION != 0 {
			tool_data.twist = Some(pen_info.rotation as u16);
		}

		tool_button = pen_info.penFlags;
	});

	(tool_button, tool_data)
}

// NOTE: According to firefox, the buttons while can be combined, in
// reality they are not.
fn pen_flags_to_button(flags: u32) -> TabletToolButton {
	if flags & PEN_FLAG_BARREL != 0 {
		TabletToolButton::Barrel
	} else if flags & PEN_FLAG_ERASER != 0 {
		// NOTE: this is likely not a correct thing to do, but route
		// it that way for now.
		TabletToolButton::Other(PEN_FLAG_ERASER as u16)
	} else {
		TabletToolButton::Contact
	}
}
