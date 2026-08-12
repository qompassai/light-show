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
`.github/workflows/android.yml` cross-compiles the Android `.so` targets on
tagged releases and uploads them as build artifacts.
