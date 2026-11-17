use std::{
    mem, ptr,
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        LazyLock,
    },
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{RedrawWindow, ValidateRect, RDW_INTERNALPAINT},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, RegisterClassExW, RegisterWindowMessageA, GWL_STYLE,
        GWL_USERDATA, WM_INPUT, WM_NCDESTROY, WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
    },
};

use crate::{
    encoding::encode_wide, event_loop::EventLoopRunner, get_instance_handle, get_window_long,
    set_window_long,
};

type ExecuteFn = Box<Box<dyn FnMut()>>;

const INVALID_MESSAGE_ID: u32 = 0x0;

pub struct LazyMessageId {
    id: AtomicU32,
    name: &'static str,
}

impl LazyMessageId {
    const fn new(name: &'static str) -> Self {
        // 作成時にはINVALID_MESSAGE_ID(初期値)で埋めておく
        Self {
            id: AtomicU32::new(INVALID_MESSAGE_ID),
            name,
        }
    }

    pub fn get(&self) -> u32 {
        let id = self.id.load(Ordering::Relaxed);

        // 初期化済みならそのidを返す
        if id != INVALID_MESSAGE_ID {
            return id;
        }

        // メッセージの名前は必ずCStringの形式である必要がある
        assert!(self.name.ends_with('\0'));

        // evloopの内部実装では絶対にASCII文字しか使用しないので、メッセージの名前をWide APIで登録する必要がない
        let new_id = unsafe { RegisterWindowMessageA(self.name.as_ptr()) };

        assert_ne!(
            new_id,
            0,
            "RegisterWindowMessageA returned 0 for '{}': {}",
            self.name,
            std::io::Error::last_os_error()
        );

        self.id.store(new_id, Ordering::Relaxed);

        new_id
    }
}

static MESSAGE_WINDOW_CLASS: LazyLock<Vec<u16>> =
    LazyLock::new(|| encode_wide("evloop message window"));

static EVLOOP_EXECUTE_MSG_ID: LazyMessageId = LazyMessageId::new("Evloop::ExecuteMsg\0");

pub(super) fn create_message_window() -> HWND {
    let class = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(message_window_callback),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: get_instance_handle(),
        hIcon: ptr::null_mut(),
        hCursor: ptr::null_mut(),
        hbrBackground: ptr::null_mut(),
        lpszMenuName: ptr::null(),
        lpszClassName: MESSAGE_WINDOW_CLASS.as_ptr(),
        hIconSm: ptr::null_mut(),
    };

    unsafe { RegisterClassExW(&class) };

    let message_window = unsafe {
        CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            MESSAGE_WINDOW_CLASS.as_ptr(),
            ptr::null(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            get_instance_handle(),
            ptr::null(),
        )
    };

    unsafe { set_window_long(message_window, GWL_STYLE, (WS_VISIBLE | WS_POPUP) as isize) };

    message_window
}

unsafe extern "system" fn message_window_callback(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = unsafe { get_window_long(window, GWL_USERDATA) } as *mut MessageWindowState;
    if state_ptr.is_null() {
        // ステートが取得できなかった場合、Windowsのデフォルト処理に移行する
        return unsafe { DefWindowProcW(window, msg, wparam, lparam) };
    }
    let state = unsafe { Box::from_raw(state_ptr) };

    // WM_PAINTがまとめられて最後にDispatchされる性質を利用し、イベントの処理が終わったら確実にRedrawが実行されるように再描画をあらかじめ予約する
    if msg != WM_PAINT {
        unsafe { RedrawWindow(window, ptr::null(), ptr::null_mut(), RDW_INTERNALPAINT) };
    }

    let mut state_removed = false;

    let callback = || match msg {
        WM_NCDESTROY => {
            // イベントループが終了するのでステートをアンマウントする
            unsafe { set_window_long(window, GWL_USERDATA, 0) };
            state_removed = true;
            0
        }
        WM_PAINT => unsafe {
            ValidateRect(window, ptr::null());
            DefWindowProcW(window, msg, wparam, lparam)
        },
        WM_INPUT => {
            // raw inputを処理する
            if let Some(raw_input) = get_raw_input_data(lparam as _) {
                unsafe { handle_raw_input(&state, data) };
            }

            unsafe { DefWindowProcW(window, msg, wparam, lparam) }
        }

        _ if msg == EVLOOP_EXECUTE_MSG_ID.get() => {
            let mut function: ExecuteFn = unsafe { Box::from_raw(wparam as *mut _) };
            function();
            0
        }
        _ => unsafe { DefWindowProcW(window, msg, wparam, lparam) },
    };

    let result = state.runner.catch_unwind(callback).unwrap_or(-1);

    if state_removed {
        // イベントループが破棄された場合、そのままステートを破棄する
        drop(state);
    } else {
        // イベントループが継続する場合、ステートをそのままリークする(Dropしない)
        Box::leak(state);
    }

    result
}

struct MessageWindowState {
    runner: Rc<EventLoopRunner>,
}
