use std::{
	mem::MaybeUninit,
	sync::{Mutex, MutexGuard},
};

use evloop_core::{
	event::ElementState,
	keyboard::{LogicalKey, NamedKey},
};
use smol_str::SmolStr;
use windows_sys::Win32::{
	Foundation::{HWND, LPARAM, WPARAM},
	UI::{
		Input::KeyboardAndMouse::{
			GetAsyncKeyState, GetKeyState, GetKeyboardState, HKL, MAPVK_VK_TO_VSC_EX,
			MapVirtualKeyExW, VIRTUAL_KEY, VK_CAPITAL, VK_CONTROL, VK_F4, VK_LCONTROL, VK_LMENU,
			VK_LSHIFT, VK_MENU, VK_NUMLOCK, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SCROLL, VK_SHIFT,
		},
		WindowsAndMessaging::{
			MSG, PM_NOREMOVE, PeekMessageW, WM_CHAR, WM_DEADCHAR, WM_KEYDOWN, WM_KEYFIRST,
			WM_KEYLAST, WM_KEYUP, WM_KILLFOCUS, WM_SETFOCUS, WM_SYSCHAR, WM_SYSDEADCHAR,
			WM_SYSKEYDOWN, WM_SYSKEYUP,
		},
	},
};

use crate::{
	input::keyboard::{
		layout::Layout,
		layout_cache::{LAYOUT_CACHE, LayoutCache},
		modifiers::WindowsModifiers,
		partial_key_event::{
			ExScancode, PartialKeyEventInfo, PartialLogicalKey, PartialText,
			ex_scancode_from_lparam, get_location,
		},
		pending::{PendingEventQueue, PendingMessageToken},
		scancode::scancode_to_physicalkey,
	},
	keyboard::KeyEvent,
	sys::proc_result::ProcResult,
};

pub struct MessageAsKeyEvent {
	pub event: KeyEvent,
	pub is_synthetic: bool,
}

pub struct KeyEventBuilder {
	event_info: Mutex<Option<PartialKeyEventInfo>>,
	pending: PendingEventQueue<MessageAsKeyEvent>,
}

impl Default for KeyEventBuilder {
	fn default() -> Self {
		Self { event_info: Mutex::new(None), pending: Default::default() }
	}
}

