extern crate core;

pub mod capability;
pub mod cursor;
pub mod error;
pub mod event;
pub mod event_loop;
pub mod geometry;
pub mod ime;
pub mod keyboard;
pub mod window;

pub enum Platform {
    Win32,
    Android,
}
