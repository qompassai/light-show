plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "ai.qompass.lightshow"
    compileSdk = 34

    defaultConfig {
        applicationId = "ai.qompass.lightshow"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        // NativeActivity loads this cdylib, matching game/Cargo.toml's
        // [lib] name and game/android/AndroidManifest.xml's
        // android.app.lib_name meta-data.
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    // Prebuilt per-ABI .so files land here via:
    //   cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
    //     -o android/app/src/main/jniLibs build --release -p light-show
    // (add --features instrumented-test-logging when building for
    // instrumentation tests — see docs/BUILD.md).
    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

    buildTypes {
        debug {
            // Instrumentation tests run against the debug build type by
            // default (connectedDebugAndroidTest). The jniLibs consumed by
            // this build type should be built with
            // `--features instrumented-test-logging` so board.rs's Logcat
            // events exist for the tests in app/src/androidTest to assert
            // against.
        }
        release {
            isMinifyEnabled = false
            // Play App Signing handles the upload/app signing keys — see
            // docs/BUILD.md "Option B" for the Play Console flow. No
            // signingConfig is declared here on purpose; CI does not
            // produce a signed release build.
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    // No Compose/View UI at all — Light Show renders entirely through Bevy
    // inside a single NativeActivity's GL surface.
    buildFeatures {
        viewBinding = false
    }
}

dependencies {
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test:rules:1.6.1")
    androidTestImplementation("androidx.test.uiautomator:uiautomator:2.3.0")
}
