#[derive(Clone, Debug)]
pub struct WindowAttributes {}

impl Default for WindowAttributes {
	fn default() -> Self {
		Self {}
	}
}

unsafe impl Send for WindowAttributes {}

unsafe impl Sync for WindowAttributes {}

impl WindowAttributes {}
