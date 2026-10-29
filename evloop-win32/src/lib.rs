#![cfg(target_os = "windows")] // FIXME(Tito5e): Hack for allow compiling on all platforms.
#![warn(clippy::exhaustive_enums)]

#[macro_use]
mod sys;
mod application;
mod data_transfer;
mod event_loop;
mod monitor;
mod resource;
mod window;
