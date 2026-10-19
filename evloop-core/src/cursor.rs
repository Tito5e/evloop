use core::fmt;
use std::error::Error;

pub const MAX_CURSOR_SIZE: u16 = 2048;

#[derive(Debug)]
pub enum BadImage {
    TooLarge {
        width: u16,
        height: u16,
    },
    ByteCountNotDivisibleBy4 {
        byte_count: usize,
    },
    DimensionsVsPixelCount {
        width: u16,
        height: u16,
        width_x_height: u64,
        pixel_count: u64,
    },
    HotspotOutOfBounds {
        width: u16,
        height: u16,
        hotspot_x: u16,
        hotspot_y: u16,
    },
}

impl fmt::Display for BadImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BadImage::TooLarge { width, height } => write!(
                f,
                "The specified dimensions ({width:?}x{height:?}) are too large. The maximum is \
                {MAX_CURSOR_SIZE:?}x{MAX_CURSOR_SIZE:?}.",
            ),
            BadImage::ByteCountNotDivisibleBy4 { byte_count } => write!(
                f,
                "The length of the `rgba` argument ({byte_count:?} isn't divisible by 4, making \
                it impossible to interpret as 32bpp RGBA pixels."
            ),
            BadImage::DimensionsVsPixelCount {
                width,
                height,
                width_x_height,
                pixel_count,
            } => {
                write!(
                    f,
                    "The specified dimensions ({width:?}x{height:?} don't match the number of \
                    pixels supplied by the `rgba` argument ({pixel_count:?}). For those \
                    dimensions, the expected pixel count is {width_x_height:?}."
                )
            }
            BadImage::HotspotOutOfBounds {
                width,
                height,
                hotspot_x,
                hotspot_y,
            } => write!(
                f,
                "The specified hotspot ({hotspot_x:?}, {hotspot_y:?}) is outside the image bounds \
                ({width:?}x{height:?})."
            ),
        }
    }
}

impl Error for BadImage {}
