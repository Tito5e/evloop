extern crate core;

pub mod capability;
pub mod cursor;
pub mod data_transfer;
pub mod error;
pub mod event;
pub mod event_loop;
pub mod icon;
pub mod ime;
pub mod keyboard;
pub mod monitor;
pub mod window;

pub enum Platform {
	Win32,
	Android,
}