impl KeyEventBuilder {
	pub(crate) fn process_message(
		&self,
		hwnd: HWND,
		msg_kind: u32,
		wparam: WPARAM,
		lparam: LPARAM,
		result: &mut ProcResult,
	) -> Vec<MessageAsKeyEvent> {
		enum MatchResult {
			Nothing,
			TokenToRemove(PendingMessageToken),
			MessagesToDispatch(Vec<MessageAsKeyEvent>),
		}

		let mut matcher = || -> MatchResult {
			match msg_kind {
				WM_SETFOCUS => {
					let kbd_state = get_async_kbd_state();
					let key_events = Self::synthesize_kbd_state(ElementState::Pressed, &kbd_state);
					MatchResult::MessagesToDispatch(self.pending.complete_multi(key_events))
				}
				WM_KILLFOCUS => {
					let kbd_state = get_kbd_state();
					let key_events = Self::synthesize_kbd_state(ElementState::Released, &kbd_state);
					MatchResult::MessagesToDispatch(self.pending.complete_multi(key_events))
				}
				WM_KEYDOWN | WM_SYSKEYDOWN => {
					if msg_kind == WM_SYSKEYDOWN && wparam as VIRTUAL_KEY == VK_F4 {
						return MatchResult::Nothing;
					}
					let pending_token = self.pending.add_pending();
					*result = ProcResult::Value(0);

					let next_msg = next_kbd_msg(hwnd);

					let mut layouts = LAYOUT_CACHE.lock().unwrap();
					let mut finished_event_info = Some(PartialKeyEventInfo::from_message(
						wparam,
						lparam,
						ElementState::Pressed,
						&mut layouts,
					));
					let mut event_info = self.event_info.lock().unwrap();
					*event_info = None;
					if let Some(next_msg) = next_msg {
						let next_msg_kind = next_msg.message;
						let next_belongs_to_this = !matches!(
							next_msg_kind,
							WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP
						);
						if next_belongs_to_this {
							*event_info = finished_event_info.take();
						} else {
							let (_, layout) = layouts.get_current_layout();
							let is_fake = {
								let curr_event = finished_event_info.as_ref().unwrap();
								is_current_fake(curr_event, next_msg, layout)
							};
							if is_fake {
								finished_event_info = None;
							}
						}
					}
					if let Some(event_info) = finished_event_info {
						let ev = event_info.finalize();
						return MatchResult::MessagesToDispatch(self.pending.complete_pending(
							pending_token,
							MessageAsKeyEvent { event: ev, is_synthetic: false },
						));
					}
					MatchResult::TokenToRemove(pending_token)
				}
				WM_DEADCHAR | WM_SYSDEADCHAR => {
					let pending_token = self.pending.add_pending();
					*result = ProcResult::Value(0);

					let event_info = self.event_info.lock().unwrap().take().unwrap();
					let ev = event_info.finalize();
					MatchResult::MessagesToDispatch(self.pending.complete_pending(
						pending_token,
						MessageAsKeyEvent { event: ev, is_synthetic: false },
					))
				}
				WM_CHAR | WM_SYSCHAR => {
					let mut event_info = self.event_info.lock().unwrap();
					if event_info.is_none() {
						tracing::trace!(
							"Received a CHAR message but no `event_info` was available. The \
                             message is probably IME, returning."
						);
						return MatchResult::Nothing;
					}
					let pending_token = self.pending.add_pending();
					*result = ProcResult::Value(0);
					let is_high_surrogate = (0xd800..=0xdbff).contains(&wparam);
					let is_low_surrogate = (0xdc00..=0xdfff).contains(&wparam);

					let is_utf16 = is_high_surrogate || is_low_surrogate;

					if is_utf16 {
						if let Some(ev_info) = event_info.as_mut() {
							ev_info.utf16parts.push(wparam as u16);
						}
					} else {
						// In this case, wparam holds a UTF-32 character.
						// Let's encode it as UTF-16 and append it to the end of `utf16parts`
						let utf16parts = match event_info.as_mut() {
							Some(ev_info) => &mut ev_info.utf16parts,
							None => {
								tracing::warn!(
									"The event_info was None when it was expected to be some"
								);
								return MatchResult::TokenToRemove(pending_token);
							}
						};
						let start_offset = utf16parts.len();
						let new_size = utf16parts.len() + 2;
						utf16parts.resize(new_size, 0);
						if let Some(ch) = char::from_u32(wparam as u32) {
							let encode_len = ch.encode_utf16(&mut utf16parts[start_offset..]).len();
							let new_size = start_offset + encode_len;
							utf16parts.resize(new_size, 0);
						}
					}
					// It's important that we unlock the mutex, and create the pending event token
					// before calling `next_msg`
					std::mem::drop(event_info);
					let next_msg = next_kbd_msg(hwnd);
					let more_char_coming = next_msg
						.map(|m| matches!(m.message, WM_CHAR | WM_SYSCHAR))
						.unwrap_or(false);
					if more_char_coming {
						MatchResult::TokenToRemove(pending_token)
					} else {
						let mut event_info = self.event_info.lock().unwrap();
						let mut event_info = match event_info.take() {
							Some(ev_info) => ev_info,
							None => {
								tracing::warn!(
									"The event_info was None when it was expected to be some"
								);
								return MatchResult::TokenToRemove(pending_token);
							}
						};
						let mut layouts = LAYOUT_CACHE.lock().unwrap();
						let kbd_state = get_kbd_state();
						let mod_state = WindowsModifiers::active_modifiers(&kbd_state);

						let (_, layout) = layouts.get_current_layout();
						let ctrl_on = if layout.has_alt_graph {
							let alt_on = mod_state.contains(WindowsModifiers::ALT);
							!alt_on && mod_state.contains(WindowsModifiers::CONTROL)
						} else {
							mod_state.contains(WindowsModifiers::CONTROL)
						};

						// If Ctrl is not pressed, just use the text with all
						// modifiers because that already consumed the dead key. Otherwise,
						// we would interpret the character incorrectly, missing the dead key.
						if !ctrl_on {
							event_info.text = PartialText::System(event_info.utf16parts.clone());
						} else {
							let mod_no_ctrl = mod_state.remove_only_ctrl();
							let num_lock_on = kbd_state[VK_NUMLOCK as usize] & 1 != 0;
							let vkey = event_info.vkey;
							let physical_key = &event_info.physical_key;
							let key = layout.get_key(mod_no_ctrl, num_lock_on, vkey, physical_key);
							event_info.text = PartialText::Text(key.to_text().map(SmolStr::new));
						}
						let ev = event_info.finalize();
						MatchResult::MessagesToDispatch(self.pending.complete_pending(
							pending_token,
							MessageAsKeyEvent { event: ev, is_synthetic: false },
						))
					}
				}
				WM_KEYUP | WM_SYSKEYUP => {
					let pending_token = self.pending.add_pending();
					*result = ProcResult::Value(0);

					let mut layouts = LAYOUT_CACHE.lock().unwrap();
					let event_info = PartialKeyEventInfo::from_message(
						wparam,
						lparam,
						ElementState::Released,
						&mut layouts,
					);
					drop(layouts);

					let next_msg = next_kbd_msg(hwnd);
					let mut valid_event_info = Some(event_info);
					if let Some(next_msg) = next_msg {
						let mut layouts = LAYOUT_CACHE.lock().unwrap();
						let (_, layout) = layouts.get_current_layout();
						let is_fake = {
							let event_info = valid_event_info.as_ref().unwrap();
							is_current_fake(event_info, next_msg, layout)
						};
						if is_fake {
							valid_event_info = None;
						}
					}
					if let Some(event_info) = valid_event_info {
						let event = event_info.finalize();
						return MatchResult::MessagesToDispatch(self.pending.complete_pending(
							pending_token,
							MessageAsKeyEvent { event, is_synthetic: false },
						));
					}
					MatchResult::TokenToRemove(pending_token)
				}
				_ => MatchResult::Nothing,
			}
		};
		let matcher_result = matcher();
		match matcher_result {
			MatchResult::TokenToRemove(t) => self.pending.remove_pending(t),
			MatchResult::MessagesToDispatch(m) => m,
			MatchResult::Nothing => Vec::new(),
		}
	}

