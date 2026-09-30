---
name: "light-show-publish"
description: "Full store-publication journey for the light-show Bevy game (qompassai/light-show): PR merge verification, fastlane metadata, F-Droid recipe, publish scripts, emulator screenshots, licensed music wiring, and the primo build/test gates. Trigger when Matt asks to publish light-show to Google Play or F-Droid."
metadata:
  includeInPrompt: "true"
---

# Light Show Publish

Take the light-show fiber-optic puzzle game (Rust + Bevy,
`qompassai/light-show`) from a merged PR to store-ready: Google Play and
F-Droid. This skill codifies the exact journey run on 2026-09-29 —
every step below was executed against the real repo, the real Android
toolchain on primo, and a real emulator.

## Standing rules (never break these)

1. **All Rust work happens on primo** (`~/workspace/bin/primo-ssh` from
   the sandbox; never plain `ssh primo` — it resolves to a dead
   address). The repo checkout is
   `/home/phaedrus/workspace/repos/light-show`.
2. **Tiger Style Rust** for every code change (`tiger-style-rust` skill):
   explicit contracts, bounded work, tests first, `cargo fmt` clean.
3. **Never force-push.** Every push is fast-forward only: `git fetch`,
   confirm the remote is an ancestor, push, then verify
   `git ls-remote origin main` matches local `main`. If the remote moved,
   rebase cleanly and report the move; if it moved incompatibly, stop
   and report.
4. **Human-gated scripts are never run by agents.** Files named `tbr-*`
   under `scripts/publish/` are for Matt to run himself (keystores,
   console submissions). Agents may read, review, and dry-run them only
   if Matt explicitly asks; the default finished state leaves them
   untouched for him.
5. **Never generate signing keys.** No keytool, no keystores, no
   exceptions — not even "throwaway" ones for testing. If a build needs
   signing to proceed, stop and report it as Matt's step.
6. **git-wip-guard** before any destructive git operation in Matt's
   repos: inspect, attribute, snapshot, confirm, verify.

## 1. PR merge verification

When a feature branch must land on `main`:

1. Check whether the histories are connected:
   `git merge-base feature-branch main`. If empty, the histories are
   disconnected (e.g. main was a squash).
2. For disconnected histories, merge with
   `git merge --allow-unrelated-histories`. Resolve add/add conflicts by
   the **newer/better** rule: compare both sides, keep the side that is
   newer and more complete — verify by reading the conflicting files,
   not by guessing. Record which side won and why in the merge commit
   message.
3. Preserve history: never rewrite the PR branch, never squash on merge
   unless Matt asks. The merge commit message names the PR, what it
   adds, and the conflict-resolution rule used.
4. Gates on the merged tree before pushing:
   - `cargo test --workspace` — all pass
   - `cargo build --workspace` — clean
   - `cargo clippy --workspace --all-targets` — zero warnings
   - `cargo fmt --all -- --check` — clean
5. Push fast-forward only, verify remote-side SHA.

Worked example (2026-09-29): PR branch
`feature/outage-loop-audio-art-store-readiness` had a disconnected
history from main. Merged with `--allow-unrelated-histories`; the PR
side won all 42 add/add conflicts (it held the complete game).
`TODO.md`, unique to main, survived the merge. Gates: 67/67 tests,
clippy 0 warnings, fmt clean. Pushed as `58c0d47`.

## 2. Fastlane metadata

Location: `fastlane/metadata/android/en-US/`. Every file here ships to
both Play and F-Droid listings.

- `images/icon.png` — **real** 512×512 PNG. Source it from the game's
  own art (`game/assets/art/`); never upscale a smaller image, never
  generate one. Verify: `file icon.png` reports `512 x 512`.
- `short_description.txt` — one line, **≤ 80 characters**, no trailing
  period. Verify: `wc -c` and `wc -l`.
- `images/phoneScreenshots/` — **2+ real captures** from the Android
  build running on the emulator (see §5). No mockups, no desktop
  screenshots, no placeholders. Name them descriptively
  (`ls_phone_01_menu.png`, `ls_phone_02_gameplay.png`).
- `full_description.txt` — full listing copy when Matt approves it.

## 3. F-Droid recipe

`docs/FDROID.md` holds the fdroiddata recipe draft. Rules:

- `commit:` is the **full 40-character SHA** of the exact commit that
  builds — never a short SHA, never a tag, never `main`. The commit
  must already be pushed; verify with `git ls-remote`.
- `ndk:` is the **official NDK revision string** from the NDK in use
  (e.g. `30.0.16248370` from `$ANDROID_NDK_ROOT/source.properties`) —
  never invent it, never copy it from another project.
- `Categories:` must be a valid F-Droid category (`Puzzle Game`,
  not `Games`).
- No invented `versionName`/`versionCode`: they come from the build,
  not from the recipe author.
- Run `scripts/publish/bin/fdroid-publish-check` directly (not through
  any human-gated wrapper) against the final tree and keep its output.
  All gates must PASS before the fdroiddata MR goes up.

## 4. Publish scripts layout

`scripts/publish/` separates what agents may run from what only Matt
may run:

- `bin/fdroid-publish-check` — repeatable, agent-runnable: license
  scan, tag check, nonfree-blob check, fastlane presence, screenshot
  presence. Exits nonzero on failure; never masks failures with
  `|| true`.
- `tbr-keystore.sh` — **human-gated.** Generates the Play upload
  keystore. Real mode must never place passwords on argv (use env vars
  or stdin). Agents do not run it.
- `tbr-fdroid-submit.sh` — **human-gated.** Runs the real gate check
  and prints the fdroiddata MR steps. Agents do not run it.
