# Crate Tour

A module-by-module map of the codebase as it exists today. File paths
are relative to the repo root.

## `crates/osp_sim` — the physics

Engine-free, dependency-light (serde for level JSON), unit-tested in
isolation.

### `src/lib.rs`

Crate root and the shared vocabulary: re-exports (`Component`,
`ConnectorType`, `SpliceType`, `PathGraph`, `LinkBudgetResult`,
`Outage`, `OutageKind`, `Wavelength`), `DEFAULT_TX_DBM = 3.0`
(standard GPON downstream launch power, ITU-T G.984.2 class B+), and
`ReceiveWindow { min_dbm, max_dbm }` with `GPON_ONT = [−27, −8]`,
`contains()`, and `margin()` — positive margin means comfortably
inside the window (distance to the nearest edge), negative means
outside (magnitude = how far out).

### `src/component.rs`

The puzzle pieces. `Component` is an enum with real-world loss per
variant via `loss_db(wavelength)`:

- `Span { length_km, plant }` — fiber run; loss is length × the
  wavelength's attenuation. `PlantType` (Aerial/Buried/Conduit) is
  currently cosmetic/hazard-flavor, reserved for future weighting.
- `Splice { kind, degradation_db }` — `Fusion` 0.075 dB / 45 s place
  time vs. `Mechanical` 0.4 dB / 10 s place time. The time/quality
  tradeoff is the game's central tension under an outage clock.
- `Connector { kind, contamination_db }` — `Upc` 0.35 dB / −50 dB
  return loss vs. `Apc` 0.30 dB / −60 dB return loss; contamination
  adds loss a "clean the connector" interaction can zero out.
- `Splitter { ratio }` — 1:2 through 1:32, real PLC insertion-loss
  figures (3.6 → 17.7 dB).
- `Macrobend { excess_loss_db }` — a kink hazard, player-introduced
  or level-seeded.

### `src/graph.rs`

`PathGraph`: nodes + connected edges, `compute_link_budget(source,
target, tx_dbm, wavelength, window)` returning `LinkBudgetResult
{ total_loss_db, received_dbm, in_window, margin_db }`, plus
`compute_link_budget_with_outage(...)` which layers an `Outage`'s
extra loss (or full cut) onto the result. `received_dbm = tx_dbm −
Σ losses` — the one equation the whole game hangs on.

### `src/outage.rs`

`OutageKind` — `FiberCut`, `AerialDamage` (both `is_full_cut()`),
`WaterIntrusion`, `ConnectorContamination`, `Macrobend` — each with
`flavor_text()` and `base_timer_seconds()` (90/75/120/60/60). The
`is_full_cut` match is deliberately exhaustive: a future variant must
make an explicit choice. `Outage` tracks elapsed time; `WaterIntrusion`
accumulates +1 dB per 10 s up to +15 dB via
`accumulated_extra_loss_db()`.

### `src/wavelength.rs`

`Wavelength::{Nm1310, Nm1490, Nm1550}` — 1310 nm O-band/PON upstream
(0.35 dB/km), 1490 nm GPON downstream data (0.28 dB/km), 1550 nm
C-band/RF video overlay (0.21 dB/km), with human-readable `label()`s.

## `game` — the Bevy app

### `src/main.rs` / `src/lib.rs`

Desktop `main()` → `run()` → `build_app()`; Android `#[bevy_main]
fn main()` (name fixed by the macro) → `build_app().run()`. Also home
to the `test_log!` macro for the on-device instrumentation tests.

### `src/board.rs`

Board rendering + input. `spawn_board_from_level` builds entities from
a `LevelDef`; `track_pointer` maps mouse cursor / first touch into
world space; `handle_pointer_input` turns gestures into placements via
the pure `resolve_press`/`resolve_release`; `draw_board_gizmos` renders
nodes, fixed edges, dashed choice edges, pills, and the drag preview.
Resources: `PlacedChoices` (the player's (from, to) → slot map),
`DragState`, `PointerWorld`. Constants worth knowing: node hit radius
50, pill radius 34, pill spread 70, grid→world mapping
`(x−1)·200, 300−y·200`. The board palette (`#1c2541` background line,
`#5bc0eb` accent, `#ffd166` warm, `#ff6fae` hot, `#ff4d4d` hazard) is
the game's visual identity.

### `src/level.rs`

`LevelDef` (nodes, fixed edges, `available_components`, source/target,
tx/wavelength/window, `scripted_outage`, dialogue keys), `LEVEL_SOURCES`
(compile-time-embedded JSON), `load_level(index)` (clamped), and
`CurrentLevelIndex`.

### `src/states/` — the state machine

- `mod.rs` — `GameState` enum + `LevelOutcome` resource.
- `menu.rs` — title, companion picker (4 buttons, Séraphine default),
  Start → `Playing`, Credits → `Credits`. Buttons react to
  `Interaction::Pressed` only.
- `playing.rs` — `setup_level` (full reset, board camera, live graph
  rebuild, companion to idle), `tick_clock`, `check_scripted_outage`
  (fires the outage, transitions to `OutageActive`),
  `check_win_condition` (skipped for outage levels; in-window →
  `Results`, companion celebrates). Owns `LiveGraph`, `LevelClock`.
- `outage.rs` — alarm banner + countdown, companion to alarmed,
  `tick_outage`, `check_outage_resolution` (repaired → win with a
  wink; expired → loss; both → `Results`, never back to `Playing`).
- `results.rs` — banner (`SERVICE RESTORED` / `OUTAGE TIMED OUT`),
  final ledger, companion line, +10 cores on wins, Continue / Retry /
  Main Menu buttons.
- `credits.rs` — code/art/font credits + compile-time-embedded music
  attribution; Back → `MainMenu`.

### `src/ui/mod.rs`

`LedgerUiPlugin`: the OTDR-styled live readout —
`Loss / Rx / Window / IN-OUT WINDOW / OUTAGE countdown / Cores`,
updated every frame in `Playing` and `OutageActive`.

### `src/audio.rs`

`MusicPlugin`: per-state track spawn on enter / despawn on exit
(Bevy guarantees exit-before-enter ordering, so tracks never overlap).
Pure track-selection functions: menu (Eric Skiff), playing (tiered by
world: tutorial Komiku → early Skiff → mid Skiff → hard MacLeod /
SubspaceAudio, two-track tiers rotate by level index),
outage (TeknoAXE / SubspaceAudio alternate per outage count), results
(victory Skiff / defeat MacLeod). Attribution ships in
`game/assets/music/` and on the credits screen.

### `src/waifu/`

The companions: `Companion` enum (Séraphine/Ondine/Linka/Lattice),
`SelectedCompanion` resource, `CompanionSprite` with moods
(Idle/Celebrate/Alarmed/Wink/…) and 64×64 sprite sheets with per-mood
accent tinting, `DialogueBank` (per-companion JSON, localizable),
`Cores` (+10 per clean win, spendable on hints),
`respawn_on_companion_change`, and `SpliceReaction` events fired from
`handle_pointer_input` so she reacts to placements.

## Data flow at a glance

```text
level JSON ──include_str!──▶ LevelDef ──setup_level──▶ board entities
Player gesture ──resolve_press/release──▶ PlacedChoices ──rebuild──▶ PathGraph (LiveGraph)
LiveGraph + Outage ──compute_link_budget(_with_outage)──▶ ledger UI / win check / outage resolution
```

Physics flows one way — player action → graph → budget → UI/verdict —
with no cached budgets anywhere, so every consumer always agrees.
