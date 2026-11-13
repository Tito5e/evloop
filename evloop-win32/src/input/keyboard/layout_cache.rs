use std::{
	collections::{HashMap, HashSet, hash_map::Entry},
	ffi::OsString,
	os::windows::ffi::OsStringExt as _,
	sync::{LazyLock, Mutex},
};

use evloop_core::keyboard::{KeyCode, LogicalKey, ModifiersState, NamedKey, PhysicalKey};
use smol_str::SmolStr;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
	GetKeyState, GetKeyboardLayout, HKL, MAPVK_VK_TO_VSC_EX, MapVirtualKeyExW, ToUnicodeEx,
	VIRTUAL_KEY, VK_ADD, VK_CONTROL, VK_DECIMAL, VK_DIVIDE, VK_LWIN, VK_MENU, VK_MULTIPLY,
	VK_NUMPAD0, VK_NUMPAD1, VK_NUMPAD2, VK_NUMPAD3, VK_NUMPAD4, VK_NUMPAD5, VK_NUMPAD6, VK_NUMPAD7,
	VK_NUMPAD8, VK_NUMPAD9, VK_RMENU, VK_RWIN, VK_SEPARATOR, VK_SHIFT, VK_SUBTRACT,
};

use crate::input::keyboard::{
	key_mapping::{keycode_to_vkey, vkey_to_non_char_key},
	layout::Layout,
	modifiers::WindowsModifiers,
	scancode::scancode_to_physicalkey,
};

pub(crate) static LAYOUT_CACHE: LazyLock<Mutex<LayoutCache>> =
	LazyLock::new(|| Mutex::new(LayoutCache::default()));

fn key_pressed(vkey: VIRTUAL_KEY) -> bool {
	unsafe { (GetKeyState(vkey as i32) & (1 << 15)) == (1 << 15) }
}

const NUMPAD_VKEYS: [VIRTUAL_KEY; 16] = [
	VK_NUMPAD0,
	VK_NUMPAD1,
	VK_NUMPAD2,
	VK_NUMPAD3,
	VK_NUMPAD4,
	VK_NUMPAD5,
	VK_NUMPAD6,
	VK_NUMPAD7,
	VK_NUMPAD8,
	VK_NUMPAD9,
	VK_MULTIPLY,
	VK_ADD,
	VK_SEPARATOR,
	VK_SUBTRACT,
	VK_DECIMAL,
	VK_DIVIDE,
];

static NUMPAD_KEYCODES: LazyLock<HashSet<KeyCode>> = LazyLock::new(|| {
	let mut keycodes = HashSet::new();
	keycodes.insert(KeyCode::Numpad0);
	keycodes.insert(KeyCode::Numpad1);
	keycodes.insert(KeyCode::Numpad2);
	keycodes.insert(KeyCode::Numpad3);
	keycodes.insert(KeyCode::Numpad4);
	keycodes.insert(KeyCode::Numpad5);
	keycodes.insert(KeyCode::Numpad6);
	keycodes.insert(KeyCode::Numpad7);
	keycodes.insert(KeyCode::Numpad8);
	keycodes.insert(KeyCode::Numpad9);
	keycodes.insert(KeyCode::NumpadMultiply);
	keycodes.insert(KeyCode::NumpadAdd);
	keycodes.insert(KeyCode::NumpadComma);
	keycodes.insert(KeyCode::NumpadSubtract);
	keycodes.insert(KeyCode::NumpadDecimal);
	keycodes.insert(KeyCode::NumpadDivide);
	keycodes
});

fn is_numpad_specific(vk: VIRTUAL_KEY) -> bool {
	matches!(
		vk,
		VK_NUMPAD0
			| VK_NUMPAD1
			| VK_NUMPAD2
			| VK_NUMPAD3
			| VK_NUMPAD4
			| VK_NUMPAD5
			| VK_NUMPAD6
			| VK_NUMPAD7
			| VK_NUMPAD8
			| VK_NUMPAD9
			| VK_ADD
			| VK_SUBTRACT
			| VK_DIVIDE
			| VK_DECIMAL
			| VK_SEPARATOR
	)
}

#[derive(Default)]
pub(crate) struct LayoutCache {
	/// Maps locale identifiers (HKL) to layouts
	pub layouts: HashMap<u64, Layout>,
}