- `tbr-play-console.md` — **human-gated checklist** (service-account
  invite, listing copy, Data Safety, first `.aab` upload). App identity
  lives in `game/Cargo.toml` (`[package.metadata.android]`); there is
  no `android/app/build.gradle.kts` in this repo — don't reference one.

Naming rule: repeatable automation goes in `bin/`; anything needing
Matt's hands, accounts, or keys is named `tbr-*` (to-be-run).

## 5. Emulator screenshot pipeline

Produces the real `phoneScreenshots/` from §2. All on primo;
`/dev/kvm` must exist.

1. Boot the AVD headless:
   `/opt/android-sdk/emulator/emulator -avd <avd-name> -no-window -gpu swiftshader_indirect &`
   Wait for `adb shell getprop sys.boot_completed` to return `1`.
2. Build the release APK: `cargo apk build --release` from `game/`
   (with `JAVA_HOME`, `ANDROID_HOME`, `ANDROID_NDK_ROOT` set). The
   `[package.metadata.android]` section of `game/Cargo.toml` **must**
   declare `assets = "assets"` — cargo-apk silently omits the entire
   `assets/` dir without it (fonts, sprites, audio all missing; the
   game renders unlabeled rectangles). Verify with
   `unzip -l target/release/apk/light_show.apk | grep -c assets/`.
3. `adb install -r target/release/apk/light_show.apk`. If a previous
   install used a different signing key,
   `INSTALL_FAILED_UPDATE_INCOMPATIBLE` appears — `adb uninstall`
   first, then fresh install.
4. Launch: `adb shell am start -n ai.qompass.lightshow/android.app.NativeActivity`.
   Confirm alive: `adb shell pidof ai.qompass.lightshow`.
5. Screenshot: `adb exec-out screencap -p > phoneScreenshotN.png`.
   Interact with `adb shell input tap x y` / `input swipe ...` to reach
   gameplay; capture menu first, then real gameplay.
6. **Inspect every PNG before committing it.** Describe exactly what it
   shows. Launcher-only or black screens are rejected — re-capture.
7. `adb emu kill` when done.

Known emulator pitfall (2026-09-29): Bevy 0.14 / wgpu-hal 0.21
segfaults on SwiftShader in `set_debug_utils_object_name` (null
function pointer from `vkGetInstanceProcAddr`). If the game dies on
launch with this tombstone, the fix is a wgpu-hal patch or an engine
upgrade — do not fake screenshots to work around it; report the exact
crash.

## 6. Licensed music

`game/assets/music/<tier>/` holds the soundtrack; the tier dirs are
`menu`, `tutorial`, `early`, `mid`, `hard`, `boss`, `victory`,
`mystery`.

- **CC0** (Komiku, SubspaceAudio): no restrictions; courtesy credit
  optional.
- **CC-BY** (Eric Skiff, Kevin MacLeod, TeknoAXE): attribution is
  **legally required**. `game/assets/music/CREDITS.md` carries the
  ready-to-paste attribution block and **ships inside the game** (it's
  under `assets/`, so it lands in the APK). Never ship a build without
  it.
- Wiring: `game/src/audio.rs` holds a pure music manager —
  `menu_track()`, `playing_track(level_index, world)`,
  `outage_track(outage_count)`, `results_track(won)` — returning
  `assets/`-relative paths. Worlds tier the in-level music
  (1=tutorial, 2=early, 3=mid, 4+=hard); two-track tiers rotate
  deterministically by level index; outage alternates the boss tracks.
  Systems spawn the track `OnEnter` each `GameState` and despawn
  `OnExit` (Bevy runs old-state `OnExit` before new-state `OnEnter`,
  so tracks never overlap).
- Tests (do not weaken): `every_referenced_track_exists_on_disk`
  covers every tier and rotation slot; mapping tests pin the
  world→tier boundaries, the rotation order, the outage alternation,
  and the win/loss split.

## 7. TODO.md

`TODO.md` is the program's ground truth. Keep it current:

- Check off what's done with the commit SHA and date.
- Items needing Matt personally (accounts, keys, console clicks) are
  marked ★.
- The "Ground truth" footer records the last `fdroid-publish-check`
  output and the music/attribution state.
- Art-rights and license decisions are recorded once, with who decided
  and when — never re-litigated.

## 8. Primo build/test gates

Run from the repo root on primo for the exact tree being published:

- `cargo test --workspace` — all pass, zero failures
- `cargo build --workspace` — clean
- `cargo clippy --workspace --all-targets` — zero warnings
- `cargo fmt --all -- --check` — clean
- `cargo apk build --release` from `game/` — APK produced (the
  trailing `Bin is not compatible with Cdylib` cargo-apk quirk after
  packaging is known; the APK itself is complete — verify with
  `unzip -l`)
- `scripts/publish/bin/fdroid-publish-check` — all gates PASS

Toolchain (primo, 2026-09-29): `cargo-apk` at
`~/.cargo/bin/cargo-apk`, NDK r30 (`30.0.16248370`) at
`/opt/android-ndk`, SDK at `/opt/android-sdk`, JDK 17, AVD with KVM.
Package `ai.qompass.lightshow`, activity
`android.app.NativeActivity`.

## What stays Matt-only

- Running any `tbr-*` script; generating any signing key.
- Play Console: service account invite (`pass` entry
  `google/ontrack-fastlane`, role `release-manager`), app creation,
  listing copy, Data Safety answers, first `.aab` upload.
- The fdroiddata GitLab MR (needs his GitLab account).
- Windows distribution story (MSIX vs Steam vs direct) — his call.
