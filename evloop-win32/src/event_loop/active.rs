use std::{mem, rc::Rc};

use evloop_core::{error::CreateWindowError, event_loop::ControlFlow, window::Theme};
use raw_window_handle::{DisplayHandle, HasDisplayHandle, RawDisplayHandle, WindowsDisplayHandle};

use crate::{
	event_loop::{EventLoopProxy, runner::EventLoopRunner},
	monitor::{MonitorHandle, available_monitors, primary_monitor},
	window::{Window, WindowAttributes, should_use_dark_mode},
};

#[repr(transparent)]
#[derive(Debug)]
pub struct ActiveEventLoop(pub(crate) Rc<EventLoopRunner>);

impl ActiveEventLoop {
	pub(crate) fn from_ref(shared_runner: &Rc<EventLoopRunner>) -> &Self {
		// FIXME(Tito5e): [UPSTREAM] Find safer way (original: Implement `ActiveEventLoop` for `Rc<EventLoopRunner>` directly)
		unsafe { mem::transmute::<&Rc<EventLoopRunner>, &Self>(shared_runner) }
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
}

impl HasDisplayHandle for ActiveEventLoop {
	fn display_handle(
		&self,
	) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
		let raw = RawDisplayHandle::Windows(WindowsDisplayHandle::new());
		unsafe { Ok(DisplayHandle::borrow_raw(raw)) }
	}
}
