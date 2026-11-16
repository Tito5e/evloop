//! The bridge between GameActivity (Java main thread) and the application thread.
//!
//! This replaces both `android_native_app_glue` and the `android-activity` crate:
//!
//! - GameActivity calls `GameActivity_onCreate` (defined here) on the Java main thread. The glue
//!   installs its callbacks and starts the application thread, which runs the application's
//!   `android_main(app: AndroidApp)`.
//! - Callbacks run on the Java main thread. They update [`Shared`], queue a [`MainEvent`] or
//!   input event, and wake the application thread's looper.
//! - The application thread (the event loop) waits on its own `ALooper`, then drains the queues.
//!
//! Synchronization contracts (same as `android_native_app_glue`):
//!
//! - `onNativeWindowDestroyed` blocks the Java main thread until the application thread has
//!   handled [`MainEvent::TerminateWindow`], so the surface is never used after it is destroyed.
//! - `onDestroy` blocks until `android_main` has returned.

mod callbacks;
pub(crate) mod input;
pub(crate) mod key_map;
pub(crate) mod logcat;

use std::{
	collections::{HashMap, VecDeque},
	ffi::{CStr, c_void},
	path::PathBuf,
	ptr::{self, NonNull},
	sync::{
		Arc, Condvar, Mutex, MutexGuard, OnceLock, RwLock,
		atomic::{AtomicPtr, Ordering},
	},
	time::Duration,
};

use jni::{JavaVM, objects::Global, objects::JObject};
use ndk::{
	asset::AssetManager,
	configuration::Configuration,
	looper::{ForeignLooper, Poll, ThreadLooper},
	native_window::NativeWindow,
};
use ndk_sys::ARect;

use self::{input::InputEvent, key_map::KeyCharacterMap};
use crate::sys::{self, GAMECOMMON_INSETS_TYPE_COUNT, GameActivity};

/// Notifications from the Java main thread, in the order they happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainEvent {
	Start,
	Resume,
	Pause,
	Stop,
	/// The activity is being destroyed; `android_main` must return.
	Destroy,
	GainedFocus,
	LostFocus,
	/// A native window (surface) is available: [`AndroidApp::native_window`] returns it.
	InitWindow,
	/// The native window is about to be destroyed. It stays valid until this event has been
	/// handled.
	TerminateWindow,
	WindowResized,
	RedrawNeeded,
	ConfigChanged,
	LowMemory,
	InsetsChanged,
	ContentRectChanged,
	SoftKeyboardVisibilityChanged,
	TextInputChanged,
	EditorAction(i32),
}

/// The text editing state reported by the IME, decoded from GameTextInput.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RawTextInputState {
	pub text: String,
	/// UTF-16 code unit offsets, as reported by Java.
	pub selection: (i32, i32),
	/// UTF-16 code unit offsets; `None` when there is no composing region.
	pub compose: Option<(i32, i32)>,
}

pub(crate) struct State {
	pub main_events: VecDeque<MainEvent>,
	pub input: VecDeque<InputEvent>,
	pub native_window: Option<NativeWindow>,
	/// Set while the Java main thread waits in `onNativeWindowDestroyed`.
	pub terminate_window_pending: bool,
	/// `false` once `android_main` has returned.
	pub app_running: bool,
	pub content_rect: ARect,
	pub insets: [ARect; GAMECOMMON_INSETS_TYPE_COUNT],
	pub soft_keyboard_visible: bool,
	pub text_input: RawTextInputState,
}

pub(crate) struct Shared {
	/// Null once the activity has been destroyed.
	activity: AtomicPtr<GameActivity>,
	pub vm: JavaVM,
	/// Our own global reference to the Java `GameActivity`, valid as long as `Shared` lives.
	activity_ref: Global<JObject<'static>>,
	asset_manager: AssetManager,
	internal_data_path: Option<PathBuf>,
	external_data_path: Option<PathBuf>,
	sdk_version: i32,
	looper: OnceLock<ForeignLooper>,
	state: Mutex<State>,
	cond: Condvar,
	config: RwLock<Configuration>,
	key_maps: Mutex<HashMap<i32, KeyCharacterMap>>,
	pub(crate) ime: Mutex<crate::ime::ImeSession>,
}

