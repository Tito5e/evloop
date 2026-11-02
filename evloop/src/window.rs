pub use evloop_core::window::*;

#[cfg(target_os = "android")]
pub use evloop_android::window::*;

#[cfg(target_os = "windows")]
pub use evloop_win32::window::*;
