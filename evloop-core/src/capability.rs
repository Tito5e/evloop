use crate::{Platform, event_loop::DeviceEventStrategy};
use crate::event_loop::ControlFlow;
use crate::window::Theme;

pub trait CanCreateWindow {}

pub trait CanExit {
    fn exit(&self);
    fn exiting(&self) -> bool;
}

pub trait ControlFlowAvailable {
    fn set_control_flow(&self, control_flow: ControlFlow);
    fn control_flow(&self) -> ControlFlow;
}

/*
TODO: move to evloop-web

pub trait PollStrategyAvailable {
    fn set_poll_strategy(&self, strategy: PollStrategy);
    fn poll_strategy(&self) -> PollStrategy;
}
*/

pub trait MonitorAvailable {
    type MonitorHandle;

    fn available_monitors(&self) -> impl Iterator<Item = Self::MonitorHandle>;

    fn primary_monitor(&self) -> Option<Self::MonitorHandle>;
}

pub trait DeviceEventAvailable {
    fn set_device_event_strategy(&self, strategy: DeviceEventStrategy);
}

pub trait SystemThemeAvailable {
    fn system_theme(&self) -> Option<Theme>;
}

pub trait CustomCursorAvailable {
    type Source;
    type Cursor;

    fn create_custom_cursor(&self, custom_cursor: Self::Source) -> Self::Cursor;
}

pub trait AsyncCustomCursorAvailable {
    type Source;
    type Cursor;

    fn create_custom_cursor_async(&self, custom_cursor: Self::Source) -> Self::Cursor;
}

pub trait PlatformAvailable {
    fn platform(&self) -> Platform;
}
