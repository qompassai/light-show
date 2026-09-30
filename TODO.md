# light-show — TODO for store publication (2026-09-29)

Goal: Google Play + F-Droid (+ Windows). Repo: `qompassai/light-show`
(Rust + Bevy fiber-optic puzzle game).

Items marked ★ need Matt personally (accounts, keys, console clicks).
Everything else is doable by agents.

## Game completeness (blocks everything)

- [ ] **Decide the fate of PR #1.** The outage-repair loop (fault injection,
      live reroute under a countdown), win/fail results screen, companion
      dialogue hookup, 4-channel NES-style chiptune synth, and all 9 OSP
      component icons live in open PR #1 (~3 weeks old, 67 tests green at PR
      time). Merge it or supersede it before any new code. `main` currently
      has soft-locking outage levels and a stub results screen.
      (Phase-1 audit, 2026-09-30. Matt's order: nothing starts here until
      ONTrack ships.)
- [ ] **AI portrait art rights: TBD.** Real blocker before any production
      release, not paperwork.

## F-Droid

- [ ] Fix the reference recipe in `docs/FDROID.md`: `commit: v0.1.0` is a
      tag, not a full 40-char SHA (F-Droid rejects tags), and no `v0.1.0`
      tag exists anyway. Add the missing `ndk:` line.
- [ ] fastlane gaps (verified by `fdroid-publish-check`, 2026-09-29):
      add `images/icon.png` (512×512), add phone screenshots (strongly
      recommended), remove the trailing dot from `short_description.txt`.
- [ ] Submit the fdroiddata MR (fork fdroiddata, add the recipe, open the
      MR, answer reviewer questions).

## Google Play

- [ ] Build the signed `.aab` on primo (Android SDK + NDK r30 present;
      see `docs/BUILD.md`; `android/` Gradle project,
      `applicationId = "ai.qompass.lightshow"`).
- [ ] ★ Create the upload keystore + key passwords (MISSING — losing the
      keystore means a new app listing). Locations:
      `~/workspace/release-scripts/docs/secrets-inventory.md`.
- [ ] ★ Invite the service account (`pass` `google/ontrack-fastlane`,
      already exists, shared with ontrack) as release-manager on the
      light-show app in Play Console.
- [ ] ★ Play Console: create the app, Data Safety answers, store listing,
      first manual `.aab` upload.

## Windows

- [ ] No installer/distribution story yet — Matt's call: MSIX vs Steam vs
      direct download. No work started.

---

Ground truth: `fdroid-publish-check` 2026-09-29 → NOT READY (fastlane);
Phase-1 audit 2026-09-30. Secrets inventory (names only):
`~/workspace/release-scripts/docs/secrets-inventory.md` — Matt handles
all keys; agents never generate or touch them.
