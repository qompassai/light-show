# Architecture

## In plain terms

The game is two crates with a clean split: **`osp_sim`** is the
physics textbook — pure math about light and loss, no graphics, no
game engine, testable in seconds. **`game`** is the Bevy application
— windows, sprites, buttons, timers, music — that *uses* the physics
to decide wins and losses. The textbook never knows the game exists;
the game reads the textbook constantly.

## The workspace

```toml
[workspace]
members = ["crates/osp_sim", "game"]
```

- `crates/osp_sim` — engine-agnostic link-budget simulation core.
- `game` — the Bevy app: states, UI, level loading, companions,
  audio. Ships as a desktop binary (`src/main.rs` → `run()`) and as
  an Android `cdylib` (`#[bevy_main] fn main()` loaded by Bevy's
  `game-activity` glue).

## One function builds the whole app

Both entry points converge on `game/src/lib.rs::build_app()` —
"exactly one place that configures the App":

```rust
app.add_plugins(DefaultPlugins.set(WindowPlugin {
    primary_window: Some(Window {
        title: "Light Show".into(),
        resolution: (720.0_f32, 1280.0_f32).into(), // portrait, phone-shaped
        ..default()
    }),
    ..default()
}))
.init_state::<GameState>()
.add_plugins((
    states::menu::MenuPlugin,
    states::credits::CreditsPlugin,
    states::playing::PlayingPlugin,
    states::outage::OutagePlugin,
    states::results::ResultsPlugin,
    waifu::SeraphinePlugin,
    ui::LedgerUiPlugin,
    audio::MusicPlugin,
));
```

Eight plugins, each owning one concern. (The actual eighth plugin is `states::credits::CreditsPlugin`; the
listing above is schematic but the membership is exact.)

## The state machine

`states::GameState` is the spine. Each variant owns its plugin, so
its systems are *only scheduled while that state is active* — the
menu's buttons don't exist during play, the outage timer doesn't tick
in the menu:

- `MainMenu` — title, companion picker, Start, Credits.
- `Playing` — normal puzzle solving; no active outage.
- `OutageActive` — fault fired, repair timer running.
- `Results` — win/fail banner, ledger summary, companion line.
- `Credits` — code/art/font credits + shipped music attribution
  (a real state, not an overlay, because CC-BY attribution must be
  user-visible).

Two resources carry cross-state meaning: `LevelOutcome { won: bool }`
(written by whichever system triggers the `Results` transition, read
by the results screen) and `ActiveOutage { outage: Option<Outage> }`
(the in-progress fault, `None` outside an outage).

The deliberate non-transition: `OutageActive` never goes back to
`Playing`. Re-entering `Playing` runs `setup_level`, which always
does a full level reset — that would wipe an in-progress repair. So
outages resolve straight to `Results`, win or lose, and the reset
hazard disappears by construction.

## The live graph: how play talks to physics

`states::playing::LiveGraph` is the bridge:

```rust
pub struct LiveGraph {
    pub graph: PathGraph,        // osp_sim's model of the current route
    pub wavelength: WavelengthWrapper, // no Default on osp_sim's enum, so wrapped
    pub tx_dbm: f64,             // launch power for this level
}
```

Every player action (`handle_pointer_input`) and every outage event
mutates the *player's* placement map, then `board::rebuild_live_graph`
re-mirrors it into the `PathGraph`. The ledger UI and the win/outage
checks then call `graph.compute_link_budget(...)` (or
`..._with_outage(...)`) against the level's source/target nodes and
receive window. The game never caches a budget — it's recomputed from
the graph on demand, so the ledger, the win check, and the outage
resolution can never disagree.

Levels themselves are **embedded at compile time** (`include_str!` of
`game/assets/levels/*.json` in `level.rs::LEVEL_SOURCES`), so desktop
and Android run the identical code path with no APK asset-file access
at runtime.

## Testability as architecture

The split is load-bearing for testing, not just aesthetics:

- `osp_sim` has zero engine dependency — `cargo test -p osp_sim`
  runs the physics suite with no window, no GPU.
- The board's decision logic (`resolve_press`, `resolve_release`) is
  pure functions over `&LevelDef`, unit-tested directly; the Bevy
  resource plumbing (`handle_pointer_input`) is a thin wrapper.
- The music manager is pure track-selection functions
  (`menu_track`, `playing_track`, `outage_track`, `results_track`) —
  no Bevy types, unit-testable.
- On Android, a `test_log!` macro (compiled away unless the
  `instrumented-test-logging` feature is enabled) emits structured
  Logcat lines so the on-device UiAutomator tests can assert on real
  gestures against a `NativeActivity` with no view hierarchy.
