use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
	VK_CAPITAL, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU, VK_RCONTROL, VK_RMENU,
	VK_RSHIFT, VK_SHIFT,
};

bitflags::bitflags! {
	#[derive(Clone, Copy, PartialEq, Eq, Hash)]
	pub struct WindowsModifiers : u8 {
		const SHIFT = 1 << 0;
		const CONTROL = 1 << 1;
		const ALT = 1 << 2;
		const CAPS_LOCK = 1 << 3;
		const FLAGS_END = 1 << 4;
	}
}

impl WindowsModifiers {
	pub fn active_modifiers(key_state: &[u8; 256]) -> WindowsModifiers {
		let shift = key_state[VK_SHIFT as usize] & 0x80 != 0;
		let lshift = key_state[VK_LSHIFT as usize] & 0x80 != 0;
		let rshift = key_state[VK_RSHIFT as usize] & 0x80 != 0;

		let control = key_state[VK_CONTROL as usize] & 0x80 != 0;
		let lcontrol = key_state[VK_LCONTROL as usize] & 0x80 != 0;
		let rcontrol = key_state[VK_RCONTROL as usize] & 0x80 != 0;

		let alt = key_state[VK_MENU as usize] & 0x80 != 0;
		let lalt = key_state[VK_LMENU as usize] & 0x80 != 0;
		let ralt = key_state[VK_RMENU as usize] & 0x80 != 0;

		let caps = key_state[VK_CAPITAL as usize] & 0x01 != 0;

		let mut result = WindowsModifiers::empty();
		if shift || lshift || rshift {
			result.insert(WindowsModifiers::SHIFT);
		}
		if control || lcontrol || rcontrol {
			result.insert(WindowsModifiers::CONTROL);
		}
		if alt || lalt || ralt {
			result.insert(WindowsModifiers::ALT);
		}
		if caps {
			result.insert(WindowsModifiers::CAPS_LOCK);
		}

		result
	}

	pub fn apply_to_kbd_state(self, key_state: &mut [u8; 256]) {
		if self.intersects(Self::SHIFT) {
			key_state[VK_SHIFT as usize] |= 0x80;
		} else {
			key_state[VK_SHIFT as usize] &= !0x80;
			key_state[VK_LSHIFT as usize] &= !0x80;
			key_state[VK_RSHIFT as usize] &= !0x80;
		}
		if self.intersects(Self::CONTROL) {
			key_state[VK_CONTROL as usize] |= 0x80;
		} else {
			key_state[VK_CONTROL as usize] &= !0x80;
			key_state[VK_LCONTROL as usize] &= !0x80;
			key_state[VK_RCONTROL as usize] &= !0x80;
		}
		if self.intersects(Self::ALT) {
			key_state[VK_MENU as usize] |= 0x80;
		} else {
			key_state[VK_MENU as usize] &= !0x80;
			key_state[VK_LMENU as usize] &= !0x80;
			key_state[VK_RMENU as usize] &= !0x80;
		}
		if self.intersects(Self::CAPS_LOCK) {
			key_state[VK_CAPITAL as usize] |= 0x01;
		} else {
			key_state[VK_CAPITAL as usize] &= !0x01;
		}
	}

	/// Removes the control modifier if the alt modifier is not present.
	/// This is useful because on Windows: (Control + Alt) == AltGr
	/// but we don't want to interfere with the AltGr state.
	pub fn remove_only_ctrl(mut self) -> WindowsModifiers {
		if !self.contains(WindowsModifiers::ALT) {
			self.remove(WindowsModifiers::CONTROL);
		}
		self
	}
}
