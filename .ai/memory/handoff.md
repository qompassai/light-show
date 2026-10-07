# Session Handoff — light-show (updated 2026-10-07, finish-up program, Coordinator A)

## Canonical tree
`/home/phaedrus/workspace/repos/light-show` (main, remote == local; art landing pushed 2026-10-07).
Build/test with
`CARGO_TARGET_DIR=$PWD/target-agent-landing cargo test --workspace --no-fail-fast`.
Suite: 789 tests passing, 0 failures.

## Where things stand (2026-10-07)
- Phase 1 landing stack pushed: README gif fix, pill-gate, responsive
  dimensions, title art (portrait + landscape), 330-file pill face fleet
  in game/assets/art/faces (10 speakers x 11 emotions x base/talk/blink).
- Tutorials implemented for all 8 companions: banks compiled-in +
  JSON mirrors (specialists have mirrors now), pill lines carry an
  Emotion, sequencer in game/src/tutorial.rs, seen/skip flag
  `tutorials_seen` on SaveData (serde default — NEVER bump SAVE_VERSION;
  a bump orphans all existing saves, there is no migration machinery).
- Field school: fj1/fj2/sp1 live as out-of-track scenario levels
  (SCENARIO_SOURCES in level.rs, id-addressed, entry on Seraphine's
  companion card). LEVEL_SOURCES is frozen at 80; do not insert levels
  mid-track (renumbers everything).
- Astra slices 1-8 landed (merge ec5a6e5): verification states,
  identification console + c1l1 win gate, defective-edge c1l5, survey/
  diagnosis, workbench, IPv4/IPv6 config console, c1l6 intermittent
  machine, capstones + badges + closeout. `level_badges` on SaveData,
  also serde default. In-game fiction technology is G.fast for this
  content (Matt ruling 2026-10-07), not CATV.
- Art landing pass (2026-10-07, after Track A froze the tree):
  expression-portrait fleet LANDED in assets/art/expressions/ (109
  portraits + Bianca/Tessa aroused pair, byte-identical to the gated
  staging keepers; Lea angry still a labeled service-refusal gap -
  exists nowhere). They have no in-game consumer. Rebuilt Ondine +
  Linka mature sheets (6/6) + picker strips/frames installed in
  game/assets/sprites (Linka's installed sheet is no longer the
  rejected ponytail design). README idle GIFs deployed from the gated
  strips (Seraphine/Ondine/Linka 12f @200ms; Lattice v11 16f, 2650ms
  loop). Quarantined out of tree (manifest in the finish-up
  quarantine dir): ondine_animated_base.gif (design-A violation),
  seraphine_animated.gif (duplicate). FLAGGED, NOT MOVED:
  clara/aino/hikari portrait_mature.jpg are the live dialogue
  portraits (Companion::portrait_path, unit-test asserted) -
  code-loaded carriers stay until a replacement lands at the path.

## Open questions for Matt
- (carried by the finish-up report's starred items: release signing,
  Play upload, real-device checks, F-Droid MR)
