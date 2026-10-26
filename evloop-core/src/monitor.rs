use core::fmt;
use std::{
	borrow::Cow,
	num::{NonZeroU16, NonZeroU32},
};

use geometry_types::{PhysicalPosition, PhysicalSize};

pub trait MonitorHandle: fmt::Debug + Send + Sync {
	fn id(&self) -> u128;

	fn native_id(&self) -> u64;

	fn name(&self) -> Option<Cow<'_, str>>;

	fn position(&self) -> Option<PhysicalPosition>;

	fn scale_factor(&self) -> f64;

	fn current_video_mode(&self) -> Option<VideoMode>;

	fn video_modes(&self) -> impl Iterator<Item = VideoMode>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VideoMode {
	size: PhysicalSize,
	bit_depth: Option<NonZeroU16>,
	refresh_rate_millihertz: Option<NonZeroU32>,
}

impl VideoMode {
	pub const fn new(
		size: PhysicalSize,
		bit_depth: Option<NonZeroU16>,
		refresh_rate_millihertz: Option<NonZeroU32>,
	) -> Self {
		Self { size, bit_depth, refresh_rate_millihertz }
	}

	pub const fn size(&self) -> PhysicalSize {
		self.size
	}

	pub const fn bit_depth(&self) -> Option<NonZeroU16> {
		self.bit_depth
	}

	pub const fn refresh_rate_millihertz(&self) -> Option<NonZeroU32> {
		self.refresh_rate_millihertz
	}
}

impl fmt::Display for VideoMode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}x{} {}{}",
			self.size.width,
			self.size.height,
			self.refresh_rate_millihertz.map(|rate| format!("@ {rate} mHz")).unwrap_or_default(),
			self.bit_depth.map(|bit_depth| format!("({bit_depth} bpp)")).unwrap_or_default()
		)
	}
}
