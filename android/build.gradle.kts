// Root Gradle wrapper project for Light Show's Android packaging + on-device
// instrumentation tests.
//
// This wraps the `light_show` cdylib (built separately via `cargo ndk`, see
// docs/BUILD.md "Option B") behind a NativeActivity for:
//   1. Google Play Store Android App Bundle (.aab) releases with Play App
//      Signing (`./gradlew bundleRelease`), and
//   2. Android instrumentation tests that drive real touch input against the
//      running app and assert on structured Logcat events, since a
//      NativeActivity has no accessible Espresso view hierarchy to assert
//      against directly (see app/src/androidTest and docs/BUILD.md).
//
// `cargo-apk` (docs/BUILD.md "Option A") remains the fast day-to-day/F-Droid
// build path and does not use this Gradle project at all.

plugins {
    id("com.android.application") version "8.5.0" apply false
    id("org.jetbrains.kotlin.android") version "1.9.24" apply false
}
