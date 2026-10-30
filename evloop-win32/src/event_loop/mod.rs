mod active;
mod attributes;
mod cross_thread;
mod runner;
mod thread_proc;
mod wait;

pub use active::ActiveEventLoop;

use std::{
	ffi::c_void,
	mem,
	os::windows::io::OwnedHandle,
	panic, ptr,
	rc::Rc,
	sync::atomic::{AtomicBool, Ordering},
	time::Duration,
};

use evloop_core::{error::EventLoopError, event_loop::PumpStatus};
use windows_sys::Win32::{
	System::Threading::GetCurrentThreadId,
	UI::WindowsAndMessaging::{DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage},
};

use crate::{
	application::ApplicationHandler,
	event_loop::{
		attributes::EventLoopAttributes,
		runner::EventLoopRunner,
		thread_proc::{create_event_target_window, insert_event_target_window_userdata},
		wait::wait_for_messages_impl,
	},
	sys::dpi::become_dpi_aware,
};

pub struct EventLoop {
	runner: Rc<EventLoopRunner>,
	msg_hook: Option<Box<dyn FnMut(*const c_void) -> bool + 'static>>,
	high_resolution_timer: Option<OwnedHandle>,
}

impl EventLoop {
	pub fn new(attributes: &mut EventLoopAttributes) -> Result<Self, EventLoopError> {
		static EVENT_LOOP_CREATED: AtomicBool = AtomicBool::new(false);
		if EVENT_LOOP_CREATED.swap(true, Ordering::Relaxed) {
			return Err(EventLoopError::Recreation);
		}

		let thread_id = unsafe { GetCurrentThreadId() };

		if !attributes.any_thread && thread_id != main_thread_id() {
			// ほかのプラットフォームではメインスレッド以外からイベントループを作成できないことも多いので、安全のためにデフォルトではパニックにする
			// TODO: メッセージ
			panic!();
		}

		if attributes.dpi_aware {
			become_dpi_aware();
		}

		let thread_msg_target = create_event_target_window();

		let runner_shared = Rc::new(EventLoopRunner::new(thread_id, thread_msg_target));

		insert_event_target_window_userdata(thread_msg_target, runner_shared.clone());

		Ok(Self {
			runner: runner_shared,
			msg_hook: attributes.msg_hook.take(),
			high_resolution_timer: None,
		})
	}

	pub fn run_app_on_demand<A: ApplicationHandler>(
		&mut self,
		mut app: A,
	) -> Result<(), EventLoopError> {
		self.runner.clear_exit();

		// SAFETY: resetter is not leaked.
		// エラーによってUnwindが発生した場合でも必ずrunnerの後処理を呼ぶ
		let _app_resetter = unsafe { self.runner.set_app(&mut app) };

		let exit_code = loop {
			self.wait_for_messages(None);
			if let Some(code) = self.runner.exit_code() {
				break code;
			}

			self.dispatch_peeked_messages();

			if let Some(code) = self.runner.exit_code() {
				break code;
			}
		};

		self.runner.loop_destroyed();
		self.runner.data_transfer().flush_clipboard_blocking();

		self.runner.reset_runner();

		if exit_code == 0 { Ok(()) } else { Err(EventLoopError::ExitFailure(exit_code)) }
	}

	pub fn pump_app_events<A: ApplicationHandler>(
		&mut self,
		timeout: Option<Duration>,
		mut app: A,
	) -> PumpStatus {
		// SAFETY: resetter is not leaked.
		// エラーによってUnwindが発生した場合でも必ずrunnerの後処理を呼ぶ
		let _app_resetter = unsafe { self.runner.set_app(&mut app) };

		self.runner.wakeup();

		if self.runner.exit_code().is_none() {
			self.wait_for_messages(timeout);
		}

		if self.runner.exit_code().is_none() {
			self.dispatch_peeked_messages();
		}

		if let Some(code) = self.runner.exit_code() {
			self.runner.loop_destroyed();
			self.runner.data_transfer().flush_clipboard_blocking();

			self.runner.reset_runner();
			PumpStatus::Exit(code)
		} else {
			self.runner.prepare_wait();
			PumpStatus::Continue
		}
	}

	fn wait_for_messages(&mut self, timeout: Option<Duration>) {
		self.runner.prepare_wait();
		wait_for_messages_impl(
			&mut self.high_resolution_timer,
			self.runner.control_flow(),
			timeout,
		);
		self.runner.wakeup();
	}

	fn dispatch_peeked_messages(&mut self) {
		self.runner.interrupt_msg_dispatch.set(false);

		let mut msg: MSG = unsafe { mem::zeroed() };

		loop {
			unsafe {
				if PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) == false.into() {
					break;
				}

				let handled = if let Some(callback) = self.msg_hook.as_deref_mut() {
					callback(&mut msg as *mut _ as *mut _)
				} else {
					false
				};

				if !handled {
					TranslateMessage(&msg);
					DispatchMessageW(&msg);
				}
			}

			if let Err(payload) = self.runner.take_panic_error() {
				self.runner.reset_runner();
				panic::resume_unwind(payload);
			}

			if let Some(_code) = self.runner.exit_code() {
				break;
			}

			if self.runner.interrupt_msg_dispatch.get() {
				break;
			}
		}
	}

	pub fn run_app<A: ApplicationHandler + 'static>(
		mut self,
		mut app: A,
	) -> Result<(), EventLoopError> {
		let result = self.run_app_on_demand(&mut app);
		drop(app);
		result
	}
}

fn main_thread_id() -> u32 {
	static mut MAIN_THREAD_ID: u32 = 0;

	#[unsafe(link_section = ".CRT$XCU")]
	static INIT_MAIN_THREAD_ID: unsafe extern "C" fn() = {
		unsafe extern "C" fn initer() {
			unsafe {
				MAIN_THREAD_ID = GetCurrentThreadId();
			}
		}
		initer
	};

	unsafe { MAIN_THREAD_ID }
}
