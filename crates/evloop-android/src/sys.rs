//! FFI declarations for the vendored GameActivity / GameTextInput sources.
//!
//! Transcribed from `third_party/games-sdk/**/*.h` (GameActivity 4.4.0). Only what evloop uses is
//! declared. Keep in sync with the headers when updating the vendored sources.

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::ffi::{c_char, c_int, c_void};

use jni::sys::{JNIEnv, JavaVM, jobject};
use ndk_sys::{AAssetManager, ANativeWindow, ARect};

// ───────────────────────── GameActivity.h ─────────────────────────

pub type GameCommonInsetsType = u8;
pub const GAMECOMMON_INSETS_TYPE_CAPTION_BAR: GameCommonInsetsType = 0;
pub const GAMECOMMON_INSETS_TYPE_DISPLAY_CUTOUT: GameCommonInsetsType = 1;
pub const GAMECOMMON_INSETS_TYPE_IME: GameCommonInsetsType = 2;
pub const GAMECOMMON_INSETS_TYPE_MANDATORY_SYSTEM_GESTURES: GameCommonInsetsType = 3;
pub const GAMECOMMON_INSETS_TYPE_NAVIGATION_BARS: GameCommonInsetsType = 4;
pub const GAMECOMMON_INSETS_TYPE_STATUS_BARS: GameCommonInsetsType = 5;
pub const GAMECOMMON_INSETS_TYPE_SYSTEM_BARS: GameCommonInsetsType = 6;
pub const GAMECOMMON_INSETS_TYPE_SYSTEM_GESTURES: GameCommonInsetsType = 7;
pub const GAMECOMMON_INSETS_TYPE_TAPABLE_ELEMENT: GameCommonInsetsType = 8;
pub const GAMECOMMON_INSETS_TYPE_WATERFALL: GameCommonInsetsType = 9;
pub const GAMECOMMON_INSETS_TYPE_COUNT: usize = 10;

#[repr(C)]
pub struct GameActivity {
	pub callbacks: *mut GameActivityCallbacks,
	pub vm: *mut JavaVM,
	pub env: *mut JNIEnv,
	pub javaGameActivity: jobject,
	pub internalDataPath: *const c_char,
	pub externalDataPath: *const c_char,
	pub sdkVersion: i32,
	pub instance: *mut c_void,
	pub assetManager: *mut AAssetManager,
	pub obbPath: *const c_char,
}

pub type SaveInstanceStateRecallback =
	Option<unsafe extern "C" fn(bytes: *const c_char, len: c_int, context: *mut c_void)>;

#[repr(C)]
pub struct GameActivityCallbacks {
	pub onStart: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onResume: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onSaveInstanceState: Option<
		unsafe extern "C" fn(
			activity: *mut GameActivity,
			recallback: SaveInstanceStateRecallback,
			context: *mut c_void,
		),
	>,
	pub onPause: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onStop: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onDestroy: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onWindowFocusChanged:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, has_focus: bool)>,
	pub onNativeWindowCreated:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, window: *mut ANativeWindow)>,
	pub onNativeWindowResized: Option<
		unsafe extern "C" fn(
			activity: *mut GameActivity,
			window: *mut ANativeWindow,
			new_width: i32,
			new_height: i32,
		),
	>,
	pub onNativeWindowRedrawNeeded:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, window: *mut ANativeWindow)>,
	pub onNativeWindowDestroyed:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, window: *mut ANativeWindow)>,
	pub onConfigurationChanged: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onTrimMemory: Option<unsafe extern "C" fn(activity: *mut GameActivity, level: c_int)>,
	pub onTouchEvent: Option<
		unsafe extern "C" fn(
			activity: *mut GameActivity,
			event: *const GameActivityMotionEvent,
		) -> bool,
	>,
	pub onKeyDown: Option<
		unsafe extern "C" fn(
			activity: *mut GameActivity,
			event: *const GameActivityKeyEvent,
		) -> bool,
	>,
	pub onKeyUp: Option<
		unsafe extern "C" fn(
			activity: *mut GameActivity,
			event: *const GameActivityKeyEvent,
		) -> bool,
	>,
	pub onTextInputEvent:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, state: *const GameTextInputState)>,
	pub onWindowInsetsChanged: Option<unsafe extern "C" fn(activity: *mut GameActivity)>,
	pub onContentRectChanged:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, rect: *const ARect)>,
	pub onSoftwareKeyboardVisibilityChanged:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, visible: bool)>,
	pub onEditorAction:
		Option<unsafe extern "C" fn(activity: *mut GameActivity, action: c_int) -> bool>,
}

