use crate::cursor::{BadImage, validate_rgba_with_hotspot};

/// The largest width or height accepted for a [`DragImage`].
pub const MAX_DRAG_IMAGE_SIZE: u16 = 2048;

/// The image shown under the pointer while an outgoing drag is in progress.
///
/// 32bpp RGBA pixels (row-major, top-left origin). The hotspot is the pixel that sits under the
/// pointer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DragImage {
	rgba: Vec<u8>,
	width: u16,
	height: u16,
	hotspot_x: u16,
	hotspot_y: u16,
}

impl DragImage {
	pub fn from_rgba(
		rgba: Vec<u8>,
		width: u16,
		height: u16,
		hotspot_x: u16,
		hotspot_y: u16,
	) -> Result<Self, BadImage> {
		validate_rgba_with_hotspot(
			&rgba,
			width,
			height,
			hotspot_x,
			hotspot_y,
			MAX_DRAG_IMAGE_SIZE,
		)?;
		Ok(Self { rgba, width, height, hotspot_x, hotspot_y })
	}

	#[inline]
	pub fn buffer(&self) -> &[u8] {
		&self.rgba
	}

	#[inline]
	pub fn width(&self) -> u16 {
		self.width
	}

	#[inline]
	pub fn height(&self) -> u16 {
		self.height
	}

	#[inline]
	pub fn hotspot_x(&self) -> u16 {
		self.hotspot_x
	}

	#[inline]
	pub fn hotspot_y(&self) -> u16 {
		self.hotspot_y
	}
}
