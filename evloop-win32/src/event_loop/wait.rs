use std::{
	os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle},
	ptr,
	time::{Duration, Instant},
};

use evloop_core::event_loop::ControlFlow;
use windows_sys::Win32::{
	Foundation::{GetLastError, HANDLE, WAIT_FAILED},
	System::Threading::{
		CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, INFINITE, SetWaitableTimer,
		TIMER_ALL_ACCESS,
	},
	UI::WindowsAndMessaging::{MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, QS_ALLINPUT},
};

fn min_timeout(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
	a.map_or(b, |a_timeout| b.map_or(Some(a_timeout), |b_timeout| Some(a_timeout.min(b_timeout))))
}

fn dur2timeout(dur: Duration) -> u32 {
	dur.as_secs()
		.checked_mul(1000)
		.and_then(|ms| ms.checked_add((dur.subsec_nanos() as u64) / 1_000_000))
		.and_then(
			|ms| {
				if dur.subsec_nanos() % 1_000_000 > 0 { ms.checked_add(1) } else { Some(ms) }
			},
		)
		.map(|ms| if ms > u32::MAX as u64 { INFINITE } else { ms as u32 })
		.unwrap_or(INFINITE)
}

const FIFTY_DAYS: Duration = Duration::from_secs(50_u64 * 24 * 60 * 60);

const MIN_WAIT: Duration = Duration::from_nanos(100);

fn create_high_resolution_timer() -> Option<OwnedHandle> {
	unsafe {
		let handle: HANDLE = CreateWaitableTimerExW(
			ptr::null(),
			ptr::null(),
			CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
			TIMER_ALL_ACCESS,
		);

		if handle.is_null() { None } else { Some(OwnedHandle::from_raw_handle(handle)) }
	}
}

unsafe fn set_high_resolution_timer(timer: RawHandle, timeout: Duration) -> Result<(), u32> {
	const INTERVAL_NS: u32 = MIN_WAIT.subsec_nanos();
	const INTERVALS_IN_SEC: u64 = (Duration::from_secs(1).as_nanos() / INTERVAL_NS as u128) as u64;
	let intervals_to_wait: u64 =
		timeout.as_secs() * INTERVALS_IN_SEC + u64::from(timeout.subsec_nanos() / INTERVAL_NS);
	debug_assert!(intervals_to_wait < i64::MAX as u64, "Must be called with smaller duration");
	let due_time: i64 = -(intervals_to_wait as i64);
	unsafe {
		let set_result =
			SetWaitableTimer(timer as HANDLE, &due_time, 0, None, ptr::null(), false.into());
		if set_result != false.into() { Ok(()) } else { Err(GetLastError()) }
	}
}

pub(crate) fn wait_for_messages_impl(
	high_resolution_timer: &mut Option<OwnedHandle>,
	control_flow: ControlFlow,
	timeout: Option<Duration>,
) {
	let timeout = {
		let control_flow_timeout = match control_flow {
			ControlFlow::Wait => None,
			ControlFlow::Poll => Some(Duration::ZERO),
			ControlFlow::WaitUntil(wait_deadline) => {
				let start = Instant::now();
				Some(wait_deadline.saturating_duration_since(start))
			}
		};
		let timeout = min_timeout(timeout, control_flow_timeout);
		if timeout == Some(Duration::ZERO) {
			return;
		}

		timeout.map(|t| t.min(FIFTY_DAYS)).map(|t| t.max(MIN_WAIT))
	};

	if timeout.is_some() && high_resolution_timer.is_none() {
		*high_resolution_timer = create_high_resolution_timer();
	}

	let high_resolution_timer: Option<RawHandle> =
		high_resolution_timer.as_ref().map(OwnedHandle::as_raw_handle);

	let use_timer: bool;
	if let (Some(handle), Some(timeout)) = (high_resolution_timer, timeout) {
		let res = unsafe { set_high_resolution_timer(handle, timeout) };
		if let Err(error_code) = res {
			tracing::trace!("Failed to set high resolution timer: last error {}", error_code);
			use_timer = false;
		} else {
			use_timer = true;
		}
	} else {
		use_timer = false;
	}

	unsafe {
		let wait_duration_ms = timeout.map(dur2timeout).unwrap_or(INFINITE);

		let (num_handles, raw_handles) =
			if use_timer { (1, [high_resolution_timer.unwrap()]) } else { (0, [ptr::null_mut()]) };

		let result = MsgWaitForMultipleObjectsEx(
			num_handles,
			raw_handles.as_ptr() as *const _,
			wait_duration_ms,
			QS_ALLINPUT,
			MWMO_INPUTAVAILABLE,
		);
		if result == WAIT_FAILED {
			tracing::warn!("Failed to MsgWaitForMultipleObjectsEx: error code {}", GetLastError());
		}
	}
}
