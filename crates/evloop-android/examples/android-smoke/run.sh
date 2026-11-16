#!/usr/bin/env bash
# Builds the Rust smoke example, packages it, installs it on the connected device or emulator,
# and starts it. Logs: `adb logcat -s RustStdoutStderr evloop`.
#
# Environment:
#   TARGET       Rust target (default: x86_64-linux-android, for the emulator)
#   NDK_HOME     Android NDK (or ANDROID_NDK_HOME); used for the linker
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
target="${TARGET:-x86_64-linux-android}"
case "$target" in
  aarch64-linux-android) abi=arm64-v8a ;;
  armv7-linux-androideabi) abi=armeabi-v7a; clang_target=armv7a-linux-androideabi ;;
  i686-linux-android) abi=x86 ;;
  x86_64-linux-android) abi=x86_64 ;;
  *) echo "unsupported target $target" >&2; exit 1 ;;
esac
clang_target="${clang_target:-$target}"

ndk="${ANDROID_NDK_HOME:-${NDK_HOME:?set NDK_HOME or ANDROID_NDK_HOME}}"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) host=windows-x86_64; ext=.cmd ;;
  Darwin) host=darwin-x86_64; ext= ;;
  *) host=linux-x86_64; ext= ;;
esac
linker="$ndk/toolchains/llvm/prebuilt/$host/bin/${clang_target}24-clang$ext"
env_name="CARGO_TARGET_$(echo "$target" | tr 'a-z-' 'A-Z_')_LINKER"
export "$env_name=$linker"

cargo build -p evloop-android --example smoke --target "$target"

out="$here/app/build/rustJniLibs/$abi"
mkdir -p "$out"
cp "$here/../../../target/$target/debug/examples/libsmoke.so" "$out/"

(cd "$here" && gradle --quiet :app:installDebug)
adb shell am start -n dev.evloop.smoke/.MainActivity
