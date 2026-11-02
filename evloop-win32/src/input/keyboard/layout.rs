use std::collections::HashMap;

use evloop_core::keyboard::{KeyCode, LogicalKey, PhysicalKey};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_MENU};

use crate::input::keyboard::{key_mapping::vkey_to_non_char_key, modifiers::WindowsModifiers};

pub(crate) struct Layout {
	pub hkl: u64,

	/// Maps numpad keys from Windows virtual key to a `Key`.
	///
	/// This is useful because some numpad keys generate different characters based on the locale.
	/// For example `VK_DECIMAL` is sometimes "." and sometimes ",". Note: numpad-specific virtual
	/// keys are only produced by Windows when the NumLock is active.
	///
	/// Making this field separate from the `keys` field saves having to add NumLock as a modifier
	/// to `WindowsModifiers`, which would double the number of items in keys.
	pub numlock_on_keys: HashMap<VIRTUAL_KEY, LogicalKey>,
	/// Like `numlock_on_keys` but this will map to the key that would be produced if numlock was
	/// off. The keys of this map are identical to the keys of `numlock_on_keys`.
	pub numlock_off_keys: HashMap<VIRTUAL_KEY, LogicalKey>,

	/// Maps a modifier state to group of key strings
	/// We're not using `ModifiersState` here because that object cannot express caps lock,
	/// but we need to handle caps lock too.
	///
	/// This map shouldn't need to exist.
	/// However currently this seems to be the only good way
	/// of getting the label for the pressed key. Note that calling `ToUnicode`
	/// just when the key is pressed/released would be enough if `ToUnicode` wouldn't
	/// change the keyboard state (it clears the dead key). There is a flag to prevent
	/// changing the state, but that flag requires Windows 10, version 1607 or newer)
	pub keys: HashMap<WindowsModifiers, HashMap<KeyCode, LogicalKey>>,
	pub has_alt_graph: bool,
}

impl Layout {
	pub fn get_key(
		&self,
		mods: WindowsModifiers,
		num_lock_on: bool,
		vkey: VIRTUAL_KEY,
		physical_key: &PhysicalKey,
	) -> LogicalKey {
		let unknown_alt = vkey == VK_MENU;
		if !unknown_alt {
			// Here we try using the virtual key directly but if the virtual key doesn't distinguish
			// between left and right alt, we can't report AltGr. Therefore, we only do this if the
			// key is not the "unknown alt" key.
			//
			// The reason for using the virtual key directly is that `MapVirtualKeyExW` (used when
			// building the keys map) sometimes maps virtual keys to odd scancodes that don't match
			// the scancode coming from the KEYDOWN message for the same key. For example: `VK_LEFT`
			// is mapped to `0x004B`, but the scancode for the left arrow is `0xE04B`.
			let key_from_vkey = vkey_to_non_char_key(vkey, self.hkl, self.has_alt_graph);

			if !matches!(key_from_vkey, LogicalKey::Unidentified) {
				return key_from_vkey;
			}
		}
		if num_lock_on {
			if let Some(key) = self.numlock_on_keys.get(&vkey) {
				return key.clone();
			}
		} else if let Some(key) = self.numlock_off_keys.get(&vkey) {
			return key.clone();
		}
		if let PhysicalKey::Code(code) = physical_key {
			if let Some(keys) = self.keys.get(&mods) {
				if let Some(key) = keys.get(code) {
					return key.clone();
				}
			}
		}
		LogicalKey::Unidentified
	}
}
