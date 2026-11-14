use evloop_core::{
	error::ImeRequestError,
	ime::{ImeCapabilities, ImeRequest},
	window::PhysicalInsets,
};
use geometry_types::{PhysicalBounds, PhysicalSize};
use ndk_sys::ARect;
use raw_window_handle::{
	AndroidDisplayHandle, AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
	HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle,
};

use crate::{
	RedrawRequester,
	event_loop::ActiveEventLoop,
	glue::AndroidApp,
	platform::InsetType,
	sys::{
		GAMECOMMON_INSETS_TYPE_DISPLAY_CUTOUT, GAMECOMMON_INSETS_TYPE_IME,
		GAMECOMMON_INSETS_TYPE_SYSTEM_BARS,
	},
};

pub struct Window {
	app: AndroidApp,
	redraw_requester: RedrawRequester,
}

impl Window {
	pub(crate) fn new(event_loop: &ActiveEventLoop) -> Self {
		Self { app: event_loop.app.clone(), redraw_requester: event_loop.redraw_requester.clone() }
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

	/// The insets covered by system UI: system bars (status / navigation / caption) and the
	/// display cutout.
	pub fn safe_area(&self) -> PhysicalInsets {
		safe_area(&self.app)
	}

	/// The part of the surface not covered by system UI: [`Self::safe_area`] removed from the
	/// surface.
	pub fn content_rect(&self) -> PhysicalBounds {
		content_rect(&self.app)
	}

	/// The insets covered by the on-screen keyboard.
	pub fn ime_occlusion(&self) -> PhysicalInsets {
		ime_occlusion(&self.app)
	}

	/// The insets of one kind (`WindowInsetsCompat.Type`). Android-specific.
	pub fn insets(&self, ty: InsetType) -> PhysicalInsets {
		to_insets(self.app.insets(ty.to_raw()))
	}

	/// Starts, updates or ends the IME session. See `evloop_core::ime` for the semantics shared by
	/// all backends.
	///
	/// On Android:
	/// - `Enable` shows the on-screen keyboard and `Disable` hides it. To show the keyboard again
	///   after the user hid it, disable and enable the session again.
	/// - `hint_and_purpose` selects the keyboard (`EditorInfo.inputType`).
	/// - `surrounding_text` is mirrored in the IME's editor, which improves suggestions and lets
	///   the IME delete text it did not type.
	/// - `cursor_area` is accepted but has no effect.
	/// - Events are delivered asynchronously, after this call returns.
	pub fn request_ime_update(&self, request: ImeRequest) -> Result<(), ImeRequestError> {
		crate::ime::request(&self.app, request)
	}

	/// The capabilities of the current IME session, or `None` when no session is enabled.
	pub fn ime_capabilities(&self) -> Option<ImeCapabilities> {
		self.app.ime_session().capabilities()
	}

	pub fn surface_size(&self) -> PhysicalSize {
		surface_size(&self.app)
	}

	/// Does nothing on Android: presentation is paced by the surface itself. Kept so that
	/// rendering code can call it on every backend.
	pub fn pre_present_notify(&self) {}

	pub fn request_redraw(&self) {
		self.redraw_requester.request_redraw()
	}

	pub fn scale_factor(&self) -> f64 {
		crate::scale_factor(&self.app)
	}
}

fn to_insets(rect: ARect) -> PhysicalInsets {
	let clamp = |v: i32| v.max(0) as u32;
	PhysicalInsets::new(clamp(rect.top), clamp(rect.left), clamp(rect.bottom), clamp(rect.right))
}

pub(crate) fn safe_area(app: &AndroidApp) -> PhysicalInsets {
	to_insets(app.insets(GAMECOMMON_INSETS_TYPE_SYSTEM_BARS))
		.union(to_insets(app.insets(GAMECOMMON_INSETS_TYPE_DISPLAY_CUTOUT)))
}

pub(crate) fn content_rect(app: &AndroidApp) -> PhysicalBounds {
	safe_area(app).inset_bounds(surface_size(app))
}

fn surface_size(app: &AndroidApp) -> PhysicalSize {
	if let Some(native_window) = app.native_window().as_ref() {
		PhysicalSize::new(native_window.width() as _, native_window.height() as _)
	} else {
		PhysicalSize::new(0, 0)
	}
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

pub(crate) fn ime_occlusion(app: &AndroidApp) -> PhysicalInsets {
	to_insets(app.insets(GAMECOMMON_INSETS_TYPE_IME))
}
