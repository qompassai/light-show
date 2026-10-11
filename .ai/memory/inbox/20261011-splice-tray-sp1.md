# 2026-10-11 — sp1 splice tray built (CONTENT-NOTES gap 4 closed)

Branch `pax/splice-tray-20261011`, commit `50b6be5` on top of
origin/main `072976d`. NOT pushed — new content beyond the Oct 7
landing authorization; landing is Matt's call.

## What

sp1 (`splice1_secondary_split`, Field School scenario shelf) already
carried the full authored `splice_work_orders` block — 144-fiber
TIA-598 chart, three beats, two damage beats — typed and validated,
but nothing consumed it: the shipped win check was the light-path
budget alone. The tray is now playable in sp1's existing slot. The
frozen count is untouched: 83 levels (80 track + fj1/fj2/sp1), 10
per companion.

- `game/src/splice.rs` — pure scoring engine. Fiber identity is the
  (tube, strand) pair; fiber number = (tube−1)×12 + strand.
  `evaluate_beat` is total (hostile shapes are ledger faults, never
  panics): a wrong splice lights the WRONG drop, the ordered drop
  stays dark, 2-cycle swaps get an explicit "Swapped legs" ledger
  line naming both fibers with full pairs and both drops. Damaged
  strands are never forced; the designated spare substitutes
  (general rule — beat 3's 55/118 are tray hazards to exclude; a
  damaged *ordered* fiber is lit via its spare).
- `game/src/states/splice_console.rs` — tray UI in the consoles'
  established pattern (Resource progress + Plugin + OnEnter setup +
  `is_changed`-gated sync): tube pages over the tubes the beat
  touches (beat 1: [1]; beat 2: [1,2,3]; beat 3: all 12), strand and
  drop picking, Verify/Clear per beat, beats gate in order; the
  full 144-pair chart is a paged reference panel reachable
  throughout, no timer (the level clock is untouched) — fluency,
  not memorization.
- Win gate: `SpliceProgress` rides `AstraProgress` as an
  `Option<Res>` field with the gate inside `astra_gates_pass`
  (that module's documented pattern — the win checks sit at Bevy's
  16-system-param limit, so slices add fields, never parameters).
  Both win paths (playing, outage resolution) inherit it.
  `playthrough.rs` inits the resource by hand like every other
  console. sp1's win is now board budget AND all beats true.
- Stale "future work" sentences updated in the `level.rs`
  doc comment and `docs/levels/field-school.md`.

## Gates (all on primo, rebased tree)

- Suite: `cargo test --workspace` **824 passed / 0 failed**
  (origin/main baseline 806/0 + exactly the 18 new tests).
  New: splice.rs unit ×4, splice_console unit ×3,
  tests/splice_work_orders.rs integration ×11 — incl. the frozen
  count, pair identity, panel completeness (144 lines, 12 pages),
  the ramp, and the acceptance case: deliberate fiber 2 ↔ fiber 13
  transposition lights Drop 1/Drop 3 wrong, Drop 3's line names
  fiber 2, and the ledger carries "Swapped legs" naming both legs;
  no "corrupt" wording anywhere (scoring_honesty honored).
- IDE gate (headless nvim, live diver config): rust-analyzer
  0 errors on all 9 touched Rust files; crates-ls + taplo 0 on
  tests/Cargo.toml. bacon-ls did not attach headless (same
  estate-wide caveat as fathom/bunker).
- rustfmt: all new/touched files clean; the residual drift in
  lib.rs/states/mod.rs is pre-existing upstream drift (the queued
  repo-wide fmt pass stays separate).

## Note for the record

The working checkout was 6 commits behind origin/main when this
work started (level-select, struct-size pins, bloch fix, art
installs); the first gate round ran on that stale tree (795/0).
The commit was rebased onto `072976d` with no textual conflicts,
placement verified by diff in the five overlapping files, and all
gates re-run on the rebased tree (numbers above).
