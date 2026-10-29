use evloop_core::{
	event::{EvloopEvent, WindowEvent},
	event_loop::StartCause,
};

pub trait ApplicationHandler {
	fn started(&mut self, event_loop: &ActiveEventLoop, cause: StartCause);

	fn stopped(&mut self, event_loop: &ActiveEventLoop) {
		let _ = event_loop;
	}

	fn surface_available(&mut self, event_loop: &ActiveEventLoop);

	fn surface_lost(&mut self, event_loop: &ActiveEventLoop);

	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let _ = event_loop;
	}

	fn paused(&mut self, event_loop: &ActiveEventLoop) {
		let _ = event_loop;
	}

	fn enter_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
		let _ = (event_loop, cause);
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent);

	fn evloop_event(&mut self, event_loop: &ActiveEventLoop, event: EvloopEvent) {
		let _ = (event_loop, event);
	}

	fn exit_events(&mut self, event_loop: &ActiveEventLoop) {
		let _ = event_loop;
	}
}
