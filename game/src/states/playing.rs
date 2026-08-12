//! Core puzzle-solving state: renders the OSP node graph, lets the player
//! drag/tap to place components on open edges (see `crate::board`), keeps
//! a live `osp_sim::PathGraph` in sync, and watches the scripted outage
//! clock.

use super::GameState;
use crate::board;
use crate::level::{self, CurrentLevelIndex, LevelDef};
use crate::test_log;
use bevy::prelude::*;
use osp_sim::{PathGraph, Wavelength};

pub struct PlayingPlugin;

impl Plugin for PlayingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LiveGraph::default())
            .insert_resource(LevelClock::default())
            .insert_resource(CurrentLevelIndex::default())
            .insert_resource(board::PlacedChoices::default())
            .insert_resource(board::DragState::default())
            .insert_resource(board::PointerWorld::default())
            .add_systems(OnEnter(GameState::Playing), setup_level)
            .add_systems(OnExit(GameState::Playing), board::teardown_board)
            .add_systems(
                Update,
                (
                    board::track_pointer,
                    board::handle_pointer_input,
                    board::draw_board_gizmos,
                    tick_clock,
                    check_scripted_outage,
                    check_win_condition,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// The player's current in-progress graph, mirrored from `LevelDef` plus
/// whatever edges they've placed so far. Wrapped as a resource so UI and
/// simulation systems can both read/write it without a full ECS redesign.
#[derive(Resource, Default)]
pub struct LiveGraph {
    pub graph: PathGraph,
    pub wavelength: WavelengthWrapper,
    pub tx_dbm: f64,
}

// Bevy resources need a concrete default; osp_sim::Wavelength has no
// Default impl (physically all three are valid "defaults"), so we wrap it.
pub struct WavelengthWrapper(pub Wavelength);
impl Default for WavelengthWrapper {
    fn default() -> Self {
        WavelengthWrapper(Wavelength::Nm1490)
    }
}

#[derive(Resource, Default)]
pub struct LevelClock {
    pub elapsed_seconds: f64,
}

/// Loads the current level, resets placements, rebuilds the live graph,
/// and spawns the board + ledger UI — all synchronously in one system so
/// there's no risk of other `OnEnter(Playing)`/`Update` systems observing
/// a half-initialized state within the same frame. `LevelDef` is inserted
/// as a resource only at the very end: Bevy flushes `Commands` at the end
/// of the state-transition schedule, before `Update` runs later in the
/// same frame, so every `Update` system can safely take a plain
/// `Res<LevelDef>` instead of `Option<Res<LevelDef>>`.
///
/// Note: this always does a full reset on every `OnEnter(Playing)`. That's
/// correct for "start/restart a level" but would also wipe progress if a
/// future change makes `OutageActive` route back into `Playing` for the
/// *same* level rather than resolving into `Results` — the outage-repair
/// loop isn't wired up to player action yet, so that's a follow-up rather
/// than a concern for this change.
fn setup_level(
    mut commands: Commands,
    index: Res<CurrentLevelIndex>,
    mut live: ResMut<LiveGraph>,
    mut placed: ResMut<board::PlacedChoices>,
    mut clock: ResMut<LevelClock>,
    asset_server: Res<AssetServer>,
) {
    let level_def = level::load_level(index.0);

    placed.0.clear();
    clock.elapsed_seconds = 0.0;
    board::rebuild_live_graph(&level_def, &placed, &mut live.graph);
    live.wavelength = WavelengthWrapper(level_def.wavelength.into());
    live.tx_dbm = level_def.tx_dbm;

    board::spawn_board_from_level(&mut commands, &level_def, &asset_server);
    // Signals the on-device instrumentation tests (android/app/src/androidTest)
    // that the board has finished spawning and is ready to receive touch
    // input — they poll Logcat for this line before injecting gestures.
    // See `test_log!` in lib.rs and docs/BUILD.md.
    test_log!("level_ready index={}", index.0);

    commands.insert_resource(level_def);
}

fn tick_clock(time: Res<Time>, mut clock: ResMut<LevelClock>) {
    clock.elapsed_seconds += time.delta_seconds_f64();
}

fn check_scripted_outage(
    clock: Res<LevelClock>,
    level: Res<LevelDef>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(outage) = &level.scripted_outage else {
        return;
    };
    if clock.elapsed_seconds >= outage.fires_after_seconds {
        next_state.set(GameState::OutageActive);
    }
}

fn check_win_condition(
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(result) = live.graph.compute_link_budget(
        level.source_node,
        level.target_node,
        live.tx_dbm,
        live.wavelength.0,
        level.receive_window(),
    ) else {
        return;
    };
    if result.in_window {
        next_state.set(GameState::Results);
    }
}
