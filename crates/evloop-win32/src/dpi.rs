use std::sync::{LazyLock, Once};

use windows_sys::{
    core::{BOOL, HRESULT},
    Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, PROCESS_DPI_AWARENESS,
        PROCESS_PER_MONITOR_DPI_AWARE,
    },
};

type SetProcessDpiAwarenessContext =
    unsafe extern "system" fn(value: DPI_AWARENESS_CONTEXT) -> BOOL;

type SetProcessDpiAwareness = unsafe extern "system" fn(value: PROCESS_DPI_AWARENESS) -> HRESULT;

type SetProcessDPIAware = unsafe extern "system" fn() -> BOOL;

static SET_PROCESS_DPI_AWARENESS_CONTEXT: LazyLock<Option<SetProcessDpiAwarenessContext>> =
    LazyLock::new(|| get_function!("user32.dll", SetProcessDpiAwarenessContext));

static SET_PROCESS_DPI_AWARENESS: LazyLock<Option<SetProcessDpiAwareness>> =
    LazyLock::new(|| get_function!("shcore.dll", SetProcessDpiAwareness));

static SET_PROCESS_DPI_AWARE: LazyLock<Option<SetProcessDPIAware>> =
    LazyLock::new(|| get_function!("user32.dll", SetProcessDPIAware));

pub fn become_api_aware() {
    static ENABLE_DPI_AWARENESS: Once = Once::new();
    ENABLE_DPI_AWARENESS.call_once(|| {
        unsafe {
            if let Some(set_process_dpi_awareness_context) = *SET_PROCESS_DPI_AWARENESS_CONTEXT {
                if set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
                    == false.into()
                {
                    set_process_dpi_awareness_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
                }
            } else if let Some(set_process_dpi_awareness) = *SET_PROCESS_DPI_AWARENESS {
                set_process_dpi_awareness(PROCESS_PER_MONITOR_DPI_AWARE);
            } else if let Some(set_process_dpi_aware) = *SET_PROCESS_DPI_AWARE {
                // evloop-win32はWin10以前のバージョンはサポートしない方針だが、わざわざ消すほどでもないので残す
                set_process_dpi_aware();
            }
        }
    });
}
