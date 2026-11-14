//! The Android backend of evloop (GameActivity).
//!
//! Everything except [`text`] only exists on Android. `text` is pure std code and is also built on
//! other hosts so that it can be unit-tested there.

mod text;

#[cfg(target_os = "android")]
pub mod application;
#[cfg(target_os = "android")]
pub mod event;
#[cfg(target_os = "android")]
pub mod event_loop;
#[cfg(target_os = "android")]
mod glue;
#[cfg(target_os = "android")]
mod ime;
#[cfg(target_os = "android")]
pub(crate) mod keycodes;
#[cfg(target_os = "android")]
pub mod platform;
#[cfg(target_os = "android")]
mod shared;
#[cfg(target_os = "android")]
mod sys;
#[cfg(target_os = "android")]
pub mod window;

#[cfg(target_os = "android")]
pub(crate) use shared::*;
