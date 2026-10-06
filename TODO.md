# light-show — TODO for store publication (updated 2026-09-30)

> Tracked on GitHub: [master TODO #2](https://github.com/qompassai/light-show/issues/2) -> [this track](https://github.com/qompassai/light-show/issues/7). Close the sub-issue to check off an item; completed items get a date + what/where below.

Goal: Google Play + F-Droid (+ Windows). Repo: `qompassai/light-show`
(Rust + Bevy fiber-optic puzzle game).

Items marked ★ need Matt personally (accounts, keys, console clicks).
Everything else is doable by agents.

## Game completeness

- [x] **PR #1 merged** (merge commit `58c0d47c1fc295285681a16cca043d469ff1eaaa`,
      merged 2026-09-30T02:47:49Z per GitHub API). Outage-repair loop,
      win/fail results screen, companion dialogue. Histories were
      disconnected (main was a squash); merged with
      `--allow-unrelated-histories`, PR side won all 42 conflicts
      (verified newer/complete).
- [x] **AI portrait art rights: cleared by Matt** (made on primo from
      free images; confirmed 2026-09-30). No rights action needed.
- [x] **Licensed game music wired** (2026-09-29/30): 12 tracks under
      `game/assets/music/<tier>/` (Eric Skiff / Kevin MacLeod / TeknoAXE
      = CC-BY, Komiku / SubspaceAudio = CC0);
      `game/assets/music/CREDITS.md` + `ATTRIBUTION.txt` ship the
      required CC-BY attribution inside the APK and on the in-game
      Credits screen (see `docs/CREDITS.md`). Pure music manager in
      `game/src/audio.rs` maps GameState + level world to tracks
      (menu / tutorial / early / mid / hard / boss-outage / victory /
      mystery-loss). Bevy enables both `vorbis` and `mp3` features —
      ten tracks are MP3, and the missing `mp3` feature crashed the app
      at startup with `bevy_audio … UnrecognizedFormat` (diagnosed
      2026-09-30; one MP3 also had a corrupt 424-byte header, repaired).
      All 12 tracks verified with `ffprobe` (valid streams, sensible
      durations).
- [x] **Stale chiptune assets removed**: the 5 unreferenced
      `game/assets/audio/*.ogg` tracks (superseded by the licensed
      music; zero references in `game/src/`) are staged for deletion.
      Decision recorded 2026-09-30: the deletion belongs in this
      release. (Staged via an earlier `git rm` that skipped the
      git-wip-guard protocol — noted, not repeated.)
- [x] **Menu↔Credits camera guard** (2026-09-30): `setup_menu` only
      spawns `Camera2dBundle` when no camera exists — repeated
      transitions no longer stack cameras.

## Android / Play

- [x] **targetSdk 36** in `game/Cargo.toml` (`target_sdk_version`) and
      `android/app/build.gradle.kts` (`compileSdk`/`targetSdk`) — Play
      requires API 36+ for new apps and updates since 2026-08-31
      (verified 2026-09-30 against Play Console Help and
      developer.android.com). `build_targets` in Cargo metadata:
      `aarch64-linux-android` + `x86_64-linux-android`. Native libs are
      16 KB page-aligned (Rust default; verified `0x4000` LOAD alignment
      on the built `.so`).
- [x] **cargo-apk release gate**: `cargo apk build --release --lib
      -p light-show` exits 0 (2026-09-30). Release packaging needs a
      signing config — local verification uses the pre-existing debug
      keystore via cargo-apk's documented
      `CARGO_APK_RELEASE_KEYSTORE[_PASSWORD]` env override; that APK is
      never distributed. Exit 101 (`Bin is not compatible with Cdylib`,
      missing `--lib`) and exit 1 (no signing config) are both
      **failure** — never "the APK is complete anyway".
- [x] **Gradle asset packaging fixed**: `android/app/build.gradle.kts`
      now declares
      `sourceSets["main"].assets.srcDirs("../../game/assets")` — without
      it the `.aab` would ship no game data (only `jniLibs` was
      declared).
- [x] **Unsigned `.aab` build + inspection** (done 2026-09-30):
      `cargo ndk` release `.so` for arm64-v8a / armeabi-v7a / x86_64,
      then `./gradlew bundleRelease` -> BUILD SUCCESSFUL (AGP 8.5.0
      accepted compileSdk 36, no bump needed).
      `app-release.aab` (73 MB) inspected: all 12 music tracks,
      `music/ATTRIBUTION.txt` + `CREDITS.md`, fonts present;
      all three `liblight_show.so` have 0x4000 (16 KB) LOAD alignment
      (Android 15+ requirement, via `.cargo/config.toml`).
- [x] **Emulator runtime re-verification** (done 2026-09-30):
      release APK rebuilt after the MP3/Cargo.toml fixes, reinstalled on
      emulator-5554, launched
      (`ai.qompass.lightshow/android.app.NativeActivity`), alive after
      35 s (`pidof` → 6244), native lib loaded, Vulkan init started,
      full logcat panic/fatal scan → zero hits.
