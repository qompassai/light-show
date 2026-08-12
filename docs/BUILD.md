# Building Light Show

## Desktop (dev loop)

```sh
cargo run -p light-show
```

Requires system audio dev headers on Linux (`libasound2-dev` on
Debian/Ubuntu, `alsa-lib` on Arch) since Bevy's `bevy_audio` feature links
against ALSA on desktop Linux.

## Running the simulation tests

The fiber-optics link-budget math lives in a pure, engine-free crate so it
can be tested in isolation:

```sh
cargo test -p osp_sim
```

## Android

Light Show targets `minSdk 26` (Android 8.0+) / `targetSdk 34`, matching
current Google Play requirements.

### Option A — `cargo-apk` (fastest path to a running APK)

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-apk
cd game
cargo apk build --release
```

`cargo-apk` reads `[package.metadata.android]` in `game/Cargo.toml` and
generates the manifest, resources, and APK automatically — good for
day-to-day testing on-device and for F-Droid, whose build server invokes a
declared Gradle/Cargo build recipe (see `docs/FDROID.md`).

### Option B — Gradle wrapper (recommended for the Play Store release build)

For a Play-Store-ready **Android App Bundle (.aab)** with Play App Signing,
wrap the compiled `cdylib` in a minimal Gradle project:

1. Build the native library per ABI:
   ```sh
   cargo install cargo-ndk
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   cd game
   cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../android/app/src/main/jniLibs build --release
   ```
2. `android/` contains a minimal Gradle wrapper project
   (`android/app/build.gradle`) that packages those `.so` files behind a
   `NativeActivity`, matching `game/android/AndroidManifest.xml`.
3. Build the bundle:
   ```sh
   cd android
   ./gradlew bundleRelease
   ```
   Output: `android/app/build/outputs/bundle/release/app-release.aab`.
4. Sign with Play App Signing (upload key) per Google Play Console's
   standard flow, then upload the `.aab` to a testing track first.

### Instrumentation tests

`android/app/src/androidTest/java/ai/qompass/lightshow/BoardInteractionInstrumentedTest.kt`
drives the drag-to-route and pill-selection interactions on a real (or
emulated) device. Light Show renders entirely inside a single
`android.app.NativeActivity` GL surface, so there is no Espresso-visible
view hierarchy to assert against — Espresso's `onView(...)` matchers have
nothing to match. Instead the tests take a black-box approach:

1. The native library, when built with the `instrumented-test-logging`
   Cargo feature, logs structured one-line events to Logcat (tag
   `LightShow`) at the moments a test needs to observe: `level_ready` once
   the board has finished spawning (`states::playing::setup_level`), and
   `select ...` / `connect ...` whenever `board::handle_pointer_input`
   places a component (see the `test_log!` macro in `game/src/lib.rs`).
   This feature is off by default — retail builds (`cargo-apk` for
   F-Droid, the Gradle `release` build type for Play Store) never enable
   it, so it has zero footprint outside test builds.
2. The Kotlin test drives real touch gestures via
   [UiAutomator](https://developer.android.com/training/testing/other-components/ui-automator)
   (`UiDevice.swipe`/`click`) at screen coordinates it computes from the
   device's actual runtime display size, mirroring `board.rs`'s
   `grid_to_world`/`pill_world_pos` world-space math for the bundled
   "First Light" level — so the test stays correct regardless of which
   emulator/device profile runs it, rather than assuming one hard-coded
   resolution.
3. It then polls `adb shell logcat -d -s LightShow:I` for the expected
   event line.

To run locally:

```sh
# 1. Build the .so with test logging enabled (x86_64 covers most emulators;
#    add other targets if testing on a physical arm64 device).
cargo install cargo-ndk
rustup target add x86_64-linux-android
cd game
cargo ndk -t x86_64 -o ../android/app/src/main/jniLibs build --features instrumented-test-logging
cd ..

# 2. Start/attach an emulator or physical device, then run the tests.
cd android
./gradlew connectedDebugAndroidTest
```

A fast Kotlin-only compile check (`./gradlew :app:compileDebugAndroidTestKotlin`,
no NDK/emulator needed) runs on every push/PR via
`.github/workflows/android-test-compile.yml`. The full on-device run
(NDK build + booted emulator via `reactivecircus/android-emulator-runner`)
is the `instrumented-tests` job in `.github/workflows/android.yml`, which
only fires on tagged releases / manual dispatch since it is much slower.

### Reproducible builds for F-Droid

F-Droid's build server compiles from source using a `metadata/ai.qompass.lightshow.yml`
recipe (see `fastlane/` + `docs/FDROID.md`) — it does not accept prebuilt
binaries. Keep `Cargo.lock` committed so F-Droid's pinned-toolchain build is
reproducible, and avoid any dependency that phones home, requires
proprietary SDKs (no Google Play Services / Firebase / ads SDKs anywhere in
the dependency tree), or fetches remote assets at build or runtime.

## CI

`.github/workflows/ci.yml` runs `cargo test --workspace` and
`cargo clippy --workspace -- -D warnings` on every push/PR.
`.github/workflows/android-test-compile.yml` compile-checks the Gradle
project's Kotlin instrumentation tests on every push/PR (fast, no NDK or
emulator). `.github/workflows/android.yml` cross-compiles the Android
`.so` targets and runs the full on-device instrumentation test suite on an
emulator on tagged releases / manual dispatch, uploading the release APK as
a build artifact.
