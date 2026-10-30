use std::{mem, rc::Rc};

use crate::event_loop::runner::EventLoopRunner;

#[repr(transparent)]
#[derive(Debug)]
pub struct ActiveEventLoop(pub Rc<EventLoopRunner>);

impl ActiveEventLoop {
	pub(crate) fn from_ref(shared_runner: &Rc<EventLoopRunner>) -> &Self {
		// FIXME(Tito5e): [UPSTREAM] Find safer way (original: Implement `ActiveEventLoop` for `Rc<EventLoopRunner>` directly)
		unsafe { mem::transmute::<&Rc<EventLoopRunner>, &Self>(shared_runner) }
	}
}
