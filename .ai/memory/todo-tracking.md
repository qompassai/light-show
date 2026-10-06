# TODO Tracking — light-show (persistent)

GitHub: master [#2](https://github.com/qompassai/light-show/issues/2),
tracks #3 (game) #4 (art) #5 (scripts) #6 (docs) #7 (store),
leaf issues #8–#37. Closing a leaf issue checks off the item.

## Completed (2026-10-05)

- Tests: 379 passing; 11 defects fixed (commit e1e26a5).
- Relicense GPL-3.0-or-later → Apache-2.0 (commit 50c6110).
- UX: TRACK COMING SOON badge + click SFX on trackless specialist cards
  (`game/src/states/companion_select.rs`, commit 7cd4920).
- Docs: NOC_CONSOLE.md, CALIX_PROVISIONING.md, OSP_DFN_GUIDE.md,
  GAME_DESIGN.md advanced track (commits 159060d, e341d10). Closed #25–28.
- Names: Calista/Amara/Terra → Clara/Aino/Hikari everywhere (commit e341d10).
- TODO files linked to GitHub issues (commit 989eec1).
- TDS resources vendored: 87 docs at docs/reference/tds/ + gamification
  design docs/TDS_GAMIFICATION.md (commit f9b4fcd). Issues #34–37.
- Expression sources vendored: assets/art/expressions/ (commit 2af2d6a).
- **Clara's provisioning track BUILT** (2026-10-05 ~19:15): ServiceProfile
  model + SERVICE_PROFILES catalog in game/src/level.rs; SubscriberDef with
  EXOS reg_id; verify_provisioning(); bulk mechanics (splitter 1:4–1:32,
  port-count enforcement, Rx window verification); 2 playable levels
  (clara1_provisioning.json, clara2_outage.json); Clara.track_start_index()
  → Some(8). 391 tests passing (+12). Closed #16, #17.
- **Portraits regenerated**: Clara (younger revision), Aino, Hikari —
  assets/art/companions/, magenta key, specialty hair accessories
  (Clara: fiber-connector pin; Aino: waveform clip; Hikari: fiber strand).
  Uncommitted, Matt reviewing.
- bloch memory wired (.ai/memory/, .ai/skills/tiger-style-rust).

## In progress

- Game code: NOC console deferred (stashed), unlock system (#13–15),
  Hikari MST rendering (#18) — after Clara's track.
- Art: mature variants, expressions, idle strips for Clara/Aino/Hikari
  (#19–21) — needs portraits approved first.
- Portraits: Aino/Hikari approved; Clara younger revision pending review.

## Model policy (Matt, 2026-10-05)

- Mature models default everywhere in-game.
- Base models ONLY: title screen, character picker.
- All test footage, animations, README GIFs: mature models.

## Matt-only (★)

- #30 Play Console app + keystore; #31 service-account invite;
  #32 fdroiddata MR; #33 Windows installer decision.
