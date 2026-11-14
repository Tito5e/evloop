// The Java side of evloop-android.
//
// Include this module from your Gradle project: it brings the GameActivity AAR (and its required
// dependencies) in the version the native side is built against.
plugins {
    id("com.android.library")
}

android {
    namespace = "dev.evloop"
    compileSdk = 36

    defaultConfig {
        minSdk = 24
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // Must match the GameActivity native sources vendored in third_party/games-sdk.
    api("androidx.games:games-activity:4.4.0")
    // GameActivity extends AppCompatActivity and uses androidx.core, but its POM declares no
    // dependencies, so they have to be added explicitly.
    api("androidx.appcompat:appcompat:1.7.1")
    api("androidx.core:core:1.16.0")
}
