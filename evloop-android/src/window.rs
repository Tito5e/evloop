use std::sync::Mutex;

use crate::event_loop::ActiveEventLoop;
use crate::{RedrawRequester, content_rect};
use android_activity::{AndroidApp, ConfigurationRef, Rect};
use evloop_core::{error::CreateWindowError, ime::ImeCapabilities};
use geometry_types::{PhysicalBounds, PhysicalSize};
use jni::objects::{JIntArray, JObject};
use jni::vm::JavaVM;
use jni::{jni_sig, jni_str};
use raw_window_handle::{
	AndroidDisplayHandle, AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
	HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle,
};

pub struct Window {
	app: AndroidApp,
	ime_capabilities: Mutex<Option<ImeCapabilities>>,
	redraw_requester: RedrawRequester,
}

impl Window {
	pub(crate) fn new(event_loop: &ActiveEventLoop) -> Self {
		Self {
			app: event_loop.app.clone(),
			ime_capabilities: Default::default(),
			redraw_requester: event_loop.redraw_requester.clone(),
		}
	}

	pub(crate) fn config(&self) -> ConfigurationRef {
		self.app.config()
	}

	fn raw_window_handle_rwh_06(&self) -> Result<RawWindowHandle, HandleError> {
		if let Some(native_window) = self.app.native_window() {
			let ptr = native_window.ptr().cast();
			Ok(RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(ptr)))
		} else {
			tracing::error!(
				"Cannot get the native window, it's null and will always be null before \
                ApplicationHandler::surface_available and after ApplicationHandler::surface_lost. \
                Make sure you only call this function between those events."
			);
			Err(HandleError::Unavailable)
		}
	}

	fn raw_display_handle_rwh_06(&self) -> Result<RawDisplayHandle, HandleError> {
		Ok(RawDisplayHandle::Android(AndroidDisplayHandle::new()))
	}

	pub fn content_rect(&self) -> PhysicalBounds {
		unsafe {
			let vm_ptr = self.app.vm_as_ptr() as *mut jni::sys::JavaVM;
			let activity_ptr = self.app.activity_as_ptr() as jni::sys::jobject;
			if vm_ptr.is_null() || activity_ptr.is_null() {
				panic!("このAPIは専用のActivityを必要とします")
			}

			let vm = JavaVM::from_raw(vm_ptr);

			let result = vm
				.attach_current_thread(|env| -> jni::errors::Result<PhysicalBounds> {
					let activity = JObject::from_raw(env, activity_ptr);

					let result_val = env.call_method(
						&activity,
						jni_str!("getSafeAreaInsets"),
						jni_sig!("()[I"),
						&[],
					)?;

					let result_obj = result_val.l()?;
					if result_obj.is_null() {
						return Err(jni::errors::Error::NullPtr("getSafeAreaInsets returned null"));
					}

					let int_array = JIntArray::from_raw(env, result_obj.as_raw());

					let mut elements = [0i32; 4];
					env.get_int_array_region(int_array, 0, &mut elements)?;

					Ok(PhysicalBounds {
						left: elements[2],
						right: elements[3],
						top: elements[0],
						bottom: elements[1],
					})
				})
				.unwrap();

			result
		}
	}

	pub fn surface_size(&self) -> PhysicalSize {
		screen_size(&self.app)
	}

	pub fn pre_present_notify(&self) {}

	pub fn request_redraw(&self) {
		self.redraw_requester.request_redraw()
	}

	pub fn scale_factor(&self) -> f64 {
		scale_factor(&self.app)
	}
}

fn screen_size(app: &AndroidApp) -> PhysicalSize {
	if let Some(native_window) = app.native_window().as_ref() {
		PhysicalSize::new(native_window.width() as _, native_window.height() as _)
	} else {
		PhysicalSize::new(0, 0)
	}
}

fn scale_factor(app: &AndroidApp) -> f64 {
	app.config().density().map(|dpi| dpi as f64 / 160.).unwrap_or(1.0)
}

impl HasDisplayHandle for Window {
	fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
		let raw = self.raw_display_handle_rwh_06()?;
		unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
	}
}

impl HasWindowHandle for Window {
	fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
		let raw = self.raw_window_handle_rwh_06()?;
		unsafe { Ok(WindowHandle::borrow_raw(raw)) }
	}
}