pub const GAMEACTIVITY_SHOW_SOFT_INPUT_IMPLICIT: u32 = 0x0001;
pub const GAMEACTIVITY_SHOW_SOFT_INPUT_FORCED: u32 = 0x0002;
pub const GAMEACTIVITY_HIDE_SOFT_INPUT_IMPLICIT_ONLY: u32 = 0x0001;
pub const GAMEACTIVITY_HIDE_SOFT_INPUT_NOT_ALWAYS: u32 = 0x0002;

unsafe extern "C" {
	pub fn GameActivity_finish(activity: *mut GameActivity);
	pub fn GameActivity_setWindowFlags(
		activity: *mut GameActivity,
		add_flags: u32,
		remove_flags: u32,
	);
	pub fn GameActivity_showSoftInput(activity: *mut GameActivity, flags: u32);
	pub fn GameActivity_hideSoftInput(activity: *mut GameActivity, flags: u32);
	pub fn GameActivity_restartInput(activity: *mut GameActivity);
	pub fn GameActivity_setTextInputState(
		activity: *mut GameActivity,
		state: *const GameTextInputState,
	);
	pub fn GameActivity_getTextInputState(
		activity: *mut GameActivity,
		callback: GameTextInputGetStateCallback,
		context: *mut c_void,
	);
	pub fn GameActivity_getWindowInsets(
		activity: *mut GameActivity,
		type_: GameCommonInsetsType,
		insets: *mut ARect,
	);
	pub fn GameActivity_isSoftwareKeyboardVisible(activity: *mut GameActivity) -> bool;
	pub fn GameActivity_setImeEditorInfo(
		activity: *mut GameActivity,
		input_type: u32,
		action_id: u32,
		ime_options: u32,
	);
}

// ───────────────────────── GameActivityEvents.h ─────────────────────────

pub const GAME_ACTIVITY_POINTER_INFO_AXIS_COUNT: usize = 48;
pub const GAMEACTIVITY_MAX_NUM_POINTERS_IN_MOTION_EVENT: usize = 8;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GameActivityPointerAxes {
	pub id: i32,
	pub toolType: i32,
	pub axisValues: [f32; GAME_ACTIVITY_POINTER_INFO_AXIS_COUNT],
	pub rawX: f32,
	pub rawY: f32,
}

#[repr(C)]
pub struct GameActivityMotionEvent {
	pub deviceId: i32,
	pub source: i32,
	pub action: i32,
	pub eventTime: i64,
	pub downTime: i64,
	pub flags: i32,
	pub metaState: i32,
	pub actionButton: i32,
	pub buttonState: i32,
	pub classification: i32,
	pub edgeFlags: i32,
	pub pointerCount: u32,
	pub pointers: [GameActivityPointerAxes; GAMEACTIVITY_MAX_NUM_POINTERS_IN_MOTION_EVENT],
	pub historySize: c_int,
	pub historicalEventTimesMillis: *mut i64,
	pub historicalEventTimesNanos: *mut i64,
	pub historicalAxisValues: *mut f32,
	pub precisionX: f32,
	pub precisionY: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GameActivityKeyEvent {
	pub deviceId: i32,
	pub source: i32,
	pub action: i32,
	pub eventTime: i64,
	pub downTime: i64,
	pub flags: i32,
	pub metaState: i32,
	pub modifiers: i32,
	pub repeatCount: i32,
	pub keyCode: i32,
	pub scanCode: i32,
}

unsafe extern "C" {
	pub fn GameActivityPointerAxes_enableAxis(axis: i32);
	pub fn GameActivityPointerAxes_disableAxis(axis: i32);
	/// Frees the history arrays of a motion event received in `onTouchEvent`. The callee of
	/// `onTouchEvent` owns them.
	pub fn GameActivityMotionEvent_destroy(event: *mut GameActivityMotionEvent);
}

// ───────────────────────── gametextinput.h ─────────────────────────

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameTextInputSpan {
	pub start: i32,
	pub end: i32,
}

#[repr(C)]
pub struct GameTextInputState {
	/// Modified UTF-8 (Java's encoding), not NUL-terminated.
	pub text_UTF8: *const c_char,
	pub text_length: i32,
	/// UTF-16 code unit offsets.
	pub selection: GameTextInputSpan,
	/// UTF-16 code unit offsets, `-1` when there is no composing region.
	pub composingRegion: GameTextInputSpan,
}

pub type GameTextInputGetStateCallback =
	Option<unsafe extern "C" fn(context: *mut c_void, state: *const GameTextInputState)>;
