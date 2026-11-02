use std::{ffi::OsString, os::windows::ffi::OsStringExt as _, sync::MutexGuard};

use evloop_core::{
	event::ElementState,
	keyboard::{KeyCode, KeyLocation, LogicalKey, NamedKey, PhysicalKey},
};
use smol_str::SmolStr;
use unicode_segmentation::UnicodeSegmentation as _;
use windows_sys::Win32::{
	Foundation::{LPARAM, WPARAM},
	UI::Input::KeyboardAndMouse::{
		HKL, MAPVK_VK_TO_VSC_EX, MAPVK_VSC_TO_VK_EX, MapVirtualKeyExW, VIRTUAL_KEY, VK_ABNT_C2,
		VK_ADD, VK_CLEAR, VK_DECIMAL, VK_DELETE, VK_DIVIDE, VK_DOWN, VK_END, VK_HOME, VK_INSERT,
		VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK,
		VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6,
		VK_NUMPAD7, VK_NUMPAD8, VK_NUMPAD9, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU,
		VK_RSHIFT, VK_RWIN, VK_SUBTRACT, VK_UP,
	},
};

use crate::{
	input::keyboard::{
		builder::get_kbd_state, layout_cache::LayoutCache, modifiers::WindowsModifiers,
		scancode::scancode_to_physicalkey,
	},
	keyboard::{KeyEvent, PhysicalRawKey},
};

pub type ExScancode = u16;

pub(crate) enum PartialText {
	// Unicode
	System(Vec<u16>),
	Text(Option<SmolStr>),
}

pub(crate) enum PartialLogicalKey {
	/// Use the text provided by the WM_CHAR messages and report that as a `Character` variant. If
	/// the text consists of multiple grapheme clusters (user-perceived characters) that means that
	/// dead key could not be combined with the second input, and in that case we should fall back
	/// to using what would have without a dead-key input.
	TextOr(LogicalKey),

	/// Use the value directly provided by this variant
	This(LogicalKey),
}

pub(crate) struct PartialKeyEventInfo {
	pub(crate) vkey: VIRTUAL_KEY,
	pub(crate) key_state: ElementState,
	pub(crate) is_repeat: bool,
	pub(crate) physical_key: PhysicalKey,
	pub(crate) location: KeyLocation,
	pub(crate) logical_key: PartialLogicalKey,

	pub(crate) key_without_modifiers: LogicalKey,

	/// The UTF-16 code units of the text that was produced by the keypress event.
	/// This take all modifiers into account. Including CTRL
	pub(crate) utf16parts: Vec<u16>,

	pub(crate) text: PartialText,
}

impl PartialKeyEventInfo {
	pub(crate) fn from_message(
		wparam: WPARAM,
		lparam: LPARAM,
		state: ElementState,
		layouts: &mut MutexGuard<'_, LayoutCache>,
	) -> Self {
		const NO_MODS: WindowsModifiers = WindowsModifiers::empty();

		let (_, layout) = layouts.get_current_layout();
		let lparam_struct = destructure_key_lparam(lparam);
		let vkey = wparam as VIRTUAL_KEY;
		let scancode = if lparam_struct.scancode == 0 {
			// In some cases (often with media keys) the device reports a scancode of 0 but a
			// valid virtual key. In these cases we obtain the scancode from the virtual key.
			unsafe { MapVirtualKeyExW(vkey as u32, MAPVK_VK_TO_VSC_EX, layout.hkl as HKL) as u16 }
		} else {
			new_ex_scancode(lparam_struct.scancode, lparam_struct.extended)
		};
		let physical_key = scancode_to_physicalkey(scancode as u32);
		let location = get_location(scancode, layout.hkl as HKL);

		let kbd_state = get_kbd_state();
		let mods = WindowsModifiers::active_modifiers(&kbd_state);
		let mods_without_ctrl = mods.remove_only_ctrl();
		let num_lock_on = kbd_state[VK_NUMLOCK as usize] & 1 != 0;

		// On Windows Ctrl+NumLock = Pause (and apparently Ctrl+Pause -> NumLock). In these cases
		// the KeyCode still stores the real key, so in the name of consistency across platforms, we
		// circumvent this mapping and force the key values to match the keycode.
		// For more on this, read the article by Raymond Chen, titled:
		// "Why does Ctrl+ScrollLock cancel dialogs?"
		// https://devblogs.microsoft.com/oldnewthing/20080211-00/?p=23503
		let code_as_key = if mods.contains(WindowsModifiers::CONTROL) {
			match physical_key {
				PhysicalKey::Code(KeyCode::NumLock) => Some(LogicalKey::Named(NamedKey::NumLock)),
				PhysicalKey::Code(KeyCode::Pause) => Some(LogicalKey::Named(NamedKey::Pause)),
				_ => None,
			}
		} else {
			None
		};

		let preliminary_logical_key =
			layout.get_key(mods_without_ctrl, num_lock_on, vkey, &physical_key);
		// FIXME(madsmtm): Is the `chars != " "` check desired here?
		let key_is_char =
			matches!(&preliminary_logical_key, LogicalKey::Character(chars) if chars != " ");
		let is_pressed = state == ElementState::Pressed;

		let logical_key = if let Some(key) = code_as_key.clone() {
			PartialLogicalKey::This(key)
		} else if is_pressed && key_is_char && !mods.contains(WindowsModifiers::CONTROL) {
			// In some cases we want to use the UNICHAR text for logical_key in order to allow
			// dead keys to have an effect on the character reported by `logical_key`.
			PartialLogicalKey::TextOr(preliminary_logical_key)
		} else {
			PartialLogicalKey::This(preliminary_logical_key)
		};
		let key_without_modifiers = if let Some(key) = code_as_key {
			key
		} else {
			match layout.get_key(NO_MODS, false, vkey, &physical_key) {
				// We convert dead keys into their character.
				// The reason for this is that `key_without_modifiers` is designed for key-bindings,
				// but the US International layout treats `'` (apostrophe) as a dead key and the
				// regular US layout treats it a character. In order for a single binding
				// configuration to work with both layouts, we forward each dead key as a character.
				LogicalKey::Dead(k) => {
					if let Some(ch) = k {
						// I'm avoiding the heap allocation. I don't want to talk about it :(
						let mut utf8 = [0; 4];
						let s = ch.encode_utf8(&mut utf8);
						LogicalKey::Character(SmolStr::new(s))
					} else {
						LogicalKey::Unidentified
					}
				}
				key => key,
			}
		};

		PartialKeyEventInfo {
			vkey,
			key_state: state,
			logical_key,
			key_without_modifiers,
			is_repeat: lparam_struct.is_repeat,
			physical_key,
			location,
			utf16parts: Vec::with_capacity(8),
			text: PartialText::System(Vec::new()),
		}
	}

