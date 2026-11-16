//! Forwards the process's stdout / stderr to logcat, so `println!` and panic messages are visible.

use std::{
	ffi::{CStr, CString},
	fs::File,
	io::{BufRead, BufReader},
	os::fd::{FromRawFd, RawFd},
	sync::Once,
};

const TAG: &CStr = c"evloop";
const STDIO_TAG: &CStr = c"RustStdoutStderr";

pub(crate) fn log(priority: ndk_sys::android_LogPriority, message: &str) {
	let message = CString::new(message.replace('\0', "\\0")).unwrap_or_default();
	unsafe { ndk_sys::__android_log_write(priority.0 as _, TAG.as_ptr(), message.as_ptr()) };
}

pub(crate) fn error(message: &str) {
	log(ndk_sys::android_LogPriority::ANDROID_LOG_ERROR, message);
}

/// Starts the forwarding thread once per process.
pub(crate) fn forward_stdio() {
	static ONCE: Once = Once::new();
	ONCE.call_once(|| {
		let file = unsafe {
			let mut pipe: [RawFd; 2] = [0; 2];
			if libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC) != 0 {
				error("failed to create the stdio pipe; stdout/stderr will not reach logcat");
				return;
			}
			libc::dup2(pipe[1], libc::STDOUT_FILENO);
			libc::dup2(pipe[1], libc::STDERR_FILENO);
			libc::close(pipe[1]);
			File::from_raw_fd(pipe[0])
		};

		let spawned = std::thread::Builder::new().name("stdio-to-logcat".into()).spawn(move || {
			let mut reader = BufReader::new(file);
			let mut line = String::new();
			loop {
				line.clear();
				match reader.read_line(&mut line) {
					Ok(0) | Err(_) => break,
					Ok(_) => {
						let text =
							CString::new(line.trim_end().replace('\0', "\\0")).unwrap_or_default();
						unsafe {
							ndk_sys::__android_log_write(
								ndk_sys::android_LogPriority::ANDROID_LOG_INFO.0 as _,
								STDIO_TAG.as_ptr(),
								text.as_ptr(),
							)
						};
					}
				}
			}
		});
		if spawned.is_err() {
			error("failed to start the stdio forwarding thread");
		}
	});
}
