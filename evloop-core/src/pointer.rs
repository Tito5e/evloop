use std::{cell::LazyCell, cmp::Ordering, mem::transmute};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerKind {
	Mouse,
	Touch(FingerId),
	TabletTool(TabletToolKind),
	Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PointerSource {
	Mouse,
	Touch { finger_id: FingerId, force: Option<Force> },
	TabletTool { kind: TabletToolKind, data: TabletToolData },
	Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ButtonSource {
	Mouse(MouseButton),
	Touch { fniger_id: FingerId, force: Option<Force> },
	TabletTool { kind: TabletToolKind, button: TabletToolButton, data: TabletToolData },
	Unknown(u16),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FingerId(usize);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum TouchPhase {
	Started,
	Moved,
	Ended,
	Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Force {
	Calibrated { force: f64, max_possible_force: f64 },
	Normalized(f64),
}

impl Force {
	pub fn normalized(&self, angle: Option<TabletToolAngle>) -> f64 {
		match self {
			Force::Calibrated { force, max_possible_force } => {
				let force = match angle {
					Some(TabletToolAngle { altitude, .. }) => force / altitude.sin(),
					None => *force,
				};
				force / max_possible_force
			}
			Force::Normalized(force) => *force,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabletToolKind {
	Pen,
	Eraser,
	Brush,
	Pencil,
	Airbrush,
	Finger,
	Mouse,
	Lens,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub struct TabletToolData {
	pub force: Option<Force>,
	pub tangential_force: Option<f32>,
	pub twist: Option<u16>,
	pub tilt: Option<TabletToolOrientation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabletToolOrientation {
	Tilt(TabletToolTilt),
	Angle(TabletToolAngle),
}

impl TabletToolOrientation {
	pub fn tilt(&self) -> TabletToolTilt {
		match self {
			Self::Tilt(tilt) => *tilt,
			Self::Angle(angle) => angle.tilt(),
		}
	}

	pub fn angle(&self) -> TabletToolAngle {
		match self {
			Self::Tilt(tilt) => tilt.angle(),
			Self::Angle(angle) => *angle,
		}
	}
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TabletToolTilt {
	pub x: i8,
	pub y: i8,
}

impl TabletToolTilt {
	pub fn angle(&self) -> TabletToolAngle {
		use std::f64::consts::*;

		const PI_0_5: f64 = FRAC_PI_2;
		const PI_1_5: f64 = 3. * FRAC_PI_2;
		const PI_2: f64 = 2. * PI;

		let x = LazyCell::new(|| f64::from(self.x).to_radians());
		let y = LazyCell::new(|| f64::from(self.y).to_radians());

		let mut azimuth = 0.;

		if self.x == 0 {
			match self.y.cmp(&0) {
				Ordering::Greater => azimuth = PI_0_5,
				Ordering::Less => azimuth = PI_1_5,
				Ordering::Equal => (),
			}
		} else if self.y == 0 {
			if self.x < 0 {
				azimuth = PI;
			}
		} else if self.x.abs() == 90 || self.y.abs() == 90 {
			azimuth = 0.;
		} else {
			azimuth = f64::atan2(y.tan(), x.tan());

			if azimuth < 0. {
				azimuth += PI_2;
			}
		}

		let altitude = if self.x.abs() == 90 || self.y.abs() == 90 {
			0.
		} else if self.x == 0 {
			PI_0_5 - y.abs()
		} else if self.y == 0 {
			PI_0_5 - x.abs()
		} else {
			f64::atan(1. / f64::sqrt(x.tan().powi(2) + y.tan().powi(2)))
		};

		TabletToolAngle { altitude, azimuth }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TabletToolAngle {
	pub altitude: f64,
	pub azimuth: f64,
}

impl TabletToolAngle {
	pub fn tilt(&self) -> TabletToolTilt {
		use std::f64::consts::*;

		const PI_0_5: f64 = FRAC_PI_2;
		const PI_1_5: f64 = 3. * FRAC_PI_2;
		const PI_2: f64 = 2. * PI;

		let mut x = 0.;
		let mut y = 0.;

		if self.altitude == 0. {
			if self.azimuth == 0. || self.azimuth == PI_2 {
				x = FRAC_PI_2;
			} else if self.azimuth == PI_0_5 {
				y = FRAC_PI_2;
			} else if self.azimuth == PI {
				x = -FRAC_PI_2;
			} else if self.azimuth == PI_1_5 {
				y = -FRAC_PI_2;
			} else if self.azimuth > 0. && self.azimuth < PI_0_5 {
				x = FRAC_PI_2;
				y = FRAC_PI_2;
			} else if self.azimuth > PI_0_5 && self.azimuth < PI {
				x = -FRAC_PI_2;
				y = FRAC_PI_2;
			} else if self.azimuth > PI && self.azimuth < PI_1_5 {
				x = -FRAC_PI_2;
				y = -FRAC_PI_2;
			} else if self.azimuth > PI_1_5 && self.azimuth < PI_2 {
				x = FRAC_PI_2;
				y = -FRAC_PI_2;
			}
		}

		if self.altitude != 0. {
			let altitude = self.altitude.tan();

			x = f64::atan(f64::cos(self.azimuth) / altitude);
			y = f64::atan(f64::sin(self.azimuth) / altitude);
		}

		TabletToolTilt { x: x.to_degrees().round() as i8, y: y.to_degrees().round() as i8 }
	}
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(u8)]
pub enum MouseButton {
	Left = 0,
	Right = 1,
	Middle = 2,
	Back = 3,
	Forward = 4,
	Button6 = 5,
	Button7 = 6,
	Button8 = 7,
	Button9 = 8,
	Button10 = 9,
	Button11 = 10,
	Button12 = 11,
	Button13 = 12,
	Button14 = 13,
	Button15 = 14,
	Button16 = 15,
	Button17 = 16,
	Button18 = 17,
	Button19 = 18,
	Button20 = 19,
	Button21 = 20,
	Button22 = 21,
	Button23 = 22,
	Button24 = 23,
	Button25 = 24,
	Button26 = 25,
	Button27 = 26,
	Button28 = 27,
	Button29 = 28,
	Button30 = 29,
	Button31 = 30,
	Button32 = 31,
}

impl MouseButton {
	pub fn try_from_u8(raw: u8) -> Option<MouseButton> {
		if raw > 31 { None } else { Some(unsafe { transmute::<u8, MouseButton>(raw) }) }
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TabletToolButton {
	Contact,
	Barrel,
	Other(u16),
}

#[cfg(test)]
mod tests {
	use crate::pointer;

	#[test]
	fn test_tilt_angle_conversions() {
		use std::f64::consts::*;

		use pointer::{TabletToolAngle, TabletToolTilt};

		// See <https://github.com/web-platform-tests/wpt/blob/5af3e9c2a2aba76ade00f0dbc3486e50a74a4506/pointerevents/pointerevent_tiltX_tiltY_to_azimuth_altitude.html#L11-L23>.
		const TILT_TO_ANGLE: &[(TabletToolTilt, TabletToolAngle)] = &[
			(TabletToolTilt { x: 0, y: 0 }, TabletToolAngle { altitude: FRAC_PI_2, azimuth: 0. }),
			(TabletToolTilt { x: 0, y: 90 }, TabletToolAngle { altitude: 0., azimuth: FRAC_PI_2 }),
			(
				TabletToolTilt { x: 0, y: -90 },
				TabletToolAngle { altitude: 0., azimuth: 3. * FRAC_PI_2 },
			),
			(TabletToolTilt { x: 90, y: 0 }, TabletToolAngle { altitude: 0., azimuth: 0. }),
			(TabletToolTilt { x: 90, y: 90 }, TabletToolAngle { altitude: 0., azimuth: 0. }),
			(TabletToolTilt { x: 90, y: -90 }, TabletToolAngle { altitude: 0., azimuth: 0. }),
			(TabletToolTilt { x: -90, y: 0 }, TabletToolAngle { altitude: 0., azimuth: PI }),
			(TabletToolTilt { x: -90, y: 90 }, TabletToolAngle { altitude: 0., azimuth: 0. }),
			(TabletToolTilt { x: -90, y: -90 }, TabletToolAngle { altitude: 0., azimuth: 0. }),
			(
				TabletToolTilt { x: 0, y: 45 },
				TabletToolAngle { altitude: FRAC_PI_4, azimuth: FRAC_PI_2 },
			),
			(
				TabletToolTilt { x: 0, y: -45 },
				TabletToolAngle { altitude: FRAC_PI_4, azimuth: 3. * FRAC_PI_2 },
			),
			(TabletToolTilt { x: 45, y: 0 }, TabletToolAngle { altitude: FRAC_PI_4, azimuth: 0. }),
			(TabletToolTilt { x: -45, y: 0 }, TabletToolAngle { altitude: FRAC_PI_4, azimuth: PI }),
		];

		for (tilt, angle) in TILT_TO_ANGLE {
			assert_eq!(tilt.angle(), *angle, "{tilt:?}");
		}

		// See <https://github.com/web-platform-tests/wpt/blob/5af3e9c2a2aba76ade00f0dbc3486e50a74a4506/pointerevents/pointerevent_tiltX_tiltY_to_azimuth_altitude.html#L38-L46>.
		const ANGLE_TO_TILT: &[(TabletToolAngle, TabletToolTilt)] = &[
			(TabletToolAngle { altitude: 0., azimuth: 0. }, TabletToolTilt { x: 90, y: 0 }),
			(TabletToolAngle { altitude: FRAC_PI_4, azimuth: 0. }, TabletToolTilt { x: 45, y: 0 }),
			(TabletToolAngle { altitude: FRAC_PI_2, azimuth: 0. }, TabletToolTilt { x: 0, y: 0 }),
			(TabletToolAngle { altitude: 0., azimuth: FRAC_PI_2 }, TabletToolTilt { x: 0, y: 90 }),
			(
				TabletToolAngle { altitude: FRAC_PI_4, azimuth: FRAC_PI_2 },
				TabletToolTilt { x: 0, y: 45 },
			),
			(TabletToolAngle { altitude: 0., azimuth: PI }, TabletToolTilt { x: -90, y: 0 }),
			(TabletToolAngle { altitude: FRAC_PI_4, azimuth: PI }, TabletToolTilt { x: -45, y: 0 }),
			(
				TabletToolAngle { altitude: 0., azimuth: 3. * FRAC_PI_2 },
				TabletToolTilt { x: 0, y: -90 },
			),
			(
				TabletToolAngle { altitude: FRAC_PI_4, azimuth: 3. * FRAC_PI_2 },
				TabletToolTilt { x: 0, y: -45 },
			),
		];

		for (angle, tilt) in ANGLE_TO_TILT {
			assert_eq!(angle.tilt(), *tilt, "{angle:?}");
		}
	}

	#[test]
	fn test_force_normalize() {
		let force = pointer::Force::Normalized(0.0);
		assert_eq!(force.normalized(None), 0.0);

		let force2 = pointer::Force::Calibrated { force: 5.0, max_possible_force: 2.5 };
		assert_eq!(force2.normalized(None), 2.0);

		let force3 = pointer::Force::Calibrated { force: 5.0, max_possible_force: 2.5 };
		assert_eq!(
			force3.normalized(Some(pointer::TabletToolAngle {
				altitude: std::f64::consts::PI / 2.0,
				azimuth: 0.
			})),
			2.0
		);
	}
}
