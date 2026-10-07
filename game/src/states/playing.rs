//! Core puzzle-solving state: renders the OSP node graph, lets the player
//! drag/tap to place components on open edges (see `crate::board`), keeps
//! a live `osp_sim::PathGraph` in sync, and watches the scripted outage
//! clock.

use super::outage::{ActiveOutage, AlarmList};
use super::{GameState, LevelOutcome};
use crate::anim::TransitionRequest;
use crate::board;
use crate::level::{self, CurrentLevelIndex, LevelDef};
use crate::test_log;
use crate::waifu::trigger_mood_pop;
use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
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
            .insert_resource(board::PulseSpawnTimer::default())
            .insert_resource(LevelOutcome::default())
            .add_systems(
                OnEnter(GameState::Playing),
                // The console needs LevelDef, which setup_level inserts:
                // chain them so the order is deterministic.
                (
                    setup_level,
                    super::api_console::setup_api_console,
                    super::triage_console::setup_triage_console,
                    super::identification::setup_identification_console,
                    super::quiz::setup_quiz_ui,
                )
                    .chain(),
            )
            // Leaving a run restores the home camera framing the menu
            // and results expect (see `restore_home_framing`).
            .add_systems(OnEnter(GameState::Results), restore_home_framing)
            .add_systems(OnEnter(GameState::MainMenu), restore_home_framing)
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
                    board::update_pill_rings,
                    board::animate_pill_pops,
                    board::spawn_signal_pulses,
                    board::move_signal_pulses,
                    board::sync_fiber_flows,
                    board::update_storm_rain,
                    refit_framing_on_resize,
                )
                    .run_if(
                        in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                    ),
            )
            .add_systems(
                Update,
                (tick_clock, check_scripted_outage, check_win_condition)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, lerp_camera);
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
/// Seconds to lerp the persistent camera to the board framing (Finding 3).
const CAMERA_LERP_SECS: f32 = 0.5;
/// The home framing: where the camera rests outside a level (menu,
/// results) — centered horizontally at 1:1, raised so the board's
/// upper rows sit mid-frame with the briefing text clear at the top.
/// Levels ease from here to their fit framing and back (see
/// `board::board_framing` and `restore_home_framing`).
const BOARD_CAM_POS: Vec3 = Vec3::new(0.0, 200.0, 0.0);

/// Persistent-camera lerp marker (Finding 3). Inserted on the existing
/// camera by `setup_level`; `lerp_camera` eases `from` -> `to` and then
/// removes this component.
#[derive(Component)]
struct CameraLerp {
    from: Vec3,
    to: Vec3,
    from_scale: f32,
    to_scale: f32,
    elapsed_secs: f32,
}

/// Eases the persistent camera from its current framing to the board
/// framing over 0.5s with CubicInOut (Finding 3).
fn lerp_camera(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Projection, &mut CameraLerp)>,
) {
    for (entity, mut transform, mut projection, mut lerp) in &mut query {
        lerp.elapsed_secs += time.delta_secs();
        let t = (lerp.elapsed_secs / CAMERA_LERP_SECS).clamp(0.0, 1.0);
        let eased = EaseFunction::CubicInOut.sample_clamped(t);
        transform.translation = lerp.from.lerp(lerp.to, eased);
        // The fit framing's zoom eases with the glide, so entering a
        // wide level feels like pulling back, not a cut.
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = lerp.from_scale + (lerp.to_scale - lerp.from_scale) * eased;
        }
        if t >= 1.0 {
            transform.translation = lerp.to;
            if let Projection::Orthographic(ortho) = &mut *projection {
                ortho.scale = lerp.to_scale;
            }
            commands.entity(entity).remove::<CameraLerp>();
        }
    }
}

