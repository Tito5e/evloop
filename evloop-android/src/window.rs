use std::sync::Mutex;

use crate::RedrawRequester;
use crate::event_loop::ActiveEventLoop;
use android_activity::{AndroidApp, ConfigurationRef, Rect};
use evloop_core::{error::CreateWindowError, ime::ImeCapabilities};
use geometry_types::PhysicalSize;
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
	pub(crate) fn new(event_loop: &ActiveEventLoop) -> Result<Self, CreateWindowError> {
		Ok(Self {
			app: event_loop.app.clone(),
			ime_capabilities: Default::default(),
			redraw_requester: event_loop.redraw_requester.clone(),
		})
	}

	pub(crate) fn config(&self) -> ConfigurationRef {
		self.app.config()
	}

	pub(crate) fn content_rect(&self) -> Rect {
		self.app.content_rect()
	}

	fn raw_window_handle_rwh_06(&self) -> Result<RawWindowHandle, HandleError> {
		if let Some(native_window) = self.app.native_window() {
			let ptr = native_window.ptr().cast();
			Ok(RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(ptr)))
		} else {
			tracing::error!(
				"Cannot get the native window, it's null and will always be null before \
                AppicationHandler::surface_available and after ApplicationHandler::surface_lost. \
                Make sure you only call this function between those events."
			);
			Err(HandleError::Unavailable)
		}
	}

	fn raw_display_handle_rwh_06(&self) -> Result<RawDisplayHandle, HandleError> {
		Ok(RawDisplayHandle::Android(AndroidDisplayHandle::new()))
	}

	pub fn inner_size(&self) -> PhysicalSize {
		self.outer_size()
	}

	pub fn outer_size(&self) -> PhysicalSize {
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
