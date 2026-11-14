use core::fmt;
use std::error::Error;

pub const MAX_CURSOR_SIZE: u16 = 2048;
const PIXEL_SIZE: usize = 4;

#[derive(Debug)]
pub enum BadImage {
	TooLarge { width: u16, height: u16, max: u16 },
	ByteCountNotDivisibleBy4 { byte_count: usize },
	DimensionsVsPixelCount { width: u16, height: u16, width_x_height: u64, pixel_count: u64 },
	HotspotOutOfBounds { width: u16, height: u16, hotspot_x: u16, hotspot_y: u16 },
}

impl fmt::Display for BadImage {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			BadImage::TooLarge { width, height, max } => write!(
				f,
				"The specified dimensions ({width:?}x{height:?}) are too large. The maximum is \
                {max:?}x{max:?}.",
			),
			BadImage::ByteCountNotDivisibleBy4 { byte_count } => write!(
				f,
				"The length of the `rgba` argument ({byte_count:?} isn't divisible by 4, making \
                it impossible to interpret as 32bpp RGBA pixels."
			),
			BadImage::DimensionsVsPixelCount { width, height, width_x_height, pixel_count } => {
				write!(
					f,
					"The specified dimensions ({width:?}x{height:?} don't match the number of \
                    pixels supplied by the `rgba` argument ({pixel_count:?}). For those \
                    dimensions, the expected pixel count is {width_x_height:?}."
				)
			}
			BadImage::HotspotOutOfBounds { width, height, hotspot_x, hotspot_y } => write!(
				f,
				"The specified hotspot ({hotspot_x:?}, {hotspot_y:?}) is outside the image bounds \
                ({width:?}x{height:?})."
			),
		}
	}
}

impl Error for BadImage {}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct CursorImage {
	pub(crate) rgba: Vec<u8>,
	pub(crate) width: u16,
	pub(crate) height: u16,
	pub(crate) hotspot_x: u16,
	pub(crate) hotspot_y: u16,
}

/// Validates a 32bpp RGBA image with a hotspot. Shared by [`CursorImage`] and
/// [`DragImage`](crate::resource::DragImage).
pub(crate) fn validate_rgba_with_hotspot(
	rgba: &[u8],
	width: u16,
	height: u16,
	hotspot_x: u16,
	hotspot_y: u16,
	max: u16,
) -> Result<(), BadImage> {
	if width > max || height > max {
		return Err(BadImage::TooLarge { width, height, max });
	}

	if !rgba.len().is_multiple_of(PIXEL_SIZE) {
		return Err(BadImage::ByteCountNotDivisibleBy4 { byte_count: rgba.len() });
	}

	let pixel_count = (rgba.len() / PIXEL_SIZE) as u64;
	let width_x_height = width as u64 * height as u64;
	if pixel_count != width_x_height {
		return Err(BadImage::DimensionsVsPixelCount {
			width,
			height,
			width_x_height,
			pixel_count,
		});
	}

	if hotspot_x >= width || hotspot_y >= height {
		return Err(BadImage::HotspotOutOfBounds { width, height, hotspot_x, hotspot_y });
	}

	Ok(())
}

impl CursorImage {
	/// Creates a cursor image from 32bpp RGBA pixels (row-major, top-left origin).
	pub fn from_rgba(
		rgba: Vec<u8>,
		width: u16,
		height: u16,
		hotspot_x: u16,
		hotspot_y: u16,
	) -> Result<Self, BadImage> {
		validate_rgba_with_hotspot(&rgba, width, height, hotspot_x, hotspot_y, MAX_CURSOR_SIZE)?;
		Ok(CursorImage { rgba, width, height, hotspot_x, hotspot_y })
	}

	pub fn buffer(&self) -> &[u8] {
		self.rgba.as_slice()
	}

	pub fn buffer_mut(&mut self) -> &mut [u8] {
		self.rgba.as_mut_slice()
	}

	pub fn width(&self) -> u16 {
		self.width
	}

	pub fn height(&self) -> u16 {
		self.height
	}

	pub fn hotspot_x(&self) -> u16 {
		self.hotspot_x
	}

	pub fn hotspot_y(&self) -> u16 {
		self.hotspot_y
	}
}