/// Eases the camera back to the home framing (`BOARD_CAM_POS` at
/// 1:1) after a level's fit zoom, on the ways out of a run (Results,
/// or straight back to the menu). A no-op when the camera is already
/// home, so arrivals that never entered a level keep today's exact
/// behavior — no drift on the first menu entry or picker round-trips.
fn restore_home_framing(
    mut commands: Commands,
    cameras: Query<(Entity, &Transform, &Projection), With<Camera>>,
) {
    if let Ok((entity, transform, projection)) = cameras.single() {
        if let Projection::Orthographic(ortho) = projection {
            if (ortho.scale - 1.0).abs() > 0.001 {
                commands.entity(entity).insert(CameraLerp {
                    from: transform.translation,
                    to: BOARD_CAM_POS,
                    from_scale: ortho.scale,
                    to_scale: 1.0,
                    elapsed_secs: 0.0,
                });
            }
        }
    }
}

/// Starts (or retargets) the camera glide to `framing` from the
/// camera's current pose. Shared by level entry (`setup_level`) and
/// mid-level window resizes (`refit_framing_on_resize`) so both
/// ease identically.
fn glide_camera_to(
    commands: &mut Commands,
    entity: Entity,
    transform: &Transform,
    projection: &Projection,
    framing: board::BoardFraming,
) {
    let from_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };
    commands.entity(entity).insert(CameraLerp {
        from: transform.translation,
        to: Vec3::new(framing.center.x, framing.center.y, 0.0),
        from_scale,
        to_scale: framing.scale,
        elapsed_secs: 0.0,
    });
}

/// Re-frames the board when the window is resized mid-level (or the
/// phone rotates): `setup_level` framed for the window at entry,
/// and without this the board would keep that framing — nodes
/// sliding off a shrunken window, or a board stranded small in a
/// grown one. Only the latest event of a drag-resize stream
/// matters; the recompute is skipped when the level's fit framing
/// for the new size is the framing already in effect.
fn refit_framing_on_resize(
    mut commands: Commands,
    // `Option`: headless harnesses build `PlayingPlugin` without
    // the window plugin that initializes this message; there are
    // no resizes to react to there, so the system stands down
    // instead of panicking on an uninitialized message.
    mut resized: Option<MessageReader<WindowResized>>,
    windows: Query<Entity, With<PrimaryWindow>>,
    cameras: Query<(Entity, &Transform, &Projection), With<Camera>>,
    index: Res<CurrentLevelIndex>,
) {
    let Some(resized) = resized.as_mut() else {
        return;
    };
    let Some(event) = resized.read().last() else {
        return;
    };
    if windows.single().ok() != Some(event.window) {
        return;
    }
    let Ok((entity, transform, projection)) = cameras.single() else {
        return;
    };
    let level_def = level::load_level(index.0);
    let framing = board::board_framing(&level_def, Vec2::new(event.width, event.height));
    let current_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };
    let target = Vec3::new(framing.center.x, framing.center.y, 0.0);
    if (framing.scale - current_scale).abs() <= 0.001
        && transform.translation.distance_squared(target) <= 1.0
    {
        return;
    }
    glide_camera_to(&mut commands, entity, transform, projection, framing);
}

