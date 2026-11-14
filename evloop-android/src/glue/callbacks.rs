//! GameActivity callbacks. All of them run on the Java main thread.

use std::{
	ffi::{c_int, c_void},
	ptr::NonNull,
};

use ndk::native_window::NativeWindow;
use ndk_sys::{ANativeWindow, ARect};

use super::{
	MainEvent, RawTextInputState, abort_on_panic,
	input::{self, InputEvent, KeyEvent, MotionEvent},
	release_instance, shared_from,
};
use crate::sys::{
	GameActivity, GameActivityKeyEvent, GameActivityMotionEvent, GameActivityMotionEvent_destroy,
	GameTextInputState, SaveInstanceStateRecallback,
};

/// `ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN`: the UI was hidden, not a memory warning.
const TRIM_MEMORY_UI_HIDDEN: c_int = 20;

pub(super) unsafe fn install(activity: *mut GameActivity) {
	let callbacks = unsafe { &mut *(*activity).callbacks };
	callbacks.onStart = Some(on_start);
	callbacks.onResume = Some(on_resume);
	callbacks.onSaveInstanceState = Some(on_save_instance_state);
	callbacks.onPause = Some(on_pause);
	callbacks.onStop = Some(on_stop);
	callbacks.onDestroy = Some(on_destroy);
	callbacks.onWindowFocusChanged = Some(on_window_focus_changed);
	callbacks.onNativeWindowCreated = Some(on_native_window_created);
	callbacks.onNativeWindowResized = Some(on_native_window_resized);
	callbacks.onNativeWindowRedrawNeeded = Some(on_native_window_redraw_needed);
	callbacks.onNativeWindowDestroyed = Some(on_native_window_destroyed);
	callbacks.onConfigurationChanged = Some(on_configuration_changed);
	callbacks.onTrimMemory = Some(on_trim_memory);
	callbacks.onTouchEvent = Some(on_touch_event);
	callbacks.onKeyDown = Some(on_key);
	callbacks.onKeyUp = Some(on_key);
	callbacks.onTextInputEvent = Some(on_text_input_event);
	callbacks.onWindowInsetsChanged = Some(on_window_insets_changed);
	callbacks.onContentRectChanged = Some(on_content_rect_changed);
	callbacks.onSoftwareKeyboardVisibilityChanged = Some(on_software_keyboard_visibility_changed);
	callbacks.onEditorAction = Some(on_editor_action);
}

