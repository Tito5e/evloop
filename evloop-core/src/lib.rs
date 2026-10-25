extern crate core;

pub mod capability;
pub mod cursor;
pub mod data_transfer;
pub mod dpi;
pub mod error;
pub mod event;
pub mod event_loop;
pub mod ime;
pub mod keyboard;
pub mod monitor;
pub mod resource;
pub mod window;

pub enum Platform {
	Win32,
	Android,
}
