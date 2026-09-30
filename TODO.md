# light-show — TODO for store publication (updated 2026-09-29)

Goal: Google Play + F-Droid (+ Windows). Repo: `qompassai/light-show`
(Rust + Bevy fiber-optic puzzle game).

Items marked ★ need Matt personally (accounts, keys, console clicks).
Everything else is doable by agents.

## Game completeness

- [x] **PR #1 merged** (2026-09-29, commit `58c0d47`, pushed to main).
      Outage-repair loop, win/fail results screen, companion dialogue,
      4-channel chiptune (5 tracks), 9 OSP icons now on main. Histories were
      disconnected (main was a squash); merged with
      `--allow-unrelated-histories`, PR side won all 42 conflicts
      (verified newer/complete). Gates on the merged tree: 67/67 tests
      green, `cargo build` clean, clippy 0 warnings, `cargo fmt --check`
      clean.
- [x] **AI portrait art rights: cleared by Matt** (free images, made on
      primo). No rights action needed.

## F-Droid

- [x] `docs/FDROID.md` recipe fixed: `commit:` is now the full 40-char SHA
      of the merged main, `ndk: 30.0.16248370` added, category corrected
      to `Puzzle Game` (was invalid `Games` + `Science & Education`).
- [x] fastlane: `images/icon.png` (512×512, from the app icon art) added;
      `short_description.txt` trailing dot removed (72 chars, single line).
- [ ] Phone screenshots: capturing from the real Android build (in
      progress on primo — emulator + `adb screencap`). `fdroid-publish-check`
      currently WARNs on these (strongly recommended, not required).
- [ ] Submit the fdroiddata MR ★ (fork fdroiddata, add the recipe, open
      the MR, answer reviewer questions — needs Matt's GitLab account).

## Google Play

- [ ] Build the signed `.aab` (Android APK build in progress on primo;
      keystore script validated — see below).
- [x] Upload keystore script: `scripts/publish/tbr-keystore.sh` —
      `--dry-run` proven end-to-end on primo's JDK 17 (throwaway keystore
      created, verified, deleted). Real creation is Matt's step ★.
- [x] F-Droid submit helper: `scripts/publish/tbr-fdroid-submit.sh`
      (runs the real gate check + prints the MR steps).
- [x] Play Console checklist: `scripts/publish/tbr-play-console.md`
      (service-account invite, listing copy, Data Safety, first upload).
- [ ] ★ Invite the service account (`pass` `google/ontrack-fastlane`,
      already exists, shared with ontrack) as release-manager on the
      light-show app in Play Console.
- [ ] ★ Play Console: create the app, Data Safety answers, store listing,
      first manual `.aab` upload (checklist in
      `scripts/publish/tbr-play-console.md`).

## Windows

- [ ] No installer/distribution story yet — Matt's call: MSIX vs Steam vs
      direct download. No work started.

---

Ground truth: `fdroid-publish-check` 2026-09-29 on the merged tree →
license/tags/nonfree PASS, fastlane WARN (screenshots pending).
Secrets inventory (names only):
`~/workspace/release-scripts/docs/secrets-inventory.md` — Matt handles
all keys; agents never generate or touch them.
