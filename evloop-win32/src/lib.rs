#![cfg(target_os = "windows")] // FIXME(Tito5e): Hack for allow compiling on all platforms.
#![warn(clippy::exhaustive_enums)]

#[macro_use]
mod sys;
mod data_transfer;
mod monitor;
mod resource;
mod window;
