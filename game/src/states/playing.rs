//! Core puzzle-solving state: renders the OSP node graph, lets the player
//! drag/tap to place components on open edges (see `crate::board`), keeps
//! a live `osp_sim::PathGraph` in sync, and watches the scripted outage
//! clock.

use super::outage::ActiveOutage;
use super::{GameState, LevelOutcome};
use crate::board;
use crate::level::{self, CurrentLevelIndex, LevelDef};
use crate::test_log;
use crate::waifu::dialogue::DialogueBank;
use bevy::prelude::*;
use osp_sim::{Outage, PathGraph, Wavelength};

pub struct PlayingPlugin;

impl Plugin for PlayingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LiveGraph::default())
            .insert_resource(LevelClock::default())
            .insert_resource(CurrentLevelIndex::default())
            .insert_resource(board::PlacedChoices::default())
            .insert_resource(board::DragState::default())
            .insert_resource(board::PointerWorld::default())
            .insert_resource(LevelOutcome::default())
            .add_systems(OnEnter(GameState::Playing), setup_level)
            // Board teardown moved off `OnExit(Playing)`: that would also
            // fire on every Playing -> OutageActive transition (the board
            // and ledger UI need to stay alive and interactive through an
            // active outage repair). Teardown now happens on the ways a
            // level run genuinely ends instead: `OnEnter(Results)` (see
            // `results::show_results`) and `OnEnter(MainMenu)` (see
            // `menu::setup_menu`).
            .add_systems(
                Update,
                (
                    board::track_pointer,
                    board::handle_pointer_input,
                    board::draw_board_gizmos,
                )
                    .run_if(
                        in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                    ),
            )
            .add_systems(
                Update,
                (tick_clock, check_scripted_outage, check_win_condition)
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
/// correct for "start/restart a level" and *also* correct now for the
/// outage-repair loop: `GameState::OutageActive` never transitions back
/// into `Playing` for the same level (see `outage::check_outage_resolution`
/// — it always resolves straight to `Results`, on both a successful repair
/// and a timeout), so this system only ever runs when a level is genuinely
/// starting or restarting from `MainMenu` or `Results`. There's no path
/// left where a full reset here could clobber in-progress repair state.
///
// One parameter per input/state resource this glue needs to read or
// mutate; splitting it up would just move the same resource list into an
// artificial bag type for no clarity gain (see `board::handle_pointer_input`
// for the same tradeoff).
#[allow(clippy::too_many_arguments)]
fn setup_level(
    mut commands: Commands,
    index: Res<CurrentLevelIndex>,
    mut live: ResMut<LiveGraph>,
    mut placed: ResMut<board::PlacedChoices>,
    mut clock: ResMut<LevelClock>,
    mut active_outage: ResMut<ActiveOutage>,
    asset_server: Res<AssetServer>,
    dialogue: Res<DialogueBank>,
    mut companions: Query<&mut crate::waifu::CompanionSprite>,
) {
    let level_def = level::load_level(index.0);

    placed.0.clear();
    clock.elapsed_seconds = 0.0;
    active_outage.outage = None;
    for mut sprite in &mut companions {
        sprite.mood = crate::waifu::Mood::Idle;
        sprite.frame = 0;
    }
    board::rebuild_live_graph(
        &level_def,
        &placed,
        active_outage.outage.as_ref(),
        &mut live.graph,
    );
    live.wavelength = WavelengthWrapper(level_def.wavelength.into());
    live.tx_dbm = level_def.tx_dbm;

    // The companion's on-enter flavor line, if this level defines one and
    // the active dialogue bank has a matching entry — purely cosmetic,
    // never gates play (see `crate::waifu` module docs).
    let on_enter_dialogue = level_def
        .on_enter_line
        .as_deref()
        .and_then(|key| dialogue.random_line(key));
    board::spawn_board_from_level(&mut commands, &level_def, &asset_server, on_enter_dialogue);
    // Signals the on-device instrumentation tests (android/app/src/androidTest)
    // that the board has finished spawning and is ready to receive touch
    // input — they poll Logcat for this line before injecting gestures.
    // See `test_log!` in lib.rs and docs/BUILD.md.
    test_log!("level_ready id={} index={}", level_def.id, index.0);

    commands.insert_resource(level_def);
}

fn tick_clock(time: Res<Time>, mut clock: ResMut<LevelClock>) {
    clock.elapsed_seconds += time.delta_seconds_f64();
}

/// Fires the level's scripted outage (if any) once its clock threshold is
/// reached: builds the `Outage`, stores it in `ActiveOutage`, and
/// immediately rebuilds the live graph so a full-cut hazard is reflected
/// in the ledger the instant it fires rather than waiting for the next
/// player interaction to trigger a rebuild.
fn check_scripted_outage(
    clock: Res<LevelClock>,
    level: Res<LevelDef>,
    placed: Res<board::PlacedChoices>,
    mut live: ResMut<LiveGraph>,
    mut active_outage: ResMut<ActiveOutage>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(scripted) = &level.scripted_outage else {
        return;
    };
    if active_outage.outage.is_some() {
        return;
    }
    if clock.elapsed_seconds >= scripted.fires_after_seconds {
        let outage = Outage::new(scripted.kind.into(), scripted.edge_from, scripted.edge_to);
        active_outage.outage = Some(outage);
        board::rebuild_live_graph(
            &level,
            &placed,
            active_outage.outage.as_ref(),
            &mut live.graph,
        );
        next_state.set(GameState::OutageActive);
    }
}

/// Evaluates the plain (non-outage) win condition. Levels with a
/// `scripted_outage` skip this entirely and can only win through
/// `outage::check_outage_resolution` instead: without this guard, a
/// player who pre-builds the eventual protection route during normal
/// play (e.g. world4's 1->3 fusion splice, which happens to already
/// satisfy `source_node`->`target_node` before the storm ever hits at
/// 20s) would win immediately and the outage/repair content -- the whole
/// point of the level -- would never fire.
fn check_win_condition(
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    mut outcome: ResMut<LevelOutcome>,
    mut next_state: ResMut<NextState<GameState>>,
    mut companions: Query<&mut crate::waifu::CompanionSprite>,
) {
    if level.scripted_outage.is_some() {
        return;
    }
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
        outcome.won = true;
        for mut sprite in &mut companions {
            sprite.mood = crate::waifu::Mood::Celebrate;
            sprite.frame = 0;
        }
        next_state.set(GameState::Results);
    }
}