	fn synthesize_kbd_state(
		key_state: ElementState,
		kbd_state: &[u8; 256],
	) -> Vec<MessageAsKeyEvent> {
		let mut key_events = Vec::new();

		let mut layouts = LAYOUT_CACHE.lock().unwrap();
		let (locale_id, _) = layouts.get_current_layout();

		macro_rules! is_key_pressed {
			($vk:expr) => {
				kbd_state[$vk as usize] & 0x80 != 0
			};
		}

		let caps_lock_on = kbd_state[VK_CAPITAL as usize] & 1 != 0;
		let num_lock_on = kbd_state[VK_NUMLOCK as usize] & 1 != 0;

		if is_key_pressed!(VK_CAPITAL) {
			let event = Self::create_synthetic(
				VK_CAPITAL,
				key_state,
				caps_lock_on,
				num_lock_on,
				locale_id as HKL,
				&mut layouts,
			);
			if let Some(event) = event {
				key_events.push(event);
			}
		}
		let do_non_modifier = |key_events: &mut Vec<_>, layouts: &mut _| {
			for vk in 0..256 {
				match vk {
					VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_SHIFT | VK_LSHIFT | VK_RSHIFT
					| VK_MENU | VK_LMENU | VK_RMENU | VK_CAPITAL => continue,
					_ => (),
				}
				if !is_key_pressed!(vk) {
					continue;
				}
				let event = Self::create_synthetic(
					vk,
					key_state,
					caps_lock_on,
					num_lock_on,
					locale_id as HKL,
					layouts,
				);
				if let Some(event) = event {
					key_events.push(event);
				}
			}
		};
		let do_modifier = |key_events: &mut Vec<_>, layouts: &mut _| {
			const CLEAR_MODIFIER_VKS: [VIRTUAL_KEY; 6] =
				[VK_LCONTROL, VK_LSHIFT, VK_LMENU, VK_RCONTROL, VK_RSHIFT, VK_RMENU];
			for vk in CLEAR_MODIFIER_VKS.iter() {
				if is_key_pressed!(*vk) {
					let event = Self::create_synthetic(
						*vk,
						key_state,
						caps_lock_on,
						num_lock_on,
						locale_id as HKL,
						layouts,
					);
					if let Some(event) = event {
						key_events.push(event);
					}
				}
			}
		};

		match key_state {
			ElementState::Pressed => {
				do_non_modifier(&mut key_events, &mut layouts);
				do_modifier(&mut key_events, &mut layouts);
			}
			ElementState::Released => {
				do_modifier(&mut key_events, &mut layouts);
				do_non_modifier(&mut key_events, &mut layouts);
			}
		}

		key_events
	}

