use evloop_core::capability::{
    CanExit, ControlFlowAvailable, PlatformAvailable, SystemThemeAvailable,
};

use crate::capability::CanGetWindow;

pub trait Started:
    CanExit + ControlFlowAvailable + SystemThemeAvailable + PlatformAvailable
{
}

pub trait Stopped: PlatformAvailable {}

pub trait SurfaceAvailable:
    CanGetWindow + CanExit + ControlFlowAvailable + SystemThemeAvailable + PlatformAvailable
{
}

pub trait SurfaceLost:
    CanExit + ControlFlowAvailable + SystemThemeAvailable + PlatformAvailable
{
}

pub trait Resumed:
    CanExit + ControlFlowAvailable + SystemThemeAvailable + PlatformAvailable
{
}

pub trait Paused:
    CanExit + ControlFlowAvailable + SystemThemeAvailable + PlatformAvailable
{
}

pub trait Running:
    CanExit + ControlFlowAvailable + SystemThemeAvailable + PlatformAvailable
{
}
