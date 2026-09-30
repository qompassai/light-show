# Play Console checklist — light-show (Matt-only)

Play Console is a web UI; nothing here can be scripted. Work through it
once, in order. Values below are the exact ones to paste.

## 1. Service account (one key for both apps)

- The service-account JSON already exists in your pass store:
  `google/ontrack-fastlane` (created 2026-09-29, shared light-show + ontrack).
- Play Console → Users and permissions → Invite new users →
  paste the service-account email → grant **release-manager**, scoped to
  the **light-show** app. (Repeat on the ontrack app when its turn comes.)

## 2. Create the app

- Play Console → Create app. Name: **Light Show**, default language en-US,
  app or game, free, no ads. Package name must be `ai.qompass.lightshow`
  (applicationId in `android/app/build.gradle.kts`, package in `game/Cargo.toml` `[package.metadata.android]` — the two must match).

## 3. Store listing (copy from this repo)

- Title: `Light Show: Fiber Optic Puzzle`
  (from `fastlane/metadata/android/en-US/title.txt`)
- Short description: copy `fastlane/metadata/android/en-US/short_description.txt`
- Full description: copy `fastlane/metadata/android/en-US/full_description.txt`
- Icon: `fastlane/metadata/android/en-US/images/icon.png` (512×512)
- Phone screenshots: `fastlane/metadata/android/en-US/images/phoneScreenshots/`
  (at least 2 of the running app)

## 4. App content

- **Data Safety:** the game is fully offline — no ads, no analytics, no
  network permission (see `docs/FDROID.md`). Answer: no data collected,
  no data shared.
- **Content rating:** complete the questionnaire (expect Everyone).
- **Target audience:** 18+ (or your call). **News apps:** No.
  **Government apps:** No. **App access:** no login wall (no accounts).
- **Privacy policy:** confirm the hosted URL is live before submitting.

## 5. First upload

- Build the signed `.aab` (see `docs/BUILD.md`; keystore via
  `scripts/publish/tbr-keystore.sh` — dry-run validated, real run is yours).
- Upload to the **Internal testing** track first. Add yourself as a tester,
  install via the testing link, play through World 1 end to end.
- Promote Internal → Closed → Production at your own pace. Every new
  upload needs a strictly higher `versionCode` than the last.