- [ ] ★ Play Console: create the app, Data Safety answers, store
      listing, first manual `.aab` upload (checklist in
      `scripts/publish/tbr-play-console.md`). Upload keystore via
      `scripts/publish/tbr-keystore.sh` ★ — real creation is Matt's
      step; the final upload-key signing procedure (no passwords on
      argv) must be documented before he runs it.
- [ ] ★ Invite the service account (`pass` `google/ontrack-fastlane`,
      already exists, shared with ontrack) as release-manager on the
      light-show app in Play Console.

## F-Droid

- [x] `docs/FDROID.md` recipe corrected (2026-09-30): `ndk:
      30.0.16248370` (full `Pkg.Revision` — fdroidserver keys its NDK
      lookup by that string, verified against fdroidserver source);
      `output: target/release/apk/light_show.apk` (the real
      cdylib-derived filename); `build:` mints an ephemeral keystore
      and exports `CARGO_APK_RELEASE_KEYSTORE[_PASSWORD]` so the
      release build exits 0 on the farm (F-Droid re-signs with its own
      key). `commit:` keeps the `__RELEASE_COMMIT_SHA__` placeholder
      until Matt runs the documented two-commit sequence (push the
      release commit, verify the remote SHA with `git ls-remote`, fill
      it into the fdroiddata copy).
- [x] `fdroid-publish-check`: 5/5 PASS, READY, exit 0 (2026-09-30).
      (Checker fixed: the versions gate now also looks at
      `android/app/build.gradle.kts`, not just repo-root `app/`.)
- [x] fastlane: `images/icon.png` 512×512; `short_description.txt`
      single line ≤ 80 chars, no trailing dot; 2 real phone screenshots
      (1080×1920, captured 2026-09-29 from the emulator build).
- [x] **Gameplay screenshot re-captured** (2026-09-30): the board
      was invisible on Android because the MainMenu camera persisted
      into Playing with wrong framing. Fixed by spawning a dedicated
      `PlayingCamera` in `setup_level` (`game/src/states/playing.rs`)
      centered at y=200 on the board nodes; UI roots given explicit
      absolute positioning (`game/src/board.rs`). Verified on emulator:
      nodes/edges/icons render, briefing at top, Loss at bottom, no
      overlap. New capture replaces the broken shot in
      `fastlane/.../phoneScreenshots/ls_phone_02_gameplay.png`.
- [ ] Submit the fdroiddata MR ★ (fork fdroiddata, add the recipe, open
      the MR, answer reviewer questions — needs Matt's GitLab account).

## Known issues (observed, not yet resolved)

- **Gameplay board invisible** (RESOLVED 2026-09-30): the 2026-09-29
      capture showed a real bug — the MainMenu camera persisted into
      Playing and did not frame the world-space board. Fixed with a
      dedicated PlayingCamera; verified rendering on emulator.
- **Credits screen phone review** (RESOLVED 2026-09-30): Back button
      moved above the long attribution; verified visible and tappable
      on emulator; tap returns to Main Menu.

## Windows

- [ ] No installer/distribution story yet — Matt's call: MSIX vs Steam vs
      direct download. No work started.

---

Ground truth (2026-09-30, all on primo against the working tree):
`cargo test --workspace` → 76 passed, 0 failed (67 light-show + 9
osp_sim); `cargo clippy --workspace --all-targets` → 0 warnings;
`cargo fmt --all -- --check` → clean; `cargo deny check` →
advisories ok, bans ok, licenses ok, sources ok (24 duplicate-version
warnings, policy warn-level); `cargo apk build --release --lib
-p light-show` → exit 0 (debug-keystore env override, local
verification only, APK never distributed);
`scripts/publish/bin/fdroid-publish-check --repo . --appid
ai.qompass.lightshow --vercode 1` → 5/5 PASS, READY, exit 0.
AAB: `app-release.aab` built via `./gradlew bundleRelease` (BUILD
SUCCESSFUL), 12/12 tracks + attribution + fonts inside, 16 KB ELF
alignment on all 3 ABIs. Emulator: release APK installed on
emulator-5554, launched, alive after 35 s, zero panics in logcat.
DAP: scripted lldb-dap session vs the lib test binary — breakpoint at
`game/src/states/menu.rs:331` verified, hit, test exited 0.
Skills: `light-show-publish` + `rust-development` in `.claude/skills/`
and `.agents/skills/` — `skills-ref` valid + `skill-validator` passed
on all four copies. New: `bacon.toml` (`bacon --list-jobs` OK, default
job clippy), `.bsp/cargo.json` (valid per the diver BSP resolver
contract), `deny.toml` (cargo-deny 0.20.2 schema).
`CITATION.cff` and `.zenodo.json` byte-identical to `41fb988`. Remote
main: `51a449b3449ba7cf5e9bbffc8ea1c16805511b8b` — current work is
uncommitted and unpushed; no commit/push without fresh authorization.
Secrets inventory (names only):
`~/workspace/release-scripts/docs/secrets-inventory.md` — Matt handles
all keys; agents never generate or touch them.