impl Shared {
	pub(crate) fn lock(&self) -> MutexGuard<'_, State> {
		self.state.lock().unwrap_or_else(|e| e.into_inner())
	}

	/// Wakes the application thread, if it has started.
	pub(crate) fn wake(&self) {
		if let Some(looper) = self.looper.get() {
			looper.wake();
		}
	}

	pub(crate) fn push_main_event(&self, event: MainEvent) {
		self.lock().main_events.push_back(event);
		self.wake();
	}

	/// Runs `f` with the activity pointer, or returns `None` after the activity was destroyed.
	pub(crate) fn with_activity<R>(&self, f: impl FnOnce(*mut GameActivity) -> R) -> Option<R> {
		let activity = self.activity.load(Ordering::Acquire);
		(!activity.is_null()).then(|| f(activity))
	}

	fn refresh_config(&self) {
		let config = Configuration::from_asset_manager(&self.asset_manager);
		*self.config.write().unwrap_or_else(|e| e.into_inner()) = config;
	}

	fn refresh_insets(&self, activity: *mut GameActivity) {
		let mut insets =
			[ARect { left: 0, top: 0, right: 0, bottom: 0 }; GAMECOMMON_INSETS_TYPE_COUNT];
		for (ty, rect) in insets.iter_mut().enumerate() {
			unsafe { sys::GameActivity_getWindowInsets(activity, ty as u8, rect) };
		}
		self.lock().insets = insets;
	}
}

/// The application's handle on its Android activity, passed to `android_main`.
///
/// Cheap to clone. Methods that need the activity do nothing once it has been destroyed.
#[derive(Clone)]
pub struct AndroidApp {
	pub(crate) shared: Arc<Shared>,
}

impl std::fmt::Debug for AndroidApp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AndroidApp").finish_non_exhaustive()
	}
}

impl AndroidApp {
	/// The `JavaVM*` of the process.
	pub fn vm_as_ptr(&self) -> *mut c_void {
		self.shared.vm.get_raw().cast()
	}

	/// A JNI global reference (`jobject`) to the Java `GameActivity`. Valid as long as any
	/// `AndroidApp` clone is alive.
	pub fn activity_as_ptr(&self) -> *mut c_void {
		self.shared.activity_ref.as_raw().cast()
	}

	/// The current native window (surface), between `surface_available` and `surface_lost`.
	pub fn native_window(&self) -> Option<NativeWindow> {
		self.shared.lock().native_window.clone()
	}

	/// A snapshot of the current configuration (density, UI mode, ...).
	pub fn config(&self) -> Configuration {
		self.shared.config.read().unwrap_or_else(|e| e.into_inner()).clone()
	}

	pub fn asset_manager(&self) -> AssetManager {
		unsafe { AssetManager::from_ptr(self.shared.asset_manager.ptr()) }
	}

	pub fn internal_data_path(&self) -> Option<PathBuf> {
		self.shared.internal_data_path.clone()
	}

	pub fn external_data_path(&self) -> Option<PathBuf> {
		self.shared.external_data_path.clone()
	}

	/// `Build.VERSION.SDK_INT` of the device.
	pub fn sdk_version(&self) -> i32 {
		self.shared.sdk_version
	}

	/// Asks the system to finish the activity (`Activity.finish()`).
	pub fn finish(&self) {
		self.shared.with_activity(|a| unsafe { sys::GameActivity_finish(a) });
	}

	/// Adds and removes `WindowManager.LayoutParams` flags (e.g. `FLAG_KEEP_SCREEN_ON`).
	pub fn set_window_flags(&self, add: u32, remove: u32) {
		self.shared.with_activity(|a| unsafe { sys::GameActivity_setWindowFlags(a, add, remove) });
	}

	pub fn create_waker(&self) -> AndroidAppWaker {
		AndroidAppWaker { shared: Arc::downgrade(&self.shared) }
	}

	// ───────────────────── crate-internal: used by the event loop ─────────────────────