#[allow(clippy::too_many_arguments)]
fn setup_level(
    mut commands: Commands,
    index: Res<CurrentLevelIndex>,
    mut live: ResMut<LiveGraph>,
    mut placed: ResMut<board::PlacedChoices>,
    mut clock: ResMut<LevelClock>,
    mut active_outage: ResMut<ActiveOutage>,
    asset_server: Res<AssetServer>,
    mut companions: Query<(Entity, &mut crate::waifu::CompanionSprite)>,
    cameras: Query<(Entity, &Transform, &Projection), With<Camera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let level_def = level::load_level(index.0);

    // Fit framing (playtest round 2): the board is laid out in fixed
    // world coordinates and most levels are wider than the window, so
    // the historical fixed framing bled nodes and labels off the
    // edges. `board::board_framing` centers the camera on the level
    // and zooms out (never in) until the whole board fits this window.
    // Falls back to the 720x1280 design resolution when no window
    // exists (unit tests).
    let (win_w, win_h) = windows
        .single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((720.0, 1280.0));
    let framing = board::board_framing(&level_def, Vec2::new(win_w, win_h));
    let cam_target = Vec3::new(framing.center.x, framing.center.y, 0.0);

    // Persistent camera (Finding 3): the MainMenu camera is a plain
    // Camera2dBundle, identical to what was spawned here before, so the
    // Android framing fix (verified 2026-09-30) is preserved — only the
    // despawn/respawn cut is gone, replaced by a 0.5s ease to the board
    // framing (now the level's fit framing: center + zoom).
    match cameras.single() {
        Ok((entity, transform, projection)) => {
            glide_camera_to(&mut commands, entity, transform, projection, framing);
        }
        Err(_) => {
            // Defensive: no camera exists (the menu always spawns one, so
            // this shouldn't happen). Spawn directly at the fit framing.
            let mut projection = OrthographicProjection::default_2d();
            projection.scale = framing.scale;
            commands.spawn((
                Camera2d,
                Projection::Orthographic(projection),
                Transform::from_translation(cam_target),
            ));
        }
    }

    placed.0.clear();
    clock.elapsed_seconds = 0.0;
    active_outage.outage = None;
    for (entity, mut sprite) in &mut companions {
        sprite.mood = crate::waifu::Mood::Idle;
        sprite.frame = 0;
        trigger_mood_pop(&mut commands, entity);
    }
    board::rebuild_live_graph(
        &level_def,
        &placed,
        active_outage.outage.as_ref(),
        &mut live.graph,
    );
    live.wavelength = WavelengthWrapper(level_def.wavelength.into());
    live.tx_dbm = level_def.tx_dbm;

    // Half the visible width in world units — the window width
    // widened by the fit zoom — plus the view's center: together they
    // keep node-label plates inside the framed view.
    let half_w = win_w * 0.5 * framing.scale;
    board::spawn_board_from_level(
        &mut commands,
        &level_def,
        &asset_server,
        half_w,
        framing.center.x,
    );
    // Signals the on-device instrumentation tests (android/app/src/androidTest)
    // that the board has finished spawning and is ready to receive touch
    // input — they poll Logcat for this line before injecting gestures.
    // See `test_log!` in lib.rs and docs/BUILD.md.
    test_log!("level_ready id={} index={}", level_def.id, index.0);

    commands.insert_resource(level_def);
}

