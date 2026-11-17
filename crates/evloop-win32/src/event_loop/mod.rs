mod opts;
mod runner;
mod thread;

use core::fmt;
use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::event_loop::{
    opts::EventLoopOpts, runner::EventLoopRunner, thread::create_message_window,
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

        // アプリケーションがDPIを処理できる場合は有効にする (通常は有効になる)
        if opts.dpi_aware {
            become_api_aware();
        }

        let message_window = create_message_window();
        let runner = Rc::new(EventLoopRunner::new(message_window));

        // アプリケーションがraw_inputを処理する場合は有効にする (通常は有効にならない)
        if opts.raw_input {
            register_devices_for_raw_input(message_window);
        }

        Ok(EventLoop { runner })
    }
}
