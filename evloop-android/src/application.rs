use crate::event::WindowEvent;
use crate::event_loop::ActiveEventLoop;
use crate::window::Window;
use evloop_core::event::EvloopEvent;
use evloop_core::event_loop::StartCause;

pub trait ApplicationHandler {
	fn started(&mut self, event_loop: &ActiveEventLoop, cause: StartCause);

	fn stopped(&mut self, event_loop: &ActiveEventLoop) {
		let _ = event_loop;
	}

	fn surface_available(&mut self, event_loop: &ActiveEventLoop, window: Window);

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

impl<A: ?Sized + ApplicationHandler> ApplicationHandler for &mut A {
	#[inline]
	fn started(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
		(**self).started(event_loop, cause);
	}

	#[inline]
	fn stopped(&mut self, event_loop: &ActiveEventLoop) {
		(**self).stopped(event_loop);
	}

	#[inline]
	fn surface_available(&mut self, event_loop: &ActiveEventLoop, window: Window) {
		(**self).surface_available(event_loop, window);
	}

	#[inline]
	fn surface_lost(&mut self, event_loop: &ActiveEventLoop) {
		(**self).surface_lost(event_loop);
	}

	#[inline]
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		(**self).resumed(event_loop);
	}

	#[inline]
	fn paused(&mut self, event_loop: &ActiveEventLoop) {
		(**self).paused(event_loop);
	}

	#[inline]
	fn enter_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
		(**self).enter_events(event_loop, cause);
	}

	#[inline]
	fn window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
		(**self).window_event(event_loop, event);
	}

	#[inline]
	fn evloop_event(&mut self, event_loop: &ActiveEventLoop, event: EvloopEvent) {
		(**self).evloop_event(event_loop, event);
	}

	#[inline]
	fn exit_events(&mut self, event_loop: &ActiveEventLoop) {
		(**self).exit_events(event_loop);
	}
}