impl LayoutCache {
	/// Checks whether the current layout is already known and
	/// prepares the layout if it isn't known.
	/// The current layout is then returned.
	pub fn get_current_layout(&mut self) -> (u64, &Layout) {
		let locale_id = unsafe { GetKeyboardLayout(0) } as u64;
		match self.layouts.entry(locale_id) {
			Entry::Occupied(entry) => (locale_id, entry.into_mut()),
			Entry::Vacant(entry) => {
				let layout = Self::prepare_layout(locale_id);
				(locale_id, entry.insert(layout))
			}
		}
	}

	pub fn get_agnostic_mods(&mut self) -> ModifiersState {
		let (_, layout) = self.get_current_layout();
		let filter_out_altgr = layout.has_alt_graph && key_pressed(VK_RMENU);
		let mut mods = ModifiersState::empty();
		mods.set(ModifiersState::SHIFT, key_pressed(VK_SHIFT));
		mods.set(ModifiersState::CONTROL, key_pressed(VK_CONTROL) && !filter_out_altgr);
		mods.set(ModifiersState::ALT, key_pressed(VK_MENU) && !filter_out_altgr);
		mods.set(ModifiersState::META, key_pressed(VK_LWIN) || key_pressed(VK_RWIN));
		mods
	}

	fn prepare_layout(locale_id: u64) -> Layout {
		let mut layout = Layout {
			hkl: locale_id,
			numlock_on_keys: Default::default(),
			numlock_off_keys: Default::default(),
			keys: Default::default(),
			has_alt_graph: false,
		};

		let mut key_state = [0u8; 256];

		layout.numlock_off_keys.reserve(NUMPAD_KEYCODES.len());
		for vk in 0..256 {
			let scancode = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC_EX, locale_id as HKL) };
			if scancode == 0 {
				continue;
			}
			let keycode = match scancode_to_physicalkey(scancode) {
				PhysicalKey::Code(code) => code,
				// TODO: validate that we can skip on unidentified keys (probably never occurs?)
				_ => continue,
			};
			if !is_numpad_specific(vk as VIRTUAL_KEY) && NUMPAD_KEYCODES.contains(&keycode) {
				let map_vkey = keycode_to_vkey(keycode, locale_id);
				if map_vkey == 0 {
					continue;
				}
				let map_value = vkey_to_non_char_key(vk as VIRTUAL_KEY, locale_id, false);
				if matches!(map_value, LogicalKey::Unidentified) {
					continue;
				}
				layout.numlock_off_keys.insert(map_vkey, map_value);
			}
		}

		layout.numlock_on_keys.reserve(NUMPAD_VKEYS.len());
		for vk in NUMPAD_VKEYS.iter() {
			let vk = (*vk) as u32;
			let scancode = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC_EX, locale_id as HKL) };
			let unicode = Self::to_unicode_string(&key_state, vk, scancode, locale_id);
			if let ToUnicodeResult::Str(s) = unicode {
				layout
					.numlock_on_keys
					.insert(vk as VIRTUAL_KEY, LogicalKey::Character(SmolStr::new(s)));
			}
		}

		// Iterate through every combination of modifiers
		let mods_end = WindowsModifiers::FLAGS_END.bits();
		for mod_state in 0..mods_end {
			let mut keys_for_this_mod = HashMap::with_capacity(256);

			let mod_state = WindowsModifiers::from_bits_retain(mod_state);
			mod_state.apply_to_kbd_state(&mut key_state);

			// Virtual key values are in the domain [0, 255].
			// This is reinforced by the fact that the keyboard state array has 256
			// elements. This array is allowed to be indexed by virtual key values
			// giving the key state for the virtual key used for indexing.
			for vk in 0..256 {
				let scancode =
					unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC_EX, locale_id as HKL) };
				if scancode == 0 {
					continue;
				}

				let key_code = match scancode_to_physicalkey(scancode) {
					PhysicalKey::Code(code) => code,
					// TODO: validate that we can skip on unidentified keys (probably never occurs?)
					_ => continue,
				};
				// Let's try to get the key from just the scancode and vk
				// We don't necessarily know yet if AltGraph is present on this layout so we'll
				// assume it isn't. Then we'll do a second pass where we set the "AltRight" keys to
				// "AltGr" in case we find out that there's an AltGraph.
				let preliminary_key = vkey_to_non_char_key(vk as VIRTUAL_KEY, locale_id, false);
				match preliminary_key {
					LogicalKey::Unidentified => (),
					_ => {
						keys_for_this_mod.insert(key_code, preliminary_key);
						continue;
					}
				}

				let unicode = Self::to_unicode_string(&key_state, vk, scancode, locale_id);
				let key = match unicode {
					ToUnicodeResult::Str(str) => LogicalKey::Character(SmolStr::new(str)),
					ToUnicodeResult::Dead(dead_char) => LogicalKey::Dead(dead_char),
					ToUnicodeResult::None => {
						let has_alt = mod_state.contains(WindowsModifiers::ALT);
						let has_ctrl = mod_state.contains(WindowsModifiers::CONTROL);
						// HACK: `ToUnicodeEx` seems to fail getting the string for the numpad
						// divide key, so we handle that explicitly here
						if !has_alt && !has_ctrl && key_code == KeyCode::NumpadDivide {
							LogicalKey::Character(SmolStr::new("/"))
						} else {
							// Just use the unidentified key, we got earlier
							preliminary_key
						}
					}
				};

				// Check for alt graph.
				// The logic is that if a key pressed with no modifier produces
				// a different `Character` from when it's pressed with CTRL+ALT then the layout
				// has AltGr.
				let ctrl_alt: WindowsModifiers = WindowsModifiers::CONTROL | WindowsModifiers::ALT;
				let is_in_ctrl_alt = mod_state == ctrl_alt;
				if !layout.has_alt_graph && is_in_ctrl_alt {
					// Unwrapping here because if we are in the ctrl+alt modifier state
					// then the alt modifier state must have come before.
					let simple_keys = layout.keys.get(&WindowsModifiers::empty()).unwrap();
					if let Some(LogicalKey::Character(key_no_altgr)) = simple_keys.get(&key_code) {
						if let LogicalKey::Character(key) = &key {
							layout.has_alt_graph = key != key_no_altgr;
						}
					}
				}

				keys_for_this_mod.insert(key_code, key);
			}
			layout.keys.insert(mod_state, keys_for_this_mod);
		}

		// Second pass: replace right alt keys with AltGr if the layout has alt graph
		if layout.has_alt_graph {
			for mod_state in 0..mods_end {
				let mod_state = WindowsModifiers::from_bits_retain(mod_state);
				if let Some(keys) = layout.keys.get_mut(&mod_state) {
					if let Some(key) = keys.get_mut(&KeyCode::AltRight) {
						*key = LogicalKey::Named(NamedKey::AltGraph);
					}
				}
			}
		}

		layout
	}

	fn to_unicode_string(
		key_state: &[u8; 256],
		vkey: u32,
		scancode: u32,
		locale_id: u64,
	) -> ToUnicodeResult {
		unsafe {
			let mut label_wide = [0u16; 8];
			let mut wide_len = ToUnicodeEx(
				vkey,
				scancode,
				(&key_state[0]) as *const _,
				(&mut label_wide[0]) as *mut _,
				label_wide.len() as i32,
				0,
				locale_id as HKL,
			);
			if wide_len < 0 {
				// If it's dead, we run `ToUnicode` again to consume the dead-key
				wide_len = ToUnicodeEx(
					vkey,
					scancode,
					(&key_state[0]) as *const _,
					(&mut label_wide[0]) as *mut _,
					label_wide.len() as i32,
					0,
					locale_id as HKL,
				);
				if wide_len > 0 {
					let os_string = OsString::from_wide(&label_wide[0..wide_len as usize]);
					if let Ok(label_str) = os_string.into_string() {
						if let Some(ch) = label_str.chars().next() {
							return ToUnicodeResult::Dead(Some(ch));
						}
					}
				}
				return ToUnicodeResult::Dead(None);
			}
			if wide_len > 0 {
				let os_string = OsString::from_wide(&label_wide[0..wide_len as usize]);
				if let Ok(label_str) = os_string.into_string() {
					return ToUnicodeResult::Str(label_str);
				}
			}
		}
		ToUnicodeResult::None
	}
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ToUnicodeResult {
	Str(String),
	Dead(Option<char>),
	None,
}