	pub(crate) fn finalize(self) -> KeyEvent {
		let mut char_with_all_modifiers = None;
		if !self.utf16parts.is_empty() {
			let os_string = OsString::from_wide(&self.utf16parts);
			if let Ok(string) = os_string.into_string() {
				char_with_all_modifiers = Some(SmolStr::new(string));
			}
		}

		// The text without Ctrl
		let mut text = None;
		match self.text {
			PartialText::System(wide) => {
				if !wide.is_empty() {
					let os_string = OsString::from_wide(&wide);
					if let Ok(string) = os_string.into_string() {
						text = Some(SmolStr::new(string));
					}
				}
			}
			PartialText::Text(s) => {
				text = s.map(SmolStr::new);
			}
		}

		let logical_key = match self.logical_key {
			PartialLogicalKey::TextOr(fallback) => match text.as_ref() {
				Some(s) => {
					if s.grapheme_indices(true).count() > 1 {
						fallback
					} else {
						LogicalKey::Character(s.clone())
					}
				}
				None => LogicalKey::Unidentified,
			},
			PartialLogicalKey::This(v) => v,
		};

		KeyEvent {
			raw_key: PhysicalRawKey::new(self.vkey),
			physical_key: self.physical_key,
			logical_key,
			text,
			location: self.location,
			state: self.key_state,
			repeat: self.is_repeat,
			text_with_all_modifiers: char_with_all_modifiers,
			key_without_modifiers: self.key_without_modifiers,
		}
	}
}

#[derive(Debug, Copy, Clone)]
struct KeyLParam {
	pub scancode: u8,
	pub extended: bool,

	/// This is `previous_state XOR transition_state`. See the lParam for WM_KEYDOWN and WM_KEYUP
	/// for further details.
	pub is_repeat: bool,
}

fn destructure_key_lparam(lparam: LPARAM) -> KeyLParam {
	let previous_state = (lparam >> 30) & 0x01;
	let transition_state = (lparam >> 31) & 0x01;
	KeyLParam {
		scancode: ((lparam >> 16) & 0xff) as u8,
		extended: ((lparam >> 24) & 0x01) != 0,
		is_repeat: (previous_state ^ transition_state) != 0,
	}
}

#[inline]
fn new_ex_scancode(scancode: u8, extended: bool) -> ExScancode {
	(scancode as u16) | (if extended { 0xe000 } else { 0 })
}

#[inline]
pub(crate) fn ex_scancode_from_lparam(lparam: LPARAM) -> ExScancode {
	let lparam = destructure_key_lparam(lparam);
	new_ex_scancode(lparam.scancode, lparam.extended)
}

pub(crate) fn get_location(scancode: ExScancode, hkl: HKL) -> KeyLocation {
	const ABNT_C2: VIRTUAL_KEY = VK_ABNT_C2 as VIRTUAL_KEY;

	let extension = 0xe000;
	let extended = (scancode & extension) == extension;
	let vkey = unsafe { MapVirtualKeyExW(scancode as u32, MAPVK_VSC_TO_VK_EX, hkl) as VIRTUAL_KEY };

	// Use the native VKEY and the extended flag to cover most cases
	// This is taken from the `druid` GUI library, specifically
	// druid-shell/src/platform/windows/keyboard.rs
	match vkey {
		VK_LSHIFT | VK_LCONTROL | VK_LMENU | VK_LWIN => KeyLocation::Left,
		VK_RSHIFT | VK_RCONTROL | VK_RMENU | VK_RWIN => KeyLocation::Right,
		VK_RETURN if extended => KeyLocation::Numpad,
		VK_INSERT | VK_DELETE | VK_END | VK_DOWN | VK_NEXT | VK_LEFT | VK_CLEAR | VK_RIGHT
		| VK_HOME | VK_UP | VK_PRIOR => {
			if extended {
				KeyLocation::Standard
			} else {
				KeyLocation::Numpad
			}
		}
		VK_NUMPAD0 | VK_NUMPAD1 | VK_NUMPAD2 | VK_NUMPAD3 | VK_NUMPAD4 | VK_NUMPAD5
		| VK_NUMPAD6 | VK_NUMPAD7 | VK_NUMPAD8 | VK_NUMPAD9 | VK_DECIMAL | VK_DIVIDE
		| VK_MULTIPLY | VK_SUBTRACT | VK_ADD | ABNT_C2 => KeyLocation::Numpad,
		_ => KeyLocation::Standard,
	}
}
