use std::rc::Rc;

use crate::event_loop::runner::{Event, EventLoopRunner};

pub(crate) struct ThreadTargetUserData {
	pub(crate) event_loop_runner: Rc<EventLoopRunner>,
}

impl ThreadTargetUserData {
	pub(crate) fn send_wakeup(&self) {
		self.event_loop_runner.send_event(Event::WakeUp);
	}
}
