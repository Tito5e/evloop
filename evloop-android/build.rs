//! Builds the vendored GameActivity / GameTextInput native sources (Android only).

use std::{env, path::PathBuf};

const SDK: &str = "third_party/games-sdk";

fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed={SDK}");
	for var in
		["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME", "ANDROID_HOME", "ANDROID_PLATFORM"]
	{
		println!("cargo:rerun-if-env-changed={var}");
	}

	if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
		return;
	}

	let target = env::var("TARGET").unwrap();

	let mut build = cc::Build::new();
	build
		.cpp(true)
		.std("c++17")
		.cpp_link_stdlib("c++_static")
		.extra_warnings(false)
		.warnings(false)
		.include(format!("{SDK}/include"))
		.include(format!("{SDK}/src/common"))
		.include(format!("{SDK}/game-activity/include"))
		.include(format!("{SDK}/game-text-input/include"))
		.file(format!("{SDK}/src/common/system_utils.cpp"))
		.file(format!("{SDK}/game-activity/src/game-activity/GameActivity.cpp"))
		.file(format!("{SDK}/game-activity/src/game-activity/GameActivityEvents.cpp"))
		.file(format!("{SDK}/game-text-input/src/game-text-input/gametextinput.cpp"));

	if let Some(toolchain) = NdkToolchain::find(&target) {
		build.compiler(toolchain.clangxx).archiver(toolchain.ar);
	}

	build.compile("evloop_game_activity");

	// The static libc++ also needs its ABI library.
	println!("cargo:rustc-link-lib=c++abi");
	println!("cargo:rustc-link-lib=log");
	println!("cargo:rustc-link-lib=android");
}

struct NdkToolchain {
	clangxx: PathBuf,
	ar: PathBuf,
}

impl NdkToolchain {
	/// Finds the NDK toolchain when the user has not configured the C++ compiler for `target`
	/// (`CXX_<target>` / `TARGET_CXX`, as set by e.g. cargo-ndk).
	///
	/// Looks in `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, `NDK_HOME`, then the newest NDK under
	/// `ANDROID_HOME/ndk`. The API level comes from `ANDROID_PLATFORM` (default 24).
	fn find(target: &str) -> Option<Self> {
		let target_env = target.replace('-', "_");
		if env::var_os(format!("CXX_{target}")).is_some()
			|| env::var_os(format!("CXX_{target_env}")).is_some()
			|| env::var_os("TARGET_CXX").is_some()
		{
			return None;
		}

		let ndk = ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME"]
			.into_iter()
			.filter_map(env::var_os)
			.map(PathBuf::from)
			.find(|p| p.is_dir())
			.or_else(|| {
				let ndk_dir = PathBuf::from(env::var_os("ANDROID_HOME")?).join("ndk");
				let mut versions: Vec<PathBuf> =
					std::fs::read_dir(ndk_dir).ok()?.filter_map(|e| Some(e.ok()?.path())).collect();
				versions.sort();
				versions.pop()
			})?;

		let host = if cfg!(windows) {
			"windows-x86_64"
		} else if cfg!(target_os = "macos") {
			"darwin-x86_64"
		} else {
			"linux-x86_64"
		};
		let bin = ndk.join("toolchains/llvm/prebuilt").join(host).join("bin");

		// armv7 uses a different clang target prefix than the Rust target.
		let clang_target = match target {
			"armv7-linux-androideabi" => "armv7a-linux-androideabi",
			other => other,
		};
		let api = env::var("ANDROID_PLATFORM")
			.ok()
			.and_then(|v| v.trim_start_matches("android-").parse::<u32>().ok())
			.unwrap_or(24);

		let (clang_suffix, exe) =
			if cfg!(windows) { ("-clang++.cmd", ".exe") } else { ("-clang++", "") };
		let candidate = |api: u32| bin.join(format!("{clang_target}{api}{clang_suffix}"));
		let clangxx = (api..=40)
			.map(candidate)
			.chain((21..api).rev().map(candidate))
			.find(|p| p.is_file())?;
		let ar = bin.join(format!("llvm-ar{exe}"));

		Some(Self { clangxx, ar })
	}
}