fn tick_clock(time: Res<Time>, mut clock: ResMut<LevelClock>) {
    clock.elapsed_seconds += time.delta_secs_f64();
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
    mut alarm_list: ResMut<AlarmList>,
    mut request: ResMut<TransitionRequest>,
    mut reaction_inbox: Option<ResMut<crate::waifu::reactions::ReactionInbox>>,
) {
    let Some(scripted) = &level.scripted_outage else {
        return;
    };
    if active_outage.outage.is_some() {
        return;
    }
    if clock.elapsed_seconds >= scripted.fires_after_seconds {
        let outage = Outage::new(scripted.kind.into(), scripted.edge_from, scripted.edge_to);
        active_outage.outage = Some(outage.clone());
        // Mirror into the NOC alarm list for the multi-alarm console.
        alarm_list.raise(outage, clock.elapsed_seconds);
        board::rebuild_live_graph(
            &level,
            &placed,
            active_outage.outage.as_ref(),
            &mut live.graph,
        );
        if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
            inbox.push(crate::waifu::reactions::ReactionTrigger::OutageStart);
        }
        request.0 = Some(GameState::OutageActive);
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
#[allow(clippy::too_many_arguments)]
fn check_win_condition(
    mut commands: Commands,
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    mut outcome: ResMut<LevelOutcome>,
    mut request: ResMut<TransitionRequest>,
    next_state: Res<NextState<GameState>>,
    mut companions: Query<(Entity, &mut crate::waifu::CompanionSprite)>,
    sfx: Res<crate::audio::Sfx>,
    api_progress: Res<super::api_console::ApiProgress>,
    triage_progress: Res<super::triage_console::TriageProgress>,
    astra: crate::astra::AstraProgress,
    board_roots: Query<Entity, With<crate::board::BoardRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut glow_materials: Option<ResMut<Assets<crate::shaders::GlowMaterial>>>,
    gate: Option<ResMut<crate::anim::ResultsGate>>,
    mut reaction_inbox: Option<ResMut<crate::waifu::reactions::ReactionInbox>>,
) {
    if level.scripted_outage.is_some() {
        return;
    }
    // Request the Results transition exactly once. This system keeps
    // running until the pending transition actually applies, and
    // re-requesting would queue a duplicate Results -> Results transition:
    // its OnExit/OnEnter pair tears down and rebuilds the results screen,
    // despawning the buttons out from under input (and replaying the win
    // SFX every frame).
    let gate_holding = gate.as_ref().is_some_and(|g| g.is_holding());
    let transition_pending =
        request.0.is_some() || matches!(*next_state, NextState::Pending(_)) || gate_holding;
    // API-driver levels (Clara's NBI/SMx puzzles) additionally require
    // the expected call sequence through the console. The board check
    // still applies underneath — both gates must pass.
    let api_ok = match &level.api_sequence {
        Some(seq) => api_progress.is_complete(&seq.expected),
        None => true,
    };
    // Triage levels (Aino's NOC board) additionally require the alarms
    // acked in the expected priority order. All three gates must pass.
    let triage_ok = match &level.alarm_triage {
        Some(triage) => triage_progress.is_complete(&triage.expected_order),
        None => true,
    };
    // Astra mechanic gates (§2b–§2f): every present optional block
    // must pass beside the board, via the one conjunction in
    // `crate::astra` (shared with the outage resolution path).
    let astra_ok = crate::astra::astra_gates_pass(&level, &astra);
    if !transition_pending
        && api_ok
        && triage_ok
        && astra_ok
        && level.is_win_state(&live.graph, live.tx_dbm, live.wavelength.0)
    {
        sfx.play(&mut commands, crate::audio::SfxKind::Win);
        outcome.won = true;
        // Celebratory glow burst at the board center (BoardRoot child
        // so teardown sweeps it). Visible during the wipe-out.
        if let (Ok(board_root), Some(mut glow_mats)) =
            (board_roots.single(), glow_materials.as_mut())
        {
            let center = level
                .nodes
                .iter()
                .filter_map(|n| crate::board::node_world_pos(&level, n.id))
                .fold(Vec2::ZERO, |a, b| a + b)
                / level.nodes.len().max(1) as f32;
            let glow = crate::shaders::spawn_glow(
                &mut commands,
                &mut meshes,
                &mut glow_mats,
                center.extend(1.0),
                LinearRgba::new(1.0, 0.9, 0.6, 1.0),
                400.0,
            );
            commands.entity(board_root).add_child(glow);
        }
        // Guarded: this system keeps running during the 0.3s fade-out,
        // and re-popping every frame would restart the animation forever.
        for (entity, mut sprite) in &mut companions {
            if sprite.mood != crate::waifu::Mood::Celebrate {
                sprite.mood = crate::waifu::Mood::Celebrate;
                sprite.frame = 0;
                trigger_mood_pop(&mut commands, entity);
            }
        }
        // Completion reaction first, results after the hold: fire the
        // reaction in-level and let the gate driver issue the
        // transition once ~1.8 s of reaction time has played. Without
        // the gate (headless harnesses), keep the immediate request.
        if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
            inbox.push(crate::waifu::reactions::ReactionTrigger::LevelComplete);
        }
        match gate {
            Some(mut gate) if !gate.is_holding() => gate.begin(),
            Some(_) => {}
            None => request.0 = Some(GameState::Results),
        }
    }
}
