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
- **No non-free assets.** All art/audio/fonts are original or CC0/CC-BY,
  tracked with attribution in `docs/CREDITS.md`. AI-assisted art passes
  used only for early placeholders (see `docs/ART_STYLE.md`) are replaced
  before a tagged F-Droid release, with the licensing chain documented in
  `docs/CREDITS.md`.
- **No pay-to-win / IAP.** There is no monetization at all — no ads,
  no purchases, no gacha, so the same build serves both stores unmodified.

## Reference metadata recipe

```yaml
Categories:
  - Games
  - Science & Education
License: GPL-3.0-or-later
SourceCode: https://github.com/qompassai/light-show
IssueTracker: https://github.com/qompassai/light-show/issues

RepoType: git
Repo: https://github.com/qompassai/light-show.git

Builds:
  - versionName: '0.1.0'
    versionCode: 1
    commit: v0.1.0
    subdir: game
    sudo:
      - apt-get update
      - apt-get install -y curl build-essential
    init:
      - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
      - . $HOME/.cargo/env
      - rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
      - cargo install cargo-apk
    build:
      - cargo apk build --release
    output: target/release/apk/light-show.apk

AutoUpdateMode: Version
UpdateCheckMode: Tags
CurrentVersion: '0.1.0'
CurrentVersionCode: 1
```

Place the authoritative copy of this recipe at
`metadata/ai.qompass.lightshow.yml` when submitting to `fdroiddata`, matching
the applicationId in `game/Cargo.toml`'s `[package.metadata.android]`.

## Store description parity

`fastlane/metadata/android/en-US/` holds the shared store listing copy
(title, short/full description, changelog) used by both the Google Play
Console upload and F-Droid's fastlane-format metadata ingestion, so the two
listings stay in sync from one source of truth.
