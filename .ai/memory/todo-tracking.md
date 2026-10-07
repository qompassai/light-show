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
- **Portraits APPROVED and committed** (2026-10-05, commit ef91526):
  Clara, Aino, Hikari — assets/art/companions/, magenta key, specialty
  hair accessories (Clara: fiber-connector pin; Aino: waveform clip;
  Hikari: fiber strand). Matt: "all three look great".
- bloch memory wired (.ai/memory/, .ai/skills/tiger-style-rust).

## In progress

- Game code: NOC console deferred (stashed), unlock system (#13–15),
  Hikari MST rendering (#18) — after Clara's track.
- Art: mature variants, expressions, idle strips for Clara/Aino/Hikari
  (#19–21) — needs portraits approved first.
- Portraits: all three approved and committed. Next: mature variants,
  expressions, idle strips (#19-21).

## Model policy (Matt, 2026-10-05)

- Mature models default everywhere in-game.
- Base models ONLY: title screen, character picker.
- All test footage, animations, README GIFs: mature models.

## Matt-only (★)

- #30 Play Console app + keystore; #31 service-account invite;
  #32 fdroiddata MR; #33 Windows installer decision.

## Overnight program (2026-10-05 ~20:05 PDT, Matt authorized)
Matt: "continue to iterate on lightshow until it is fully finished and
polished ready to be run on my linux desktop, and publish to fdroid and
google play store. everything gets tested, validated and curatively fixed
for issues, persistent memory updated, and commit/push as needed."

- **Model terminology** (Matt directive): chibi = current style (picker/
  title); mature = young adult (early 20s, NOT aged up) for HD interactions.
  Three young-adult portraits regenerated, Matt approved ("these look
  great"), committed (young-adult set; aged-up set discarded).
- **Dialogue UI** (MMBN-style): built, 9 tests, 400 passing, committed.
  Portrait box + typewriter + blips + advance. Uses mature portraits.
- **Level expansion**: content supports ~40-48 levels (not 10). Building
  character-by-character: Clara (cms/ 6 guides -> 8-10) -> Aino (ams/ 14
  topics -> 8-10) -> Hikari (bxe/ 6 guides -> 7-9) -> Lea (ta/ 12 quiz
  files -> 8-10). Clara expansion in progress.
- **Footage**: capturing all 10 levels to game/assets/footage/<level-id>/
  (in progress).
- **Publication**: F-Droid Gates 1-4 + Play prep in progress. Gate 5
  (GitLab MR) + signing keys + Play upload are Matt-only.


## Completed (2026-10-07, finish-up program)
- Phase 1 landing: README gif fix, pill-gate patch, responsive
  dimensions, title art, 330-file pill face fleet (game/assets/art/faces).
- Tutorials for all 8 companions (banks + mirrors, pill Emotion
  transport, tutorial.rs sequencer, tutorials_seen save field).
- Field school scenarios fj1/fj2/sp1 (SCENARIO_SOURCES, out-of-track).
- Astra slices 1-8 merged (verification states, consoles, badges;
  level_badges save field; G.fast fiction per Matt ruling).
- Art landing: expression fleet (109 + aroused pair) in
  assets/art/expressions/; Ondine + Linka sheets/pickers (6/6)
  installed; README idle GIFs deployed (Lattice v11 16f); two
  non-canonical GIFs quarantined out of tree.
- Trailer sting pilot (spec section A) rendered as code animation
  and landed with the README Trailer section (docs/media/ mp4 +
  preview gif).
- Suite: 789 tests passing, 0 failures. Remote main == f976ab5.
