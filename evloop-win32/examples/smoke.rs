//! Opens a window and prints every event.
//!
//! Keys:
//! - `D`: start dragging the window (hold the left mouse button while pressing)
//! - `M`: show the system window menu at the pointer
//! - `C`: switch to a custom (checkerboard) cursor
//! - `R`: toggle raw device events (`DeviceEventStrategy::Always` / `Never`)
//! - `I`: toggle the IME session (set `EVLOOP_SMOKE_IME` to enable it at startup)
//!
//! Set `EVLOOP_SMOKE_EXIT_MS` to exit automatically after that many milliseconds.

#[cfg(target_os = "windows")]
mod app {
	use std::time::{Duration, Instant};

	use evloop_core::{
		cursor::CursorImage,
		dpi::Position,
		event::{DeviceId, ElementState},
		event_loop::{ControlFlow, DeviceEventStrategy, StartCause},
		ime::{ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData},
		keyboard::{KeyCode, PhysicalKey},
		window::WindowId,
	};
	use evloop_win32::{
		application::ApplicationHandler,
		event::{DeviceEvent, WindowEvent},
		event_loop::{ActiveEventLoop, EventLoop, EventLoopAttributes},
		window::{Window, WindowAttributes},
	};
	use geometry_types::PhysicalPosition;

	#[derive(Default)]
	struct App {
		window: Option<Window>,
		exit_at: Option<Instant>,
		raw_input: bool,
		pointer: Option<PhysicalPosition>,
		device_events: u64,
	}

	impl ApplicationHandler for App {
		fn started(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
			let window = event_loop
				.create_window(WindowAttributes::default().with_title("evloop smoke"))
				.expect("failed to create window");
			println!(
				"created: scale_factor={} surface_size={:?} safe_area={:?} content_rect={:?} enabled_buttons={:?}",
				window.scale_factor(),
				window.surface_size(),
				window.safe_area(),
				window.content_rect(),
				window.enabled_buttons(),
			);
			if std::env::var_os("EVLOOP_SMOKE_IME").is_some() {
				toggle_ime(&window);
			}
			self.window = Some(window);
			self.raw_input = true;
			event_loop.set_device_event_strategy(DeviceEventStrategy::Always);

			if let Some(ms) =
				std::env::var("EVLOOP_SMOKE_EXIT_MS").ok().and_then(|v| v.parse().ok())
			{
				let at = Instant::now() + Duration::from_millis(ms);
				self.exit_at = Some(at);
				event_loop.set_control_flow(ControlFlow::WaitUntil(at));
			}
		}

		fn enter_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
			if self.exit_at.is_some_and(|at| Instant::now() >= at) {
				println!("exit timer reached ({} device events seen)", self.device_events);
				event_loop.exit();
			}
		}

		fn window_event(
			&mut self,
			event_loop: &ActiveEventLoop,
			_id: WindowId,
			event: WindowEvent,
		) {
			match &event {
				WindowEvent::PointerMoved { .. } | WindowEvent::RedrawRequested => {}
				_ => println!("window: {event:?}"),
			}

			let Some(window) = self.window.as_ref() else { return };

			match event {
				WindowEvent::CloseRequested => event_loop.exit(),
				WindowEvent::PointerMoved { position, .. } => self.pointer = Some(position),
				WindowEvent::ScaleFactorChanging { mut surface_size_writer, .. } => {
					println!("  proposed size: {:?}", surface_size_writer.surface_size());
				}
				WindowEvent::KeyboardInput { event, .. }
					if event.state == ElementState::Pressed =>
				{
					match event.physical_key {
						PhysicalKey::Code(KeyCode::KeyD) => {
							println!("  drag_window: {:?}", window.drag_window());
						}
						PhysicalKey::Code(KeyCode::KeyM) => {
							let p = self.pointer.unwrap_or(PhysicalPosition::new(0, 0));
							window.show_window_menu(Position::Physical(p));
						}
						PhysicalKey::Code(KeyCode::KeyC) => {
							let mut rgba = Vec::with_capacity(32 * 32 * 4);
							for y in 0..32u32 {
								for x in 0..32u32 {
									let on = ((x / 8) + (y / 8)) % 2 == 0;
									rgba.extend_from_slice(if on {
										&[255, 0, 0, 255]
									} else {
										&[0, 0, 255, 255]
									});
								}
							}
							let image = CursorImage::from_rgba(rgba, 32, 32, 0, 0).unwrap();
							let cursor = event_loop.create_custom_cursor(image).unwrap();
							window.set_cursor(cursor);
						}
						PhysicalKey::Code(KeyCode::KeyI) => toggle_ime(window),
						PhysicalKey::Code(KeyCode::KeyR) => {
							self.raw_input = !self.raw_input;
							event_loop.set_device_event_strategy(if self.raw_input {
								DeviceEventStrategy::Always
							} else {
								DeviceEventStrategy::Never
							});
							println!("  raw input: {}", self.raw_input);
						}
						_ => {}
					}
				}
				_ => {}
			}
		}

		fn device_event(
			&mut self,
			_event_loop: &ActiveEventLoop,
			device_id: Option<DeviceId>,
			event: DeviceEvent,
		) {
			self.device_events += 1;
			if !matches!(event, DeviceEvent::PointerMotion { .. }) {
				println!(
					"device {:?} ({:?}): {event:?}",
					device_id.map(|id| id.into_raw()),
					device_id.and_then(evloop_win32::event::device_persistent_identifier),
				);
			}
		}
	}

	fn toggle_ime(window: &Window) {
		if window.ime_capabilities().is_some() {
			println!("  disable ime: {:?}", window.request_ime_update(ImeRequest::Disable));
		} else {
			let request = ImeEnableRequest::new(
				ImeCapabilities::new().with_cursor_area(),
				ImeRequestData::default().with_cursor_area(
					Position::Physical(PhysicalPosition::new(20, 20)),
					evloop_core::dpi::Size::Physical(geometry_types::PhysicalSize::new(10, 20)),
				),
			)
			.unwrap();
			println!("  enable ime: {:?}", window.request_ime_update(ImeRequest::Enable(request)));
		}
	}

	pub fn main() {
		let mut attributes = EventLoopAttributes::default();
		let event_loop = EventLoop::new(&mut attributes).expect("failed to create event loop");
		event_loop.run_app(App::default()).expect("event loop failed");
	}
}

fn main() {
	#[cfg(target_os = "windows")]
	app::main();
}