	/// Waits until something happens or `timeout` elapses. Returns `true` on timeout.
	///
	/// Must be called on the application thread.
	pub(crate) fn wait(&self, timeout: Option<Duration>) -> bool {
		{
			let state = self.shared.lock();
			if !state.main_events.is_empty() || !state.input.is_empty() {
				return false;
			}
		}
		let Some(looper) = ThreadLooper::for_thread() else {
			logcat::error("evloop: the event loop must run on the android_main thread");
			return true;
		};
		let result = match timeout {
			Some(timeout) => looper.poll_once_timeout(timeout),
			None => looper.poll_once(),
		};
		matches!(result, Ok(Poll::Timeout))
	}

	pub(crate) fn take_main_event(&self) -> Option<MainEvent> {
		self.shared.lock().main_events.pop_front()
	}

	pub(crate) fn take_input(&self) -> VecDeque<InputEvent> {
		std::mem::take(&mut self.shared.lock().input)
	}

	pub(crate) fn has_pending_input(&self) -> bool {
		!self.shared.lock().input.is_empty()
	}

	/// Must be called once `MainEvent::TerminateWindow` has been handled: releases the window
	/// and lets `onNativeWindowDestroyed` return.
	pub(crate) fn window_terminated(&self) {
		let mut state = self.shared.lock();
		state.native_window = None;
		state.terminate_window_pending = false;
		drop(state);
		self.shared.cond.notify_all();
	}

	pub(crate) fn insets(&self, ty: u8) -> ARect {
		self.shared.lock().insets.get(ty as usize).copied().unwrap_or(ARect {
			left: 0,
			top: 0,
			right: 0,
			bottom: 0,
		})
	}

	pub(crate) fn soft_keyboard_visible(&self) -> bool {
		self.shared.lock().soft_keyboard_visible
	}

	pub(crate) fn raw_text_input_state(&self) -> RawTextInputState {
		self.shared.lock().text_input.clone()
	}

	// The GameActivity IME calls below are queued to the Java main thread by GameActivity, so they
	// can be called from any thread and are applied in order.

	pub(crate) fn show_soft_input(&self) {
		self.shared.with_activity(|a| unsafe { sys::GameActivity_showSoftInput(a, 0) });
	}

	pub(crate) fn hide_soft_input(&self) {
		self.shared.with_activity(|a| unsafe { sys::GameActivity_hideSoftInput(a, 0) });
	}

	pub(crate) fn restart_input(&self) {
		self.shared.with_activity(|a| unsafe { sys::GameActivity_restartInput(a) });
	}

	pub(crate) fn set_ime_editor_info(&self, input_type: u32, action: u32, options: u32) {
		self.shared.with_activity(|a| unsafe {
			sys::GameActivity_setImeEditorInfo(a, input_type, action, options)
		});
	}

	/// Replaces the IME's editor state. `selection` is in UTF-16 code units. There is no
	/// composing region afterwards.
	pub(crate) fn set_text_input_state(&self, text: &str, selection: (i32, i32)) {
		let bytes = callbacks::encode_modified_utf8(text);
		let state = sys::GameTextInputState {
			text_UTF8: bytes.as_ptr().cast(),
			text_length: bytes.len() as i32,
			selection: sys::GameTextInputSpan { start: selection.0, end: selection.1 },
			composingRegion: sys::GameTextInputSpan { start: -1, end: -1 },
		};
		// GameActivity copies the text before returning.
		self.shared.with_activity(|a| unsafe { sys::GameActivity_setTextInputState(a, &state) });
	}

	pub(crate) fn ime_session(&self) -> MutexGuard<'_, crate::ime::ImeSession> {
		self.shared.ime.lock().unwrap_or_else(|e| e.into_inner())
	}

	pub(crate) fn wake(&self) {
		self.shared.wake();
	}

	pub(crate) fn device_key_character_map(
		&self,
		device_id: i32,
	) -> Result<KeyCharacterMap, jni::errors::Error> {
		let mut maps = self.shared.key_maps.lock().unwrap_or_else(|e| e.into_inner());
		if let Some(map) = maps.get(&device_id) {
			return Ok(map.clone());
		}
		let map = KeyCharacterMap::for_device(&self.shared.vm, device_id)?;
		maps.insert(device_id, map.clone());
		Ok(map)
	}
}

/// Wakes the application thread from any thread. Does nothing after the activity is gone.
#[derive(Clone)]
pub struct AndroidAppWaker {
	shared: std::sync::Weak<Shared>,
}

