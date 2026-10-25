use core::fmt;
use std::{error::Error, io, mem};

pub(crate) const PIXEL_SIZE: usize = mem::size_of::<u32>();

#[derive(Debug)]
/// An error produced when using [`RgbaIcon::new`] with invalid arguments.
#[non_exhaustive]
pub enum BadIcon {
	/// Produced when the length of the `rgba` argument isn't divisible by 4, thus `rgba` can't be
	/// safely interpreted as 32bpp RGBA pixels.
	ByteCountNotDivisibleBy4 { byte_count: usize },
	/// Produced when the number of pixels (`rgba.len() / 4`) isn't equal to `width * height`.
	/// At least one of your arguments is incorrect.
	DimensionsVsPixelCount { width: u32, height: u32, width_x_height: usize, pixel_count: usize },
	/// Produced when underlying OS functionality failed to create the icon
	OsError(io::Error),
}

impl fmt::Display for BadIcon {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			BadIcon::ByteCountNotDivisibleBy4 { byte_count } => write!(
				f,
				"The length of the `rgba` argument ({byte_count:?}) isn't divisible by 4, making \
                 it impossible to interpret as 32bpp RGBA pixels.",
			),
			BadIcon::DimensionsVsPixelCount { width, height, width_x_height, pixel_count } => {
				write!(
					f,
					"The specified dimensions ({width:?}x{height:?}) don't match the number of \
                     pixels supplied by the `rgba` argument ({pixel_count:?}). For those \
                     dimensions, the expected pixel count is {width_x_height:?}.",
				)
			}
			BadIcon::OsError(e) => write!(f, "OS error when instantiating the icon: {e:?}"),
		}
	}
}

impl Error for BadIcon {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RgbaIcon {
	pub(crate) width: u32,
	pub(crate) height: u32,
	pub(crate) rgba: Vec<u8>,
}

impl RgbaIcon {
	pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
		if !rgba.len().is_multiple_of(PIXEL_SIZE) {
			return Err(BadIcon::ByteCountNotDivisibleBy4 { byte_count: rgba.len() });
		}
		let pixel_count = rgba.len() / PIXEL_SIZE;
		if pixel_count != (width * height) as usize {
			Err(BadIcon::DimensionsVsPixelCount {
				width,
				height,
				width_x_height: (width * height) as usize,
				pixel_count,
			})
		} else {
			Ok(RgbaIcon { rgba, width, height })
		}
	}

	pub fn width(&self) -> u32 {
		self.width
	}

	pub fn height(&self) -> u32 {
		self.height
	}

	pub fn buffer(&self) -> &[u8] {
		self.rgba.as_slice()
	}
}
