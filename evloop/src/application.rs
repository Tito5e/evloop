#[cfg(target_os = "android")]
pub use evloop_android::application::*;

#[cfg(target_os = "windows")]
pub use evloop_win32::application::*;
