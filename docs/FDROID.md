# F-Droid Packaging Notes

F-Droid builds from source on its own infrastructure using a metadata
recipe (submitted to the `fdroiddata` repo, not stored here beyond a
reference copy). Key requirements this project is designed to satisfy:

## Anti-features avoided

- **No ads.** No ad SDK is or will be a dependency.
- **No tracking.** No analytics/telemetry SDK; no network permission
  requested at all — Light Show runs fully offline.
- **No non-free dependencies.** The dependency tree (`Cargo.lock`) must stay
  free of Google Play Services, Firebase, or other proprietary blobs.
  `bevy`'s Android backend (`android-activity`/`game-activity`) is Apache-
  2.0/MIT, part of the free-software `rust-mobile` ecosystem.
- **No non-free assets.** All art/fonts are original, deterministic
  tool-generated, or OFL-licensed (Monaspace Neon, Inter), tracked with full
  attribution in `docs/CREDITS.md`. Background music is licensed
  third-party audio — CC-BY and CC0, 12 tracks under
  `game/assets/music/` — and the required CC-BY attribution ships both as
  `game/assets/music/CREDITS.md` in the APK and on the in-game Credits
  screen (see `docs/CREDITS.md`).
- **No pay-to-win / IAP.** There is no monetization at all — no ads,
  no purchases, no gacha, so the same build serves both stores unmodified.

## Reference metadata recipe

```yaml
Categories:
  - Puzzle Game
License: Apache-2.0
SourceCode: https://github.com/qompassai/light-show
IssueTracker: https://github.com/qompassai/light-show/issues

RepoType: git
Repo: https://github.com/qompassai/light-show.git

Builds:
  - versionName: '0.1.0'
    # versionCode is NOT 1: cargo-apk derives it from the crate semver
    # and rejects any TOML override. For 0.1.0 the real emitted code is
    # 16777472 (verified via aapt on the built APK, 2026-09-30).
    # F-Droid's build.py raises BuildException when the built APK's
    # versionCode differs from this field, so it must be re-verified
    # from the actual build output on every version bump -- never
    # invented.
    versionCode: 16777472
    commit: __RELEASE_COMMIT_SHA__
    ndk: 30.0.16248370
    subdir: game
    sudo:
      - apt-get update
      - apt-get install -y curl build-essential
    init:
      - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
      - . $HOME/.cargo/env
      - rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
      - cargo install cargo-apk --version 0.10.0
    # cargo-apk release builds require a signing config (exit 1 without
    # one), so the farm mints an ephemeral keypair valid for this build
    # only. Verified 2026-09-30 against fdroidserver's own source:
    # build.py treats the recipe output as the *unsigned* APK and
    # publish.py re-signs it with F-Droid's key via `apksigner sign`;
    # apksigner was empirically confirmed to accept a v2/v3-signed
    # input, so the ephemeral signature is replaced, never shipped.
    # The key material never leaves the build environment.
    # NOT yet verified: a real `fdroid build` run on F-Droid infra --
    # validate before submitting the merge request.
    build:
      - keytool -genkeypair -keystore /tmp/fdroid-ephemeral.jks -alias fdroid
        -keyalg RSA -keysize 2048 -validity 10950
        -storepass fdroid -keypass fdroid -dname "CN=F-Droid Ephemeral"
      - export CARGO_APK_RELEASE_KEYSTORE=/tmp/fdroid-ephemeral.jks
      - export CARGO_APK_RELEASE_KEYSTORE_PASSWORD=fdroid
      - cargo apk build --release --lib
    output: target/release/apk/light_show.apk

AutoUpdateMode: Version
UpdateCheckMode: Tags
CurrentVersion: '0.1.0'
CurrentVersionCode: 16777472
```

`commit:` must be the full 40-char SHA of the validated release commit.
A commit cannot contain its own SHA, so publication is a two-commit
sequence (the placeholder stays in this reference copy until Matt runs
it):

1. Commit and push the release tree, then verify the remote SHA:
   `git ls-remote origin main` must show the pushed commit.
2. Copy this recipe into `fdroiddata` with that verified SHA in
   `commit:` — never a guessed or local-only SHA.
3. Optionally fill the verified SHA into this reference copy in a
   follow-up docs commit, so the repo records exactly what shipped.

Place the authoritative copy of this recipe at
`metadata/ai.qompass.lightshow.yml` when submitting to `fdroiddata`,
matching the applicationId in `game/Cargo.toml`'s
`[package.metadata.android]` (also declared as `applicationId` in
`android/app/build.gradle.kts` — the two must match).

## Store description parity

`fastlane/metadata/android/en-US/` holds the shared store listing copy
(title, short/full description, changelog) used by both the Google Play
Console upload and F-Droid's fastlane-format metadata ingestion, so the two
listings stay in sync from one source of truth.
