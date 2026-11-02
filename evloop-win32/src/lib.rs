#![cfg(target_os = "windows")] // FIXME(Tito5e): Hack for allow compiling on all platforms.
#![warn(clippy::exhaustive_enums)]

#[macro_use]
mod sys;
pub mod application;
//mod data_transfer;
pub mod event;
pub mod event_loop;
mod input;
pub mod keyboard;
mod monitor;
mod resource;
mod temporary_mod;
pub mod window;