impl AndroidAppWaker {
	pub fn wake(&self) {
		if let Some(shared) = self.shared.upgrade() {
			shared.wake();
		}
	}
}

impl std::fmt::Debug for AndroidAppWaker {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AndroidAppWaker").finish_non_exhaustive()
	}
}

// ───────────────────────────── entry point ─────────────────────────────

unsafe extern "C" {
	/// The C++ implementation, renamed with a `_C` suffix in the vendored sources.
	fn Java_com_google_androidgamesdk_GameActivity_initializeNativeCode_C(
		env: *mut jni::sys::JNIEnv,
		java_game_activity: jni::sys::jobject,
		internal_data_dir: jni::sys::jstring,
		obb_dir: jni::sys::jstring,
		external_data_dir: jni::sys::jstring,
		asset_manager: jni::sys::jobject,
		saved_state: jni::sys::jbyteArray,
		java_config: jni::sys::jobject,
	) -> jni::sys::jlong;
}

/// The JNI entry point called by `GameActivity.onCreate`.
///
/// Re-exported from Rust because symbols of C/C++ static libraries are not exported from a Rust
/// `cdylib` (<https://github.com/rust-lang/rfcs/issues/2771>).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_google_androidgamesdk_GameActivity_initializeNativeCode(
	env: *mut jni::sys::JNIEnv,
	java_game_activity: jni::sys::jobject,
	internal_data_dir: jni::sys::jstring,
	obb_dir: jni::sys::jstring,
	external_data_dir: jni::sys::jstring,
	asset_manager: jni::sys::jobject,
	saved_state: jni::sys::jbyteArray,
	java_config: jni::sys::jobject,
) -> jni::sys::jlong {
	unsafe {
		Java_com_google_androidgamesdk_GameActivity_initializeNativeCode_C(
			env,
			java_game_activity,
			internal_data_dir,
			obb_dir,
			external_data_dir,
			asset_manager,
			saved_state,
			java_config,
		)
	}
}

unsafe extern "Rust" {
	/// Defined by the application with `#[unsafe(no_mangle)] fn android_main(app: AndroidApp)`.
	safe fn android_main(app: AndroidApp);
}

/// Called by GameActivity on the Java main thread when the activity is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GameActivity_onCreate(
	activity: *mut GameActivity,
	_saved_state: *mut c_void,
	_saved_state_size: usize,
) {
	abort_on_panic(|| unsafe { on_create(activity) });
}

unsafe fn on_create(activity: *mut GameActivity) {
	logcat::forward_stdio();

	let raw = unsafe { &*activity };
	let vm = unsafe { JavaVM::from_raw(raw.vm.cast()) };

	let activity_ref = vm
		.attach_current_thread(|env| -> jni::errors::Result<_> {
			let obj = unsafe { JObject::from_raw(env, raw.javaGameActivity) };
			let global = env.new_global_ref(&obj)?;
			init_ndk_context(&vm, env, &obj);
			Ok(global)
		})
		.expect("evloop: failed to create a global reference to the activity");

	let asset_manager = unsafe {
		AssetManager::from_ptr(NonNull::new(raw.assetManager).expect("null AAssetManager"))
	};
	let path = |p: *const std::ffi::c_char| {
		(!p.is_null())
			.then(|| unsafe { CStr::from_ptr(p) }.to_str().ok().map(PathBuf::from))
			.flatten()
	};

	let shared = Arc::new(Shared {
		activity: AtomicPtr::new(activity),
		vm,
		activity_ref,
		config: RwLock::new(Configuration::from_asset_manager(&asset_manager)),
		asset_manager,
		internal_data_path: path(raw.internalDataPath),
		external_data_path: path(raw.externalDataPath),
		sdk_version: raw.sdkVersion,
		looper: OnceLock::new(),
		state: Mutex::new(State {
			main_events: VecDeque::new(),
			input: VecDeque::new(),
			native_window: None,
			terminate_window_pending: false,
			app_running: true,
			content_rect: ARect { left: 0, top: 0, right: 0, bottom: 0 },
			insets: [ARect { left: 0, top: 0, right: 0, bottom: 0 }; GAMECOMMON_INSETS_TYPE_COUNT],
			soft_keyboard_visible: false,
			text_input: RawTextInputState::default(),
		}),
		cond: Condvar::new(),
		key_maps: Mutex::new(HashMap::new()),
		ime: Mutex::new(crate::ime::ImeSession::default()),
	});

	for &axis in input::ENABLED_AXES {
		unsafe { sys::GameActivityPointerAxes_enableAxis(axis) };
	}

	unsafe { callbacks::install(activity) };
	// The callbacks find `Shared` through `instance`; this reference is released in `onDestroy`.
	unsafe { (*activity).instance = Arc::into_raw(shared.clone()) as *mut c_void };

	let thread_shared = shared.clone();
	let spawned = std::thread::Builder::new().name("android_main".into()).spawn(move || {
		let looper = ThreadLooper::prepare();
		let _ = thread_shared.looper.set(looper.as_foreign().clone());

		// Attach permanently so that JNI calls from the event loop are cheap.
		let _ =
			thread_shared.vm.attach_current_thread(|_env| -> jni::errors::Result<()> { Ok(()) });

		let app = AndroidApp { shared: thread_shared.clone() };
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| android_main(app)));
		if result.is_err() {
			logcat::error("evloop: android_main panicked");
		}

		let mut state = thread_shared.lock();
		state.app_running = false;
		state.terminate_window_pending = false;
		drop(state);
		thread_shared.cond.notify_all();
		drop(looper);
	});

	if spawned.is_err() {
		logcat::error("evloop: failed to start the android_main thread");
		shared.lock().app_running = false;
	}
}