/// Runs `f` with the activity's `Shared`, catching panics.
fn with_shared<R: Default>(
	activity: *mut GameActivity,
	f: impl FnOnce(&'static super::Shared) -> R,
) -> R {
	abort_on_panic(|| match unsafe { shared_from(activity) } {
		Some(shared) => f(shared),
		None => R::default(),
	})
}

fn push(activity: *mut GameActivity, event: MainEvent) {
	with_shared(activity, |shared| shared.push_main_event(event));
}

unsafe extern "C" fn on_start(activity: *mut GameActivity) {
	push(activity, MainEvent::Start);
}

unsafe extern "C" fn on_resume(activity: *mut GameActivity) {
	push(activity, MainEvent::Resume);
}

unsafe extern "C" fn on_pause(activity: *mut GameActivity) {
	push(activity, MainEvent::Pause);
}

unsafe extern "C" fn on_stop(activity: *mut GameActivity) {
	push(activity, MainEvent::Stop);
}

unsafe extern "C" fn on_save_instance_state(
	_activity: *mut GameActivity,
	_recallback: SaveInstanceStateRecallback,
	_context: *mut c_void,
) {
	// Saved state is not supported (nothing is written back).
}

unsafe extern "C" fn on_destroy(activity: *mut GameActivity) {
	with_shared(activity, |shared| {
		shared.push_main_event(MainEvent::Destroy);
		// `android_main` must return before the activity (and its window) goes away.
		shared.wait_while(|_| true);
	});
	abort_on_panic(|| unsafe { release_instance(activity) });
}

unsafe extern "C" fn on_window_focus_changed(activity: *mut GameActivity, has_focus: bool) {
	push(activity, if has_focus { MainEvent::GainedFocus } else { MainEvent::LostFocus });
}

unsafe extern "C" fn on_native_window_created(
	activity: *mut GameActivity,
	window: *mut ANativeWindow,
) {
	with_shared(activity, |shared| {
		let window = NonNull::new(window).map(|w| unsafe { NativeWindow::clone_from_ptr(w) });
		shared.lock().native_window = window;
		shared.push_main_event(MainEvent::InitWindow);
	});
}

unsafe extern "C" fn on_native_window_resized(
	activity: *mut GameActivity,
	_window: *mut ANativeWindow,
	_width: i32,
	_height: i32,
) {
	push(activity, MainEvent::WindowResized);
}

unsafe extern "C" fn on_native_window_redraw_needed(
	activity: *mut GameActivity,
	_window: *mut ANativeWindow,
) {
	push(activity, MainEvent::RedrawNeeded);
}

unsafe extern "C" fn on_native_window_destroyed(
	activity: *mut GameActivity,
	_window: *mut ANativeWindow,
) {
	with_shared(activity, |shared| {
		{
			let mut state = shared.lock();
			if state.native_window.is_none() {
				return;
			}
			state.terminate_window_pending = true;
			state.main_events.push_back(MainEvent::TerminateWindow);
		}
		shared.wake();
		// The surface must stay valid until the application has stopped using it.
		shared.wait_while(|state| state.terminate_window_pending);
		shared.lock().native_window = None;
	});
}

unsafe extern "C" fn on_configuration_changed(activity: *mut GameActivity) {
	with_shared(activity, |shared| {
		shared.refresh_config();
		shared.push_main_event(MainEvent::ConfigChanged);
	});
}

unsafe extern "C" fn on_trim_memory(activity: *mut GameActivity, level: c_int) {
	if level != TRIM_MEMORY_UI_HIDDEN {
		push(activity, MainEvent::LowMemory);
	}
}

unsafe extern "C" fn on_touch_event(
	activity: *mut GameActivity,
	event: *const GameActivityMotionEvent,
) -> bool {
	with_shared(activity, |shared| {
		let raw = unsafe { &*event };
		let handled = input::is_pointer_source(raw.source);
		if handled {
			let event = unsafe { MotionEvent::from_raw(raw) };
			shared.lock().input.push_back(InputEvent::Motion(event));
			shared.wake();
		}
		// The callee owns the history arrays, whether or not the event is handled.
		unsafe { GameActivityMotionEvent_destroy(event.cast_mut()) };
		handled
	})
}

unsafe extern "C" fn on_key(
	activity: *mut GameActivity,
	event: *const GameActivityKeyEvent,
) -> bool {
	with_shared(activity, |shared| {
		let raw = unsafe { *event };
		if input::is_system_key(raw.keyCode) {
			return false;
		}
		shared.lock().input.push_back(InputEvent::Key(KeyEvent::from_raw(raw)));
		shared.wake();
		true
	})
}

unsafe extern "C" fn on_text_input_event(
	activity: *mut GameActivity,
	state: *const GameTextInputState,
) {
	with_shared(activity, |shared| {
		let Some(text_input) = (unsafe { decode_text_input_state(state) }) else {
			return;
		};
		shared.lock().text_input = text_input;
		shared.push_main_event(MainEvent::TextInputChanged);
	});
}

unsafe extern "C" fn on_window_insets_changed(activity: *mut GameActivity) {
	with_shared(activity, |shared| {
		shared.refresh_insets(activity);
		shared.push_main_event(MainEvent::InsetsChanged);
	});
}

unsafe extern "C" fn on_content_rect_changed(activity: *mut GameActivity, rect: *const ARect) {
	with_shared(activity, |shared| {
		if let Some(rect) = unsafe { rect.as_ref() } {
			shared.lock().content_rect = *rect;
		}
		shared.push_main_event(MainEvent::ContentRectChanged);
	});
}

unsafe extern "C" fn on_software_keyboard_visibility_changed(
	activity: *mut GameActivity,
	visible: bool,
) {
	with_shared(activity, |shared| {
		shared.lock().soft_keyboard_visible = visible;
		shared.push_main_event(MainEvent::SoftKeyboardVisibilityChanged);
	});
}

unsafe extern "C" fn on_editor_action(activity: *mut GameActivity, action: c_int) -> bool {
	push(activity, MainEvent::EditorAction(action));
	// Let GameActivity apply the default behavior of the action as well.
	false
}

/// Decodes GameTextInput's state. The text is Java's modified UTF-8.
pub(super) unsafe fn decode_text_input_state(
	state: *const GameTextInputState,
) -> Option<RawTextInputState> {
	let state = unsafe { state.as_ref()? };
	let bytes = if state.text_UTF8.is_null() || state.text_length <= 0 {
		&[][..]
	} else {
		unsafe {
			std::slice::from_raw_parts(state.text_UTF8.cast::<u8>(), state.text_length as usize)
		}
	};
	let text = decode_modified_utf8(bytes)?;
	let compose = (state.composingRegion.start >= 0 && state.composingRegion.end >= 0)
		.then_some((state.composingRegion.start, state.composingRegion.end));
	Some(RawTextInputState {
		text,
		selection: (state.selection.start, state.selection.end),
		compose,
	})
}

/// Decodes Java's modified UTF-8 (CESU-8 surrogate pairs, `C0 80` for NUL).
pub(crate) fn decode_modified_utf8(bytes: &[u8]) -> Option<String> {
	if let Ok(s) = std::str::from_utf8(bytes) {
		if !s.contains('\u{0}') {
			return Some(s.to_owned());
		}
	}

	let mut units: Vec<u16> = Vec::with_capacity(bytes.len());
	let mut i = 0;
	while i < bytes.len() {
		let b = bytes[i];
		let (unit, len) = if b < 0x80 {
			(b as u16, 1)
		} else if b & 0xe0 == 0xc0 && i + 1 < bytes.len() {
			((((b & 0x1f) as u16) << 6) | (bytes[i + 1] & 0x3f) as u16, 2)
		} else if b & 0xf0 == 0xe0 && i + 2 < bytes.len() {
			(
				(((b & 0x0f) as u16) << 12)
					| (((bytes[i + 1] & 0x3f) as u16) << 6)
					| (bytes[i + 2] & 0x3f) as u16,
				3,
			)
		} else {
			return None;
		};
		units.push(unit);
		i += len;
	}
	String::from_utf16(&units).ok()
}

/// Encodes a string as Java's modified UTF-8.
pub(crate) fn encode_modified_utf8(text: &str) -> Vec<u8> {
	let mut out = Vec::with_capacity(text.len());
	for unit in text.encode_utf16() {
		match unit {
			0x0001..=0x007f => out.push(unit as u8),
			0x0000 | 0x0080..=0x07ff => {
				out.push(0xc0 | (unit >> 6) as u8);
				out.push(0x80 | (unit & 0x3f) as u8);
			}
			_ => {
				out.push(0xe0 | (unit >> 12) as u8);
				out.push(0x80 | ((unit >> 6) & 0x3f) as u8);
				out.push(0x80 | (unit & 0x3f) as u8);
			}
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn modified_utf8_round_trip() {
		for s in ["", "abc", "日本語", "emoji 😀 and \u{0} nul", "é"] {
			assert_eq!(decode_modified_utf8(&encode_modified_utf8(s)).as_deref(), Some(s));
		}
	}

	#[test]
	fn supplementary_characters_use_surrogate_pairs() {
		// U+1F600 is encoded as two 3-byte surrogates in modified UTF-8.
		assert_eq!(encode_modified_utf8("😀").len(), 6);
	}
}
