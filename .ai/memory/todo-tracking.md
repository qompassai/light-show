# TODO Tracking — light-show (persistent)

GitHub: master [#2](https://github.com/qompassai/light-show/issues/2),
tracks #3 (game) #4 (art) #5 (scripts) #6 (docs) #7 (store),
leaf issues #8–#33. Closing a leaf issue checks off the item.

## Completed (2026-10-05)

- UX: TRACK COMING SOON badge + click SFX on trackless specialist cards
  (`game/src/states/companion_select.rs`, commit 7cd4920).
- Docs: NOC_CONSOLE.md, CALIX_PROVISIONING.md, OSP_DFN_GUIDE.md,
  GAME_DESIGN.md advanced track (commits 159060d, e341d10). Closed #25–28.
- Relicense GPL-3.0-or-later → Apache-2.0 (commit 50c6110).
- Tests: 379 passing; 11 defects fixed (commit e1e26a5).
- bloch memory wired (.ai/memory/, .ai/skills/tiger-style-rust).

## In progress

- Game code (issues #8–18): NOC console, unlock system, Clara/Aino/Hikari
  mechanics — coordinator working, tree dirty.
- Art (issues #19–21): BLOCKED — source portraits missing from primo;
  last state (2026-10-04) had disputed quality (Aino side-scroll,
  breathing artifacts). Needs Matt's direction.

## Model policy (Matt, 2026-10-05)

- Mature models are the default everywhere in-game.
- Base models ONLY for: title screen, character picker.
- All test footage, animations, README GIFs: mature models.

## Matt-only (★)

- #30 Play Console app + keystore upload; #31 service-account invite;
  #32 fdroiddata MR (needs GitLab); #33 Windows installer decision.
