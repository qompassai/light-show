# Warehouse feature wired end to end (uncommitted)

Affected paths: game/src/states/warehouse.rs (new), game/src/warehouse/mod.rs
(new, prior worker), game/src/{lib.rs, save.rs, board.rs, waifu/mod.rs},
game/src/states/{mod.rs, menu.rs, outage.rs, results.rs}.

Non-obvious facts:
- `warehouse::Loadout` is owned by `WarehousePlugin` (init + Startup seed
  from SaveData). It is refreshed on purchase, on spare-remote use, and at
  outage start (outage.rs). results.rs reads it as `Option<Res<Loadout>>`
  so the headless playthrough harness (no WarehousePlugin) still works.
- No hint Favor charge exists anywhere. `Loadout::hint_free` is computed
  (and tested) but nothing reads it yet; see the doc on `FavorPoints`.
- Spare remotes are honored only in the Warehouse quiz, at most one per
  run: a forgiven miss doesn't count and the question stays open. Level
  quizzes (states/quiz.rs) do not consume remotes yet.
- `save::tests::ENV_LOCK` is `pub(crate)` inside a private `mod tests`, so
  other modules can't reach it. Tests that would write a real save must
  avoid the handler path or make that module visible.
- The baseline was not clean on `cargo fmt` or `cargo clippy -D warnings`
  before this work: rustfmt reformats footage.rs, level.rs, and tests/*.rs.
  Clippy has 11 lib errors (type_complexity x9, too_many_arguments in
  footage.rs, needless borrow in playing.rs), and in --all-targets runs
  test code also ignores `run_system_once` results.

Validation: `cargo test` (570 passed, 0 failed),
`cargo test --lib -- states::warehouse` (15 passed), `cargo build` ok.
