use crate::geometry::{PhysicalInsets, PhysicalPosition, PhysicalSize};
use crate::keyboard::{Key, KeyLocation, PhysicalKey};
use smol_str::SmolStr;

pub enum WindowEvent {
    ContentRectChanged(PhysicalInsets),
    Focused(bool),
    ScaleFactorChanged(f64),
    Occluded(bool),
    SurfaceResized(PhysicalSize),
    PointerEntered {
        device_id: Option<DeviceId>,
        position: PhysicalPosition,
        primary: bool,
        kind: PointerKind,
    },
    PointerButton {
        device_id: Option<DeviceId>,
        state: ElementState,
        position: PhysicalPosition,
        primary: bool,
        button: ButtonSource,
        is_macos_activation_click: bool,
    },
    PointerMoved {
        device_id: Option<DeviceId>,
        position: PhysicalPosition,
        primary: bool,
        source: PointerSource,
    },
    PointerLeft {
        device_id: Option<DeviceId>,
        position: Option<PhysicalPosition>,
        primary: bool,
        kind: PointerKind,
    },
    KeyboardInput {
        device_id: Option<DeviceId>,
        event: KeyEvent,
        is_synthetic: bool,
    },
}

pub struct KeyEvent {
    pub physical_key: PhysicalKey,
    pub logical_key: Key,
    pub text: Option<SmolStr>,
    pub location: KeyLocation,
    pub state: ElementState,
    pub repeat: bool,
    pub text_with_all_modifiers: Option<SmolStr>,
    pub key_without_modifiers: Key,
}

pub enum PointerKind {
    Mouse,
    Touch(FingerId),
    TabletTool(TabletToolKind),
    Unknown,
}

pub enum PointerSource {
    Mouse,
    Touch {
        finger_id: FingerId,
        force: Option<Force>,
    },
    TabletTool {
        kind: TabletToolKind,
        data: TabletToolData,
    },
    Unknown,
}

pub enum ButtonSource {
    Mouse(MouseButton),
    Touch {
        finger_id: FingerId,
        force: Option<Force>,
    },
    TabletTool {
        kind: TabletToolKind,
        button: TabletToolButton,
        data: TabletToolData,
    },
    Unknown(u16),
}

#[derive(Default)]
pub enum TabletToolKind {
    #[default]
    Pen,
    Eraser,
    Brush,
    Pencil,
    Airbrush,
    Finger,
    Mouse,
    Lens,
}

pub struct TabletToolData {
    pub force: Option<Force>,
    pub tangential_force: Option<f32>,
    pub twist: Option<u16>,
    pub tilt: Option<TabletToolTilt>,
    pub angle: Option<TabletToolAngle>,
}

pub struct TabletToolTilt {
    pub x: i8,
    pub y: i8,
}

pub enum TabletToolButton {
    Contact,
    Barrel,
    Other(u16),
}

pub struct TabletToolAngle {
    pub altitude: f64,
    pub azimuth: f64,
}

pub enum DeviceEvent {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(i64);

impl DeviceId {
    pub const fn into_raw(self) -> i64 {
        self.0
    }

    pub const fn from_raw(id: i64) -> Self {
        Self(id)
    }
}

#[derive(Debug, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FingerId(usize);

impl FingerId {
    pub const fn into_raw(self) -> usize {
        self.0
    }

    pub const fn from_raw(id: usize) -> Self {
        Self(id)
    }
}

pub enum Force {
    Calibrated { force: f64, max_possible_force: f64 },
    Normalized(f64),
}

impl Force {
    pub fn normalized(&self, angle: Option<TabletToolAngle>) -> f64 {
        match self {
            Force::Calibrated {
                force,
                max_possible_force,
            } => {
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

pub enum EvloopEvent {
    ProxyWakeUp,
    MemoryWarning,
}
