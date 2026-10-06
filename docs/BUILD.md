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

Light Show targets `minSdk 26` (Android 8.0+) / `targetSdk 36`.
Google Play has required API 36+ for new apps and updates since
2026-08-31 (extension to 2026-11-01 on request) — verified
2026-09-30 against Play Console Help and developer.android.com.
Native libs are 16 KB page-aligned (Rust default), satisfying the
API 35+ 16 KB page-size requirement.

### Option A — `cargo-apk` (fastest path to a running APK)

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-apk
cd game
# Release packaging requires a signing config. For local verification
# builds only, point cargo-apk at the pre-existing debug keystore via
# its documented env override (never generate keys, never ship the
# resulting APK — F-Droid re-signs from source, and the Play upload
# key comes from scripts/publish/tbr-keystore.sh, which is Matt's step).
CARGO_APK_RELEASE_KEYSTORE=$HOME/.android/debug.keystore \
CARGO_APK_RELEASE_KEYSTORE_PASSWORD=android \
  cargo apk build --release --lib
```

`cargo-apk` reads `[package.metadata.android]` in `game/Cargo.toml` and
generates the manifest, resources, and APK automatically — good for
day-to-day testing on-device and for F-Droid, whose build server invokes a
declared Gradle/Cargo build recipe (see `docs/FDROID.md`).

`--lib` is required, not optional: the `light-show` package ships both a
`cdylib` (the Android native-activity entry point) and a desktop `bin`
target, and `cargo apk build` without `--lib` tries the bin target and
dies with `Bin is not compatible with Cdylib` (exit 101 — no APK worth
keeping). `--lib` builds exactly the cdylib the manifest's
`android.app.lib_name` points at.

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
   The 64-bit targets default to 16 KB ELF alignment, but
   `armv7-linux-androideabi` (armeabi-v7a) defaults to 4 KB. Android
   15+ (API 35+) requires 16 KB page alignment for apps targeting API
   35+, so `.cargo/config.toml` forces it for the 32-bit target:
   ```toml
   [target.armv7-linux-androideabi]
   rustflags = ["-C", "link-arg=-Wl,-z,max-page-size=16384"]
   ```
   Verify with `readelf -lW <liblight_show.so>` -- every `LOAD`
   segment of every ABI must show `0x4000`. (The `-Wl,` prefix is
   required: cargo-ndk drives the link through clang, which rejects a
   bare `-z` flag.)
2. `android/` contains a minimal Gradle wrapper project
   (`android/app/build.gradle.kts`) that packages those `.so` files behind a
   `NativeActivity`, matching `game/android/AndroidManifest.xml`.
3. Build the bundle:
   ```sh
   cd android
   ./gradlew bundleRelease
   ```
   Output: `android/app/build/outputs/bundle/release/app-release.aab`.
   This is the **unsigned technical AAB** — no `signingConfig` is
   declared on purpose, so CI never produces a signed release build.
4. Sign it with the upload key (Matt's step — the keystore is created
   with `scripts/publish/tbr-keystore.sh`, never by agents). Passwords
   must never appear on argv (`ps` exposes them); `apksigner` reads
   them from named environment variables instead:
   ```sh
   # Pull the passwords from pass into this shell only (same terminal,
   # never into a file or a command line):
   export LIGHTSHOW_KS_PASS="$(pass qompassai/light-show/upload-keystore-password)"
   export LIGHTSHOW_KEY_PASS="$(pass qompassai/light-show/upload-key-password)"
   /opt/android-sdk/build-tools/36.0.0/apksigner sign \
       --ks "$HOME/.config/qompassai/light-show/upload.jks" \
       --ks-pass:env LIGHTSHOW_KS_PASS \
       --key-pass:env LIGHTSHOW_KEY_PASS \
       --out app-release-signed.aab \
       app/build/outputs/bundle/release/app-release.aab
   /opt/android-sdk/build-tools/36.0.0/apksigner verify --print-certs \
       app-release-signed.aab
   unset LIGHTSHOW_KS_PASS LIGHTSHOW_KEY_PASS
   ```
   Upload `app-release-signed.aab` to a testing track first.

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
no NDK/emulator needed) is the cheap gate before the full on-device run.

### Reproducible builds for F-Droid

F-Droid's build server compiles from source using a `metadata/ai.qompass.lightshow.yml`
recipe (see `fastlane/` + `docs/FDROID.md`) — it does not accept prebuilt
binaries. Keep `Cargo.lock` committed so F-Droid's pinned-toolchain build is
reproducible, and avoid any dependency that phones home, requires
proprietary SDKs (no Google Play Services / Firebase / ads SDKs anywhere in
the dependency tree), or fetches remote assets at build or runtime.

## Gates (run locally — CI workflows were removed)

There is no CI on this repo (workflows removed 2026-09-30); the release
gates run on the build machine before any release commit is pushed:

```sh
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
# Subshell keeps the checker's cwd at the repo root; the release build
# needs the signing env from Option A and must exit 0.
(cd game && CARGO_APK_RELEASE_KEYSTORE=$HOME/.android/debug.keystore \
  CARGO_APK_RELEASE_KEYSTORE_PASSWORD=android \
  cargo apk build --release --lib)
