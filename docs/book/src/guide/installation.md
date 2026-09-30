# Installation

Light Show runs on desktop (Linux, and in principle anything Bevy
targets) and on Android (Google Play and F-Droid). Everything below is
grounded in the repo's `README.md`, `docs/BUILD.md`, and `flake.nix` —
if a command appears here, it appears there too.

## Desktop — the dev loop

```sh
cargo run -p light-show
```

The workspace has two members (`crates/osp_sim`, `game`); the binary
target lives in the `game` package. One platform requirement on Linux:
Bevy's `bevy_audio` feature links against ALSA, so you need the audio
dev headers — `libasound2-dev` on Debian/Ubuntu, `alsa-lib` on Arch —
or the build fails at link time.

Want to exercise just the simulation math, with no engine involved?

```sh
cargo test -p osp_sim
```

The link-budget core is a pure, engine-free crate, so its full test
suite runs in seconds without a GPU or a window.

## Nix — deterministic environments

The repo ships a `flake.nix` (NixOS/nixpkgs `nixos-26.05` pinned in
`flake.lock`) with a pinned Rust toolchain — the repo carries no
`rust-toolchain` file, so nixpkgs' `rustc`/`cargo` *is* the
deterministic toolchain:

```sh
nix develop            # pinned toolchain + bacon + lldb + deny/audit
nix run .#gates         # build/clippy/fmt/test + safety + debugger smoke
nix run .#publish-check # F-Droid/store metadata readiness (dry run)
nix run .#debug-smoke   # standalone lldb-dap breakpoint smoke test
nix run .#release -- v1.2.3 [--dry-run]  # GitHub release end to end
```

Every app transcribes its run into `reports/<app>-<UTC timestamp>.md`
and cleans up its scratch space via an `EXIT` trap; see `docs/FLAKE.md`.

## Android

The game targets `minSdk 26` (Android 8.0+) / `targetSdk 36`. Two
documented paths exist.

### Option A — `cargo-apk` (fastest path to a running APK)

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-apk
cd game
cargo apk build --release --lib
```

`--lib` is required, not optional: the `light-show` package ships both
a `cdylib` (the Android native-activity entry point) and a desktop
`bin` target, and `cargo apk build` without `--lib` tries the bin
target and dies with `Bin is not compatible with Cdylib`. The manifest
uses `android.app.lib_name` pointing at the cdylib.

For local verification builds only, `cargo-apk` reads the debug
keystore via its documented env override; for anything releasable the
signing story is human-gated (see below).

### Option B — Gradle wrapper (Play Store release build)

For a Play-Store-ready **Android App Bundle (.aab)** with Play App
Signing, the compiled `cdylib` is wrapped in the minimal Gradle project
under `android/`:

```sh
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cd game
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o ../android/app/src/main/jniLibs build --release
cd ../android
./gradlew bundleRelease
```

Output: `android/app/build/outputs/bundle/release/app-release.aab`.
One subtlety worth knowing: the 32-bit `armv7` target defaults to
4 KB ELF page alignment, but Android 15+ requires 16 KB for apps
targeting API 35+, so `.cargo/config.toml` forces it via
`link-arg=-Wl,-z,max-page-size=16384` for that target.

## The human-gated boundary

Release signing is deliberately *not* automated. Generating the
keystore (`scripts/publish/tbr-keystore.sh`), the F-Droid submission,
and the Play Console checklist are Matt-only steps — a deterministic
sandbox must never mint signing identities or submit anything to a
store. The `.aab` produced above is intentionally unsigned (no
`signingConfig`), so CI can never produce a signed release build by
accident. F-Droid re-signs from source anyway; the Play upload key
comes from Matt's keystore step.
