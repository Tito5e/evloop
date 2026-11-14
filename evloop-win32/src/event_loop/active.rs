use std::rc::Rc;

use evloop_core::{
	cursor::CursorImage,
	error::{CreateWindowError, CustomCursorError},
	event_loop::{ControlFlow, DeviceEventStrategy},
	window::Theme,
};
use raw_window_handle::{DisplayHandle, HasDisplayHandle, RawDisplayHandle, WindowsDisplayHandle};

use crate::{
	event_loop::{EventLoopProxy, runner::EventLoopRunner},
	input::raw_input::register_all_mouse_and_keyboards_for_raw_input,
	monitor::{MonitorHandle, available_monitors, primary_monitor},
	resource::Cursor,
	window::{Window, WindowAttributes, should_use_dark_mode},
};

#[repr(transparent)]
#[derive(Debug)]
pub struct ActiveEventLoop(pub(crate) Rc<EventLoopRunner>);

impl ActiveEventLoop {
	pub(crate) fn from_ref(shared_runner: &Rc<EventLoopRunner>) -> &Self {
		// SAFETY: `ActiveEventLoop` is `#[repr(transparent)]` over `Rc<EventLoopRunner>`, so both
		// types have the same layout and the reference keeps the same lifetime.
		unsafe { &*(shared_runner as *const Rc<EventLoopRunner> as *const Self) }
	}

	/// A display handle that can be kept, cloned, and sent to other threads, independently of the
	/// event loop's lifetime (e.g. to create a graphics instance before any window exists).
	pub fn owned_display_handle(&self) -> OwnedDisplayHandle {
		OwnedDisplayHandle { _private: () }
	}

	pub fn create_proxy(&self) -> EventLoopProxy {
		EventLoopProxy { target_window: self.0.thread_msg_target }
	}

	pub fn create_window(
		&self,
		window_attributes: WindowAttributes,
	) -> Result<Window, CreateWindowError> {
		Window::new(self, window_attributes)
	}

	/// Creates a cursor from an RGBA image, to be passed to `Window::set_cursor`.
	pub fn create_custom_cursor(&self, image: CursorImage) -> Result<Cursor, CustomCursorError> {
		Cursor::new(&image)
	}

	pub fn available_monitors(&self) -> impl Iterator<Item = MonitorHandle> {
		available_monitors().into_iter()
	}

	pub fn primary_monitor(&self) -> Option<MonitorHandle> {
		Some(primary_monitor())
	}

	pub fn exiting(&self) -> bool {
		self.0.exit_code().is_some()
	}

	pub fn system_theme(&self) -> Option<Theme> {
		Some(if should_use_dark_mode() { Theme::Dark } else { Theme::Light })
	}

	pub fn set_control_flow(&self, control_flow: ControlFlow) {
		self.0.set_control_flow(control_flow)
	}

	pub fn control_flow(&self) -> ControlFlow {
		self.0.control_flow()
	}

	pub fn exit(&self) {
		self.0.set_exit_code(0)
	}

	/// Changes when [`ApplicationHandler::device_event`](crate::application::ApplicationHandler::device_event)
	/// is called. The default is [`DeviceEventStrategy::WhenFocused`].
	pub fn set_device_event_strategy(&self, strategy: DeviceEventStrategy) {
		register_all_mouse_and_keyboards_for_raw_input(self.0.thread_msg_target, strategy);
	}
}

impl HasDisplayHandle for ActiveEventLoop {
	fn display_handle(
		&self,
	) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
		let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
		unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
	}
}

/// An owned display handle, see [`ActiveEventLoop::owned_display_handle`].
///
/// On Windows there is no display connection to own; the handle is always available.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedDisplayHandle {
	_private: (),
}

impl HasDisplayHandle for OwnedDisplayHandle {
	fn display_handle(
		&self,
	) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
		let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
		unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
	}
}
