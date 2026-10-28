# 2026-10-27 — Animated previews for every level (D-182)

Owner commission: reconcile the trees, commit + push, and give EVERY
level an animated preview in the shipped format.

## What landed

- Primo tree was 29 behind / 0 ahead of origin (not diverged):
  fast-forwarded to 474fd26. Nothing of the brief's "unpushed stack"
  remained to push — it had landed via a merge from another checkout.
- `feat(footage)` (2dc567d): the footage harness can now capture the
  three Field School scenarios. Scenario seeding via
  `CurrentScenarioId` (they are deliberately outside LEVEL_SOURCES),
  new Splice/SpliceVerify driver actions, sp1's script derived from
  its own work orders (all three beats verify true; the run wins).
  w4l1's protection placement 1500 -> 1000 frames (wall-clock outage
  vs frame-indexed script).
- `docs(gameplay)` (d29ae0b): 81 new previews (244 files) + README
  Gameplay index of all 83 levels + docs/levels pointer.
  docs/gameplay now holds 254 files / 149 MB. Captures are the real
  game under Xvfb (llvmpipe), silent, sampling normalized to elapsed
  game-frames so pacing matches the shipped pair. Outcomes from the
  game logs: 40 won, 33 budget-run showcases, 8 outage losses (all
  outage-repair levels — see follow-on), 2 shipped.
- Gates: cargo test --workspace 829/0 before and after; clippy
  (resolved 0.1.99) shows no new warnings from footage.rs (repo
  baseline carries pre-existing warnings: 51 lib / 101 all-targets).

## Follow-ons (not done here)

- Harness cannot win outage-REPAIR levels: resolution runs through
  the interactive NOC alarm ack/dispatch flow, which the driver's
  direct PlacedChoices writes bypass. 8 previews end on the outage
  timeout screen (real gameplay of the level's signature event).
  Fix = teach the driver the ack/dispatch interaction.
- Sandbox tree's uncommitted in-app-MCP changeset (crates/osp_mcp,
  mcp_dev, D-118) does NOT port cleanly: it targets Bevy 0.14 while
  main is on 0.19, its game/src/fonts.rs conflicts with the tracked
  fonts.rs on main, and its base (41fb988) is not an ancestor of
  main. Needs a real port, not an apply — left untouched in the
  sandbox tree.
- Capture-environment note: rodio's stream can fall into an ALSA
  POLLERR log loop under load (null-class PCM; never audible, sink
  watcher clean). Mitigation used: 200 MB log cap in the watcher +
  one retry; standalone re-runs are clean.
