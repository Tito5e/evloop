mod opts;
mod runner;
mod thread;
mod window;

use core::fmt;
use std::{
    mem,
    os::windows::io::{AsRawHandle, OwnedHandle, RawHandle},
    panic, ptr,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{GetLastError, WAIT_FAILED},
    UI::WindowsAndMessaging::{
        DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, TranslateMessage, MSG,
        MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
    },
};

use crate::{
    dpi::become_api_aware,
    event_loop::{opts::EventLoopOpts, runner::EventLoopRunner, thread::create_message_window},
};

pub struct EventLoop {
    runner: Rc<EventLoopRunner>,
}

impl fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoop").finish_non_exhaustive()
    }
}

impl EventLoop {
    pub fn new(opts: EventLoopOpts) -> Result<Self, ()> {
        // イベントループを同一プロセス内で複数作る需要が分からないので、作れないようにする方が実装が楽になる
        static EVENT_LOOP_CREATED: AtomicBool = AtomicBool::new(false);
        if EVENT_LOOP_CREATED.swap(true, Ordering::Relaxed) {
            // TODO: 具体的なエラー型を用意したい
            return Err(());
        }

        // アプリケーションがDPIを処理できる場合は有効にする
        if opts.dpi_aware {
            become_api_aware();
        }

        let message_window = create_message_window();
        let runner = Rc::new(EventLoopRunner::new(message_window));

        // アプリケーションがraw_inputを処理する場合は有効にする
        if opts.raw_input {
            // TODO: Implement Raw Input
            // register_devices_for_raw_input(message_window);
        }

        Ok(EventLoop { runner })
    }

    // TODO: エラー型を用意する
    pub fn run_app_on_demand<A: ApplicationHandler>(&mut self, mut app: A) -> Result<(), ()> {
        self.runner.clear_exit();

        // SAFETY: resetterが必ずDropされること
        let _resetter = unsafe { self.runner.set_app(&mut app) };

        let exit_code = loop {
            self.wait_for_messages(None);

            if let Some(code) = self.exit_code() {
                break code;
            }

            self.dispatch_peeked_messages();

            if let Some(code) = self.exit_code() {
                break code;
            }
        };

        self.runner.loop_destroyed();

        self.runner.reset_runner();

        if exit_code == 0 {
            Ok(())
        } else {
            Err(())
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
}

fn wait_for_messages_impl(
    high_resolution_timer: &mut Option<OwnedHandle>,
    control_flow: ControlFlow,
    timeout: Option<Duration>,
) {
    let timeout = {
        let control_flow_timeout = match control_flow {
            ControlFlow::Wait => None,
            ControlFlow::Poll => Some(Duration::ZERO),
            ControlFlow::WaitUntil(wait_deadline) => {
                let start = Instant::now();
                Some(wait_deadline.saturating_duration_since(start))
            }
        };
        let timeout = min_timeout(timeout, control_flow_timeout);
        if timeout == Some(Duration::ZERO) {
            return;
        }

        timeout.map(|t| t.min(FIFTY_DAYS)).map(|t| t.max(MIN_WAIT))
    };

    if timeout.is_some() && high_resolution_timer.is_none() {
        *high_resolution_timer = create_high_resolution_timer();
    }

    let high_resolution_timer: Option<RawHandle> = high_resolution_timer
        .as_ref()
        .map(OwnedHandle::as_raw_handle);

    let use_timer: bool;
    if let (Some(handle), Some(timeout)) = (high_resolution_timer, timeout) {
        let res = unsafe { set_high_resolution_timer(handle, timeout) };
        if let Err(error_code) = res {
            tracing::trace!(
                "Failed to set high resolution timer: last error {}",
                error_code
            );
            use_timer = false;
        } else {
            use_timer = true;
        }
    } else {
        use_timer = false;
    }

    unsafe {
        let wait_duration_ms = timeout.map(dur2timeout).unwrap_or(INFINITE);

        let (num_handles, raw_handles) = if use_timer {
            (1, [high_resolution_timer.unwrap()])
        } else {
            (0, [ptr::null_mut()])
        };

        let result = MsgWaitForMultipleObjectsEx(
            num_handles,
            raw_handles.as_ptr() as *const _,
            wait_duration_ms,
            QS_ALLINPUT,
            MWMO_INPUTAVAILABLE,
        );
        if result == WAIT_FAILED {
            tracing::warn!(
                "Failed to MsgWaitForMultipleObjectsEx: error code {}",
                GetLastError()
            );
        }
    }
}
