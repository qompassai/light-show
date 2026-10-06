# Session Handoff — light-show (2026-10-05, ~18:55 PDT)

## Canonical tree
`/home/phaedrus/workspace/repos/light-show` is the working tree (branch
`main` @ 21ee32b + dirty WIP). NOTE: `/home/phaedrus/.GH/Qompass/LightShow`
is a STALE clone at 67046c3 (2026-09-21) — do not work there.

## Done today (all on primo, machine-verified)
- **tests/ integration crate**: 7 targets, 123 tests (51 validation / 78
  adversarial). Validates physics end-to-end vs docs/GAME_DESIGN.md, game
  levels/cheats/dialogue/audio, all README image refs decode, packaging
  structure. `cargo clippy -p light-show-tests` and `cargo fmt --check` clean.
- **Run-failure fixed** (uncommitted): bevy_audio decode panic guard +
  `asset_root` module so installed builds find `assets/`. xvfb smoke test:
  0 panics.
- **11 defects fixed** (uncommitted, all BUG tests un-ignored and green):
  osp_sim — NaN lengths fail closed, negative passive losses clamped,
  iterative DFS + PathTooDeep (was SIGABRT), Ethernet NaN violations,
  outage elapsed_seconds clamped at deserialization; game — KMP Konami
  fallback, progress field privatized, B/A buffer isolation, specialist
  track index -> Option, flat dialogue JSON, world4 file world=4.
- **Relicensed GPL-3.0-or-later -> Apache-2.0** (Matt's explicit decision):
  full Apache 2.0 LICENSE (Copyright 2026 Qompass AI), Cargo.toml,
  README, CREDITS, FDROID, BUILD, deny.toml (copyleft now denied by
  omission), in-game credits string, import_sheet.lua SPDX, structural.rs
  fixtures.

## Test status
`cargo test --workspace`: **379 passed, 0 failed, 0 ignored**.
Pre-existing clippy/fmt debt untouched (workers added zero new warnings).
Headless-nvim LSP gate is VACUOUS on primo: bacon-ls not on
non-interactive PATH, so ERROR_COUNT=0 is meaningless — fix the recipe
(export PATH="$HOME/.cargo/bin") before trusting it.

## Pending Matt
- **"looks good, move it"** for commit+push of the whole dirty tree
  (tests/ + fixes + relicense). No commit/push done.
- UX: clicking an unlocked specialist card now does nothing — he may want
  a "track coming soon" affordance.
- Pre-existing clippy/fmt debt deliberately left alone (would balloon diff).

## Refreshing this memory
`export PATH="$HOME/.cargo/bin:$PATH" && bloch . --update-memory`
from the repo root rewrites `.ai/memory/bloch-last.json` +
`bloch-summary.md`. Update this file when session state changes.
