pub use evloop_core::event_loop::*;

#[cfg(target_os = "windows")]
pub use evloop_win32::event_loop::*;

#[cfg(target_os = "android")]
pub use evloop_android::event_loop::*;
