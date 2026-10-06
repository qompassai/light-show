# Defects 6–11 fix: non-obvious findings (candidate memory)

## Headless nvim LSP diagnostics are vacuous for Rust on primo
- In Matt's diver config, `rustana_ls` (rust-analyzer) runs with
  `diagnostics.enable = false` and `checkOnSave = false`; diagnostics are
  delegated to `bacon_ls`.
- `bacon_ls` never attaches in agent shells: `bacon-ls` is not on PATH
  (`which bacon-ls` → none; `/usr/bin/bacon` exists).
- Positive control: injecting `let _canary: u32 = "canary";` into
  `game/src/cheat_codes.rs` still gave `ERROR_COUNT=0` after 240s.
  So "ERROR_COUNT=0" from the standard recipe proves nothing for `.rs`.
- Equivalent real signal: `cargo clippy --workspace --all-targets` (the
  exact bacon-ls job command in the config).
- JSON files: `json_ls` + `biome_ls` attach; `spectral` always reports 1
  ERROR "No ruleset has been found" — environmental, also on untouched files.

## Pre-existing lint/fmt debt blocks `-D warnings` and `fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings` fails on
  pre-existing code: `too_many_arguments` (outage.rs:310, playing.rs:277),
  `type_complexity` (results.rs:135), `assertions_on_constants`
  (audio.rs:566-567), and ~35 `unused_must_use` from
  `world.run_system_once(...)` in test modules (Bevy now returns Result).
- Allowing exactly those four classes makes clippy clean, which isolates
  new lints.
- `cargo fmt --check` is dirty across ~19 files incl. crates/; files
  arguments after `--` are ignored by `cargo fmt`.

## Design facts
- Konami KMP failure table is `[0,1,0,...,0]`: only the leading Up,Up
  self-overlaps; Left,Right at 4..8 is NOT a prefix overlap.
- `Companion::track_start_index()` now returns `Option<usize>`; specialists
  are `None` and the select screen refuses them (sole gate into Playing).
- `DialogueBank` is `#[serde(transparent)]` (flat JSON shape).

## Smoke test recipe
- `xvfb-run -a timeout -s INT 45 ./target/debug/light-show` (timeout
  INSIDE xvfb-run). `timeout ... xvfb-run ...` signals the wrapper only,
  so the game hangs.

Validation: `cargo test --workspace` (all suites green, 0 ignored).
