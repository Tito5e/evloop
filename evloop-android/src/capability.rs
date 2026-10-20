use evloop_core::error::CreateWindowError;
use crate::window::Window;

pub trait CanGetWindow {
    fn get_window(&self) -> Result<Window, CreateWindowError>;
}
