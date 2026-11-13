#[cfg(target_os = "android")]
pub use evloop_android::event::*;
pub use evloop_core::event::*;
#[cfg(target_os = "windows")]
pub use evloop_win32::event::*;
