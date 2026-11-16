# Vendored GameActivity / GameTextInput native sources

Native ("prefab") sources of `GameActivity` and `GameTextInput` from the Android Game Development
Kit (AGDK), as imported by [`android-activity` 0.6.1](https://crates.io/crates/android-activity)
from the [rust-mobile fork of android-games-sdk](https://github.com/rust-mobile/android-games-sdk).

- GameActivity: 4.4.0 (`GAMEACTIVITY_VERSION_REVISION a0e943c3a84fd7f344c3d36cdf4e88fd595f81b8`)
- GameTextInput: 4.3.0 (`GAMETEXTINPUT_VERSION_REVISION 7cd950d0022d01f1e7e2b470aba5a7b1abacdfaa`)

The Java side comes from the `androidx.games:games-activity:4.4.0` AAR. The native sources here
are bound to that AAR through JNI, so **the application must depend on exactly that version**.

`native_app_glue` is intentionally not included: evloop-android implements the bridge between
the Java main thread and the application thread in Rust (`src/glue`).

Licensed under the Apache License, Version 2.0 (see `LICENSE`).