scripts/publish/bin/fdroid-publish-check --repo . --appid ai.qompass.lightshow --vercode 1
```

## Editor toolchain: bacon, cargo-deny, BSP

Three repo-root files wire this project into Matt's Neovim setup on the
build machine (primo). They are developer tooling, not build inputs: the
game builds identically without them.

### `bacon.toml` — background checks for bacon-ls

bacon-ls (Matt's `lsp/bacon_ls.lua`) runs `bacon --headless -j bacon-ls`
in the background and reads the `.bacon-locations` export file bacon
writes, publishing the diagnostics as LSP diagnostics. The `bacon-ls`
job in `bacon.toml` is the exact job definition bacon-ls's bacon backend
expects (clippy, `--message-format json-diagnostic-rendered-ansi`,
`analyzer = "cargo_json"`, plus the `[exports.cargo-json-spans]` export
block). The other jobs are the project's real gates for terminal use:

| job      | command (abridged)                              |
|----------|-------------------------------------------------|
| `check`  | `cargo check --workspace --all-targets`          |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` (default job) |
| `test`   | `cargo test --workspace`                        |
| `bacon-ls` | clippy with JSON diagnostics, consumed by bacon-ls |

```sh
bacon --list-jobs   # verify the file parses
bacon               # run the default (clippy) job
bacon test          # run a named job
```

`.bacon-locations` is gitignored; it is regenerated on every bacon run.

### `deny.toml` — license / advisory / ban policy

`cargo deny check` enforces the dependency policy for a store-shipped
Apache-2.0 game (cargo-deny 0.20.2 on primo, `/usr/bin/cargo-deny`):

- **licenses**: allow-list is the policy (deny-by-default since
  cargo-deny 0.18.4) — permissive licenses only (MIT/Apache-2.0/BSD),
  compatible with the project's own license. Copyleft licenses
  (GPL/AGPL/LGPL families) are denied by omission.
- **advisories**: yanked crates are denied; unmaintained-crate
  advisories fail for any crate in the tree. Known exceptions carry a
  dated reason in `advisories.ignore` (e.g. RUSTSEC-2026-0192,
  ttf-parser via bevy 0.14 — no safe upgrade, revisit on the bevy
  upgrade).
- **bans**: wildcard version requirements are denied (a path
  dependency without `version` counts as a wildcard — hence
  `osp_sim = { path = ..., version = "0.1.0" }`); duplicate crate
  versions only warn.
- **sources**: only crates.io is an acceptable registry; git sources
  are denied.

`[graph.targets]` pins the check to the project's real build matrix
(desktop Linux + the three Android ABIs in the AAB) so
target-specific dependencies are checked too. All four checks pass:
`advisories ok, bans ok, licenses ok, sources ok`.

### `.bsp/cargo.json` — Build Server Protocol connection card

Matt's native BSP client (`diver` `lua/bsp/servers/cargo.lua`) reads
`.bsp/*.json` connection cards with the fields `name`, `version`,
`bspVersion`, `languages`, and `argv`. The upstream server,
[cargo-bsp](https://github.com/cargo-bsp/cargo-bsp), is unmaintained
(last push 2023) and is **not installed** on primo, so this card is
hand-written: `argv` is `["cargo-bsp"]` (PATH-resolved once the server
is installed, e.g. `cargo install --git
https://github.com/cargo-bsp/cargo-bsp`), `version` is honestly marked
`0.0.0-handwritten`. The resolver tolerates hand-written cards, so the
client will pick this up as soon as a compatible server exists on PATH.
Until then, rebuild queries go through `cargo` directly — BSP is not
required for any gate in this repo.

## Deterministic builds with Nix

This repo ships a Nix flake (`flake.nix`, `flake.lock`) that pins the
full toolchain (rustc 1.95.0, lldb 21.1.8, cargo-deny/audit, bacon) and
turns the deterministic-safe validation scripts into `nix run` apps with
markdown reports under `reports/`. See [docs/FLAKE.md](FLAKE.md) for the
app list, the `nix run .#release` flow, and the documented Android gap
(APK/AAB builds stay primo-local; the Nix toolchain has no Android SDK).
