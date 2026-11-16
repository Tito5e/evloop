//! `android.view.KeyCharacterMap` access through JNI, used to turn key codes into characters.

use jni::{JavaVM, objects::Global, sys::jint};

use crate::glue::input::Keycode;

/// A unicode character, a dead-key accent, or nothing (non-printable key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyMapChar {
	None,
	Unicode(char),
	CombiningAccent(char),
}

jni::bind_java_type! {
	pub(crate) JKeyCharacterMap => "android.view.KeyCharacterMap",
	methods {
		fn get(key_code: jint, meta_state: jint) -> jint,
		static fn get_dead_char(accent_char: jint, base_char: jint) -> jint,
	}
}

jni::bind_java_type! {
	pub(crate) JInputDevice => "android.view.InputDevice",
	type_map {
		JKeyCharacterMap => "android.view.KeyCharacterMap",
	},
	methods {
		static fn get_device(id: jint) -> JInputDevice,
		fn get_key_character_map() -> JKeyCharacterMap,
	}
}

/// The key character map of one input device.
#[derive(Debug)]
pub struct KeyCharacterMap {
	vm: JavaVM,
	map: Global<JKeyCharacterMap<'static>>,
}

impl Clone for KeyCharacterMap {
	fn clone(&self) -> Self {
		self.vm
			.attach_current_thread(|env| -> jni::errors::Result<_> {
				Ok(Self { vm: self.vm.clone(), map: env.new_global_ref(&self.map)? })
			})
			.expect("failed to clone KeyCharacterMap global reference")
	}
}

impl KeyCharacterMap {
	/// Looks up the map of `device_id`. Returns an error for virtual devices (id 0) or unknown
	/// ids.
	pub(crate) fn for_device(vm: &JavaVM, device_id: i32) -> Result<Self, jni::errors::Error> {
		if device_id == 0 {
			return Err(jni::errors::Error::WrongObjectType);
		}
		vm.attach_current_thread(|env| -> jni::errors::Result<_> {
			let result = (|| {
				let device = JInputDevice::get_device(env, device_id)?;
				if device.is_null() {
					return Err(jni::errors::Error::WrongObjectType);
				}
				let map = device.get_key_character_map(env)?;
				Ok(Self { vm: vm.clone(), map: env.new_global_ref(map)? })
			})();
			clear_exception(env, result)
		})
	}

	/// The character produced by `key_code` with `meta_state` (`KeyCharacterMap.get`).
	pub fn get(
		&self,
		key_code: Keycode,
		meta_state: i32,
	) -> Result<KeyMapChar, jni::errors::Error> {
		const COMBINING_ACCENT: u32 = 0x8000_0000;

		let key_code: i32 = key_code.into();
		self.vm.attach_current_thread(|env| {
			let result = self.map.get(env, key_code, meta_state);
			let unicode = clear_exception(env, result)? as u32;
			Ok(if unicode == 0 {
				KeyMapChar::None
			} else if unicode & COMBINING_ACCENT != 0 {
				char::from_u32(unicode & !COMBINING_ACCENT)
					.map_or(KeyMapChar::None, KeyMapChar::CombiningAccent)
			} else {
				char::from_u32(unicode).map_or(KeyMapChar::None, KeyMapChar::Unicode)
			})
		})
	}

	/// The character produced by combining the dead-key `accent` with `base`
	/// (`KeyCharacterMap.getDeadChar`), e.g. `` ('`', 'e') `` → `'è'`.
	pub fn get_dead_char(
		&self,
		accent: char,
		base: char,
	) -> Result<Option<char>, jni::errors::Error> {
		self.vm.attach_current_thread(|env| {
			let result = JKeyCharacterMap::get_dead_char(env, accent as jint, base as jint);
			let unicode = clear_exception(env, result)? as u32;
			Ok(if unicode == 0 { None } else { char::from_u32(unicode) })
		})
	}
}

/// Calls made outside of a Java native method must not leave a pending exception behind.
fn clear_exception<T>(
	env: &mut jni::Env<'_>,
	result: jni::errors::Result<T>,
) -> jni::errors::Result<T> {
	if matches!(result, Err(jni::errors::Error::JavaException)) {
		let _ = env.exception_clear();
	}
	result
}
