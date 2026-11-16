plugins {
    id("com.android.application")
}

android {
    namespace = "dev.evloop.smoke"
    compileSdk = 36

    defaultConfig {
        applicationId = "dev.evloop.smoke"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // `run.sh` copies the Rust library (libsmoke.so) here.
    sourceSets["main"].jniLibs.directories.add("build/rustJniLibs")
}

dependencies {
    implementation(project(":evloop"))
}