	fn create_synthetic(
		vk: VIRTUAL_KEY,
		key_state: ElementState,
		caps_lock_on: bool,
		num_lock_on: bool,
		locale_id: HKL,
		layouts: &mut MutexGuard<'_, LayoutCache>,
	) -> Option<MessageAsKeyEvent> {
		let scancode = unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_VSC_EX, locale_id) };
		if scancode == 0 {
			return None;
		}
		let scancode = scancode as ExScancode;
		let physical_key = scancode_to_physicalkey(scancode as u32);
		let mods =
			if caps_lock_on { WindowsModifiers::CAPS_LOCK } else { WindowsModifiers::empty() };
		let layout = layouts.layouts.get(&(locale_id as u64)).unwrap();
		let logical_key = layout.get_key(mods, num_lock_on, vk, &physical_key);
		let key_without_modifiers =
			layout.get_key(WindowsModifiers::empty(), false, vk, &physical_key);
		let text = if key_state == ElementState::Pressed {
			logical_key.to_text().map(SmolStr::new)
		} else {
			None
		};
		let event_info = PartialKeyEventInfo {
			vkey: vk,
			logical_key: PartialLogicalKey::This(logical_key.clone()),
			key_without_modifiers,
			key_state,
			is_repeat: false,
			physical_key,
			location: get_location(scancode, locale_id),
			utf16parts: Vec::with_capacity(8),
			text: PartialText::Text(text.clone()),
		};

		let mut event = event_info.finalize();
		event.logical_key = logical_key;
		event.text_with_all_modifiers = text;
		Some(MessageAsKeyEvent { event, is_synthetic: true })
	}
}

pub(crate) fn get_kbd_state() -> [u8; 256] {
	unsafe {
		let mut kbd_state: MaybeUninit<[u8; 256]> = MaybeUninit::uninit();
		GetKeyboardState(kbd_state.as_mut_ptr() as *mut u8);
		kbd_state.assume_init()
	}
}

fn get_async_kbd_state() -> [u8; 256] {
	unsafe {
		let mut kbd_state: [u8; 256] = [0; 256];
		for (vk, state) in kbd_state.iter_mut().enumerate() {
			let vk = vk as VIRTUAL_KEY;
			let async_state = GetAsyncKeyState(vk as i32);
			let is_down = (async_state & (1 << 15)) != 0;
			*state = if is_down { 0x80 } else { 0 };

			if matches!(vk, VK_CAPITAL | VK_NUMLOCK | VK_SCROLL) {
				// Toggle states aren't reported by `GetAsyncKeyState`
				let toggle_state = GetKeyState(vk as i32);
				let is_active = (toggle_state & 1) != 0;
				*state |= u8::from(is_active);
			}
		}
		kbd_state
	}
}

fn is_current_fake(curr_info: &PartialKeyEventInfo, next_msg: MSG, layout: &Layout) -> bool {
	let curr_is_ctrl = matches!(
		curr_info.logical_key,
		PartialLogicalKey::This(LogicalKey::Named(NamedKey::Control))
	);
	if layout.has_alt_graph {
		let next_code = ex_scancode_from_lparam(next_msg.lParam);
		let next_is_altgr = next_code == 0xe038; // 0xE038 is right alt
		if curr_is_ctrl && next_is_altgr {
			return true;
		}
	}
	false
}

pub fn next_kbd_msg(hwnd: HWND) -> Option<MSG> {
	unsafe {
		let mut next_msg = MaybeUninit::uninit();
		let peek_retval =
			PeekMessageW(next_msg.as_mut_ptr(), hwnd, WM_KEYFIRST, WM_KEYLAST, PM_NOREMOVE);
		(peek_retval != 0).then(|| next_msg.assume_init())
	}
}