/// Initializes `ndk-context` (used by crates such as `cpal` or `oboe`) with the `Application`
/// context, once per process.
fn init_ndk_context(vm: &JavaVM, env: &mut jni::Env<'_>, activity: &JObject<'_>) {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		let result = (|| -> jni::errors::Result<()> {
			let application = env
				.call_method(
					activity,
					jni::jni_str!("getApplication"),
					jni::jni_sig!("()Landroid/app/Application;"),
					&[],
				)?
				.l()?;
			// Leaked on purpose: the context must stay valid for the whole process.
			let application = env.new_global_ref(application)?.into_raw();
			unsafe {
				ndk_context::initialize_android_context(vm.get_raw().cast(), application.cast())
			};
			Ok(())
		})();
		if let Err(err) = result {
			env.exception_clear();
			logcat::error(&format!("evloop: failed to initialize ndk-context: {err:?}"));
		}
	});
}

/// Panics must not unwind into the Java runtime.
pub(crate) fn abort_on_panic<R>(f: impl FnOnce() -> R) -> R {
	match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
		Ok(r) => r,
		Err(_) => {
			logcat::error("evloop: panic in a GameActivity callback; aborting");
			std::process::abort()
		}
	}
}

/// Recovers `Shared` from `GameActivity::instance` inside a callback.
///
/// # Safety
///
/// `activity` must be the pointer passed to a GameActivity callback between `onCreate` and the
/// end of `onDestroy`.
pub(crate) unsafe fn shared_from(activity: *mut GameActivity) -> Option<&'static Shared> {
	let instance = unsafe { (*activity).instance } as *const Shared;
	if instance.is_null() { None } else { Some(unsafe { &*instance }) }
}

/// Releases the reference held by `GameActivity::instance` (end of `onDestroy`).
pub(crate) unsafe fn release_instance(activity: *mut GameActivity) {
	let instance = unsafe { (*activity).instance } as *const Shared;
	unsafe { (*activity).instance = ptr::null_mut() };
	if !instance.is_null() {
		let shared = unsafe { Arc::from_raw(instance) };
		shared.activity.store(ptr::null_mut(), Ordering::Release);
	}
}

impl Shared {
	/// Blocks the calling (Java main) thread while `condition` holds and `android_main` runs.
	pub(crate) fn wait_while(&self, mut condition: impl FnMut(&State) -> bool) {
		let mut state = self.lock();
		while state.app_running && condition(&state) {
			let (next, timeout) = self
				.cond
				.wait_timeout(state, Duration::from_secs(5))
				.unwrap_or_else(|e| e.into_inner());
			state = next;
			if timeout.timed_out() && state.app_running && condition(&state) {
				logcat::error(
					"evloop: still waiting for the application thread; is the event loop running?",
				);
			}
		}
	}
}
