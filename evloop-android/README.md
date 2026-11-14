# evloop-android

The Android backend of evloop, built on
[GameActivity](https://developer.android.com/games/agdk/game-activity) (AGDK).

evloop-android does not use the `android-activity` crate: it compiles the GameActivity /
GameTextInput native sources itself (`third_party/games-sdk`) and bridges the Java main thread and
the application thread in Rust (`src/glue`).

## Application setup

An evloop Android application is a Rust `cdylib` loaded by a Gradle project.

### Rust

```toml
[lib]
crate-type = ["cdylib"]
```

```rust
use evloop::platform::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let event_loop = evloop::event_loop::EventLoop::builder().build(app).unwrap();
    event_loop.run_app(MyApp::default()).unwrap();
}
```

`android_main` runs on its own thread and is called once per activity instance: when the activity
is destroyed (for example on a configuration change without `android:configChanges`) the event loop
exits, `android_main` returns, and it is called again for the new activity.

### Gradle

The application provides its own activity and manifest. evloop requires the activity to be a
`GameActivity` (or a subclass). Include the `java/` module of this crate, which brings GameActivity
in the version the native side is built against:

```kotlin
// settings.gradle.kts
include(":evloop")
project(":evloop").projectDir = file("<path to evloop-android>/java")

// app/build.gradle.kts
dependencies { implementation(project(":evloop")) }
```

```java
public class MainActivity extends com.google.androidgamesdk.GameActivity {}
```

```xml
<activity android:name=".MainActivity" android:exported="true">
    <meta-data android:name="android.app.lib_name" android:value="<your library name>" />
    ...
</activity>
```

The `java/` module depends on `androidx.games:games-activity:4.4.0`. **This exact version is
required**: the native sources in `third_party/games-sdk` are bound to its Java classes through JNI.

Put the built `lib<name>.so` in the application's `jniLibs/<abi>/`.

### Building the native library

The build script compiles the C++ sources with the NDK's clang. It uses `CXX_<target>` if set
(as `cargo-ndk` does); otherwise it finds the NDK in `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`,
`NDK_HOME`, or the newest one under `ANDROID_HOME/ndk`. The minimum API level is taken from
`ANDROID_PLATFORM` (default 24).

Linking still needs the NDK linker for the target, e.g.:

```sh
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_HOME/toolchains/llvm/prebuilt/<host>/bin/aarch64-linux-android24-clang"
cargo build --target aarch64-linux-android
```

(or use `cargo ndk -t arm64-v8a build`).

## Behavior notes

- stdout and stderr (including panic messages) are forwarded to logcat with the tag
  `RustStdoutStderr`; the glue logs with the tag `evloop`.
- Volume, camera and zoom keys are left to the system. All other keys, including Back, are
  delivered to the application.
- Pointer input is accepted from touchscreens, mice, styluses and touchpads.
- `ndk-context` is initialized with the `Application` context, for crates that rely on it.
- Saved instance state is not supported.
- **Safe area**: `Window::safe_area` is computed from the window insets and assumes the activity
  is laid out edge-to-edge (the surface extends behind the system bars), as enforced on
  Android 15+ for apps targeting SDK 35+. Other layouts are not supported yet.
- **IME**: the soft keyboard's Backspace usually edits the IME's buffer and arrives as
  `Ime::DeleteSurrounding`. Once that buffer is empty it arrives as a `KeyboardInput` with
  `NamedKey::Backspace`; in that case GameTextInput only forwards the key *release*.

## Example

`examples/smoke.rs` with the Gradle project in `examples/android-smoke` logs every event:

```sh
NDK_HOME=... evloop-android/examples/android-smoke/run.sh   # emulator (x86_64) by default
adb logcat -s RustStdoutStderr evloop
```
