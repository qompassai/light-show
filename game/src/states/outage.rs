//! Outage-active state: a fault has fired on the current level's graph.
//! Shows the alarm banner, alarms the on-screen companion, ticks the
//! repair timer against the live link budget, and resolves straight to
//! `Results` on either a successful repair or a timeout.
//!
//! Deliberately never transitions back to `Playing`: routing back would
//! run `playing::setup_level` again, which always does a full level
//! reset (see its doc comment) and would wipe the player's in-progress
//! repair. Resolving only into `Results` avoids that class of bug
//! entirely — see `check_outage_resolution` below.

use super::playing::LiveGraph;
use super::{GameState, LevelOutcome};
use crate::anim::TransitionRequest;
use crate::fonts::FONT_SIZE_ADJUST;
use crate::level::LevelDef;
use crate::waifu::{trigger_mood_pop, CompanionSprite, Mood};
use crate::warehouse::Loadout;
use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use osp_sim::{Alarm, AlarmAck, Outage};

pub struct OutagePlugin;

impl Plugin for OutagePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveOutage::default())
            .init_resource::<AlarmList>()
            .add_systems(
                OnEnter(GameState::OutageActive),
                (
                    announce_outage,
                    spawn_outage_banner,
                    alarm_companion,
                    spawn_alarm_list_panel,
                )
                    .after(apply_loadout_timer),
            )
            .add_systems(OnEnter(GameState::OutageActive), apply_loadout_timer)
            .add_systems(
                Update,
                (
                    tick_outage,
                    update_outage_banner,
                    update_alarm_list_panel,
                    handle_alarm_ack_input,
                    check_outage_resolution,
                )
                    .chain()
                    .run_if(in_state(GameState::OutageActive)),
            )
            .add_systems(
                OnExit(GameState::OutageActive),
                (teardown_outage_banner, teardown_alarm_list_panel),
            )
            .add_systems(Update, (animate_banner_entrance, animate_banner_exit));
    }
}

/// The single in-progress outage, if any. `None` outside of
/// `GameState::OutageActive` and immediately after a level (re)start;
/// populated by `playing::check_scripted_outage` the instant a level's
/// scripted hazard fires.
#[derive(Resource, Default)]
pub struct ActiveOutage {
    pub outage: Option<Outage>,
    /// Seconds the Warehouse loadout adds to this outage's repair timer,
    /// set by `apply_loadout_timer` on every outage start. Kept here, not
    /// in `elapsed_seconds`, so degrading hazards still worsen on the
    /// real clock: gear buys time, it doesn't slow the water.
    pub timer_bonus_secs: f64,
}

impl ActiveOutage {
    /// Seconds left on the repair timer, gear bonus included; 0 with no
    /// outage.
    pub fn time_remaining(&self) -> f64 {
        self.outage.as_ref().map_or(0.0, |o| {
            (o.kind.base_timer_seconds() + self.timer_bonus_secs - o.elapsed_seconds).max(0.0)
        })
    }

    /// True when an unresolved outage has run out of (bonus-extended)
    /// time. Mirrors `Outage::is_expired`.
    pub fn is_expired(&self) -> bool {
        self.outage
            .as_ref()
            .is_some_and(|o| !o.resolved && self.time_remaining() <= 0.0)
    }
}

/// All active NOC alarms. Each outage that fires creates an `Alarm` here;
/// the list supports multiple concurrent alarms with ack/dispatch/clear.
/// The `ActiveOutage` single-outage banner stays for backward compatibility
/// with the existing outage-repair loop.
///
/// Alarm identity is a monotonic `u64` id (see `AlarmList::raise`), never
/// a `Vec` index: rows are re-sorted for display and cleared alarms are
/// removed, so indices shift but ids stay stable for the session.
#[derive(Resource, Default)]
pub struct AlarmList {
    pub alarms: Vec<Alarm>,
    next_id: u64,
}

impl AlarmList {
    /// Severity rank for ordering: Critical first. Higher is more urgent.
    fn severity_rank(severity: osp_sim::AlarmSeverity) -> u8 {
        match severity {
            osp_sim::AlarmSeverity::Critical => 3,
            osp_sim::AlarmSeverity::Major => 2,
            osp_sim::AlarmSeverity::Minor => 1,
            osp_sim::AlarmSeverity::Warning => 0,
        }
    }

    /// Raise a new alarm from an outage. Assigns the next monotonic id and
    /// returns it. Ids are never reused within a session, even after
    /// cleared alarms are removed.
    pub fn raise(&mut self, outage: Outage, now_secs: f64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.alarms.push(Alarm::new(id, outage, now_secs));
        id
    }

    /// Find an alarm by id.
    /// NOC console API (see `states::noc`); not yet wired to a screen.
    #[allow(dead_code)]
    pub fn get(&self, id: u64) -> Option<&Alarm> {
        self.alarms.iter().find(|a| a.id == id)
    }

    /// Acknowledge an alarm by id, dispatching a companion. Records the ack
    /// timestamp for response-time scoring. Returns false for unknown ids;
    /// the underlying `Alarm::acknowledge` is idempotent for the rest.
    pub fn ack(&mut self, id: u64, companion_idx: u8, now_secs: f64) -> bool {
        match self.alarms.iter_mut().find(|a| a.id == id) {
            Some(alarm) => {
                alarm.acknowledge(companion_idx, now_secs);
                true
            }
            None => false,
        }
    }

    /// Remove a resolved alarm by id (archive to history). Only `Resolved`
    /// alarms can be cleared -- anything else is left in place. Returns
    /// true when an alarm was removed.
    /// NOC console API (see `states::noc`); not yet wired to a screen.
    #[allow(dead_code)]
    pub fn clear(&mut self, id: u64) -> bool {
        let before = self.alarms.len();
        self.alarms
            .retain(|a| !(a.id == id && matches!(a.ack, AlarmAck::Resolved)));
        self.alarms.len() < before
    }

    /// Remove every cleared alarm from the active list (history archival).
    pub fn clear_resolved(&mut self) {
        self.alarms.retain(|a| a.ack != AlarmAck::Cleared);
    }

    /// Count of unacknowledged alarms.
    pub fn unacked_count(&self) -> usize {
        self.alarms
            .iter()
            .filter(|a| matches!(a.ack, AlarmAck::New))
            .count()
    }

    /// Display order: unacked first, then severity descending, then age
    /// descending (oldest first -- the one that has been waiting longest).
    pub fn sorted(&self) -> Vec<&Alarm> {
        let mut refs: Vec<&Alarm> = self.alarms.iter().collect();
        refs.sort_by(|a, b| {
            let unacked_a = matches!(a.ack, AlarmAck::New);
            let unacked_b = matches!(b.ack, AlarmAck::New);
            unacked_b
                .cmp(&unacked_a)
                .then(Self::severity_rank(b.severity).cmp(&Self::severity_rank(a.severity)))
                .then(a.raised_at_secs.total_cmp(&b.raised_at_secs))
        });
        refs
    }

    /// The alarm the NOC banner shows: highest-severity unacked alarm,
    /// oldest first on ties. `None` when everything is acked -- the banner
    /// shows ALL CLEAR.
    /// NOC console API (see `states::noc`); not yet wired to a screen.
    #[allow(dead_code)]
    pub fn highest_unacked(&self) -> Option<&Alarm> {
        self.sorted()
            .into_iter()
            .find(|a| matches!(a.ack, AlarmAck::New))
    }
}

/// Root node of the on-screen alarm banner, despawned `OnExit(OutageActive)`.
#[derive(Component)]
struct OutageBanner;

/// The countdown text child, refreshed every frame by `update_outage_banner`.
#[derive(Component)]
pub(crate) struct OutageBannerText;

/// Seconds for the banner slide-down entrance (Finding 4).
const BANNER_SLIDE_IN_SECS: f32 = 0.35;
/// Seconds for the banner slide-up exit (Finding 4).
const BANNER_SLIDE_OUT_SECS: f32 = 0.25;
/// Banner resting alpha once fully entered.
const BANNER_ALPHA: f32 = 0.85;
/// Banner start/end offset for the slide (px above the viewport top).
const BANNER_HIDDEN_TOP_PX: f32 = -80.0;

/// Marker for the banner entrance animation: slides the banner down
/// from `top: -80px` to `top: 0` with a BackOut overshoot while fading
/// its alpha 0 -> 0.85 (Finding 4).
#[derive(Component)]
struct BannerSlide {
    elapsed_secs: f32,
}

/// Marker for the banner exit animation: reverses the entrance with a
/// 0.25s slide-up + fade-out instead of an instant despawn (Finding 4).
#[derive(Component)]
struct BannerSlideOut {
    elapsed_secs: f32,
}

/// Applies Warehouse gear to the outage that just fired: recomputes the
/// `Loadout` from the save, extends the repair timer by the spare
/// battery's multiplier and the field coffee's bonus, and uses up one
/// coffee when it contributed. Without a save (headless tests) the timer
/// is the plain base timer.
fn apply_loadout_timer(
    mut active: ResMut<ActiveOutage>,
    save: Option<ResMut<crate::save::SaveData>>,
    loadout: Option<ResMut<Loadout>>,
    save_writer: Option<MessageWriter<crate::save::SaveRequest>>,
) {
    active.timer_bonus_secs = 0.0;
    let (Some(base_secs), Some(mut save)) = (
        active.outage.as_ref().map(|o| o.kind.base_timer_seconds()),
        save,
    ) else {
        return;
    };
    let gear = Loadout::from_save(&save);
    active.timer_bonus_secs = gear.outage_extra_secs(base_secs);
    if gear.outage_timer_bonus_secs > 0.0
        && crate::warehouse::consume_one(&mut save.consumables, crate::warehouse::FIELD_COFFEE)
    {
        if let Some(mut writer) = save_writer {
            writer.write(crate::save::SaveRequest);
        }
    }
    if let Some(mut loadout) = loadout {
        *loadout = Loadout::from_save(&save);
    }
}

fn announce_outage(mut commands: Commands, active: Res<ActiveOutage>, sfx: Res<crate::audio::Sfx>) {
    sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
    if let Some(outage) = &active.outage {
        info!(
            "OUTAGE: {} (timer: {:.0}s)",
            outage.kind.flavor_text(),
            active.time_remaining()
        );
    }
}

/// Swaps every spawned companion into the `Alarmed` mood and resets its
/// animation frame, so the sprite flip reads as an immediate reaction to
/// the storm/fault rather than mid-idle-loop. There's only ever one
/// companion spawned at a time (see `waifu::respawn_on_companion_change`),
/// but this iterates the query rather than assuming that invariant.
fn alarm_companion(mut commands: Commands, mut query: Query<(Entity, &mut CompanionSprite)>) {
    for (entity, mut sprite) in &mut query {
        sprite.mood = Mood::Alarmed;
        sprite.frame = 0;
        trigger_mood_pop(&mut commands, entity);
    }
}

/// Spawns a full-width alarm banner: hazard flavor text on top, a live
/// "REPAIR NOW — Ns" countdown underneath. Colors/font follow the hazard
/// palette in `docs/ART_STYLE.md`.
fn spawn_outage_banner(
    mut commands: Commands,
    active: Res<ActiveOutage>,
    asset_server: Res<AssetServer>,
) {
    let Some(outage) = &active.outage else {
        return;
    };
    commands
        .spawn((
            OutageBanner,
            // The banner starts above the viewport and fully transparent;
            // `animate_banner_entrance` slides/fades it in (Finding 4).
            BannerSlide { elapsed_secs: 0.0 },
            Node {
                width: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                top: Val::Px(BANNER_HIDDEN_TOP_PX),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::vertical(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.65, 0.0, 0.05, 0.0)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(outage.kind.flavor_text()),
                TextFont {
                    font: asset_server.load(crate::fonts::BODY).into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                OutageBannerText,
                Text::new(format!("REPAIR NOW — {:.0}s", active.time_remaining())),
                TextFont {
                    font: asset_server.load(crate::fonts::DISPLAY_BOLD).into(),
                    font_size: FontSize::Px(20.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.82, 0.4)), // #ffd166
            ));
        });
}

fn update_outage_banner(
    active: Res<ActiveOutage>,
    mut query: Query<&mut Text, With<OutageBannerText>>,
) {
    if active.outage.is_none() {
        return;
    }
    for mut text in &mut query {
        text.0 = format!("REPAIR NOW — {:.0}s", active.time_remaining());
    }
}

fn teardown_outage_banner(
    mut commands: Commands,
    query: Query<Entity, (With<OutageBanner>, Without<BannerSlideOut>)>,
) {
    // Don't despawn instantly: hand the banner to `animate_banner_exit`
    // for the 0.25s slide-up + fade-out (Finding 4). The fullscreen fade
    // covers the state swap anyway, so this plays out underneath it.
    for entity in &query {
        commands
            .entity(entity)
            .insert(BannerSlideOut { elapsed_secs: 0.0 });
    }
}

/// Slides the banner down from `top: -80px` to `top: 0` over 0.35s with
/// a BackOut overshoot (the "alarm" feel) while fading alpha 0 -> 0.85.
fn animate_banner_entrance(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Node, &mut BackgroundColor, &mut BannerSlide)>,
) {
    for (entity, mut node, mut background, mut slide) in &mut query {
        slide.elapsed_secs += time.delta_secs();
        let t = (slide.elapsed_secs / BANNER_SLIDE_IN_SECS).clamp(0.0, 1.0);
        let eased = EaseFunction::BackOut.sample_clamped(t);
        node.top = Val::Px(BANNER_HIDDEN_TOP_PX + -BANNER_HIDDEN_TOP_PX * eased);
        background.0 = Color::srgba(0.65, 0.0, 0.05, BANNER_ALPHA * eased.clamp(0.0, 1.0));
        if t >= 1.0 {
            node.top = Val::Px(0.0);
            background.0 = Color::srgba(0.65, 0.0, 0.05, BANNER_ALPHA);
            commands.entity(entity).remove::<BannerSlide>();
        }
    }
}

/// Reverses the entrance: 0.25s slide-up + fade-out, then despawns.
fn animate_banner_exit(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Node, &mut BackgroundColor, &mut BannerSlideOut)>,
) {
    for (entity, mut node, mut background, mut slide_out) in &mut query {
        slide_out.elapsed_secs += time.delta_secs();
        let t = (slide_out.elapsed_secs / BANNER_SLIDE_OUT_SECS).clamp(0.0, 1.0);
        let eased = EaseFunction::CubicInOut.sample_clamped(t);
        node.top = Val::Px(BANNER_HIDDEN_TOP_PX * eased);
        background.0 = Color::srgba(0.65, 0.0, 0.05, BANNER_ALPHA * (1.0 - eased));
        if t >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Final-seconds countdown window for the repair tick: one tick per
/// whole second, so the player hears the clock running out.
const TICK_WINDOW_SECONDS: f64 = 5.0;

fn tick_outage(
    mut commands: Commands,
    time: Res<Time>,
    mut active: ResMut<ActiveOutage>,
    sfx: Res<crate::audio::Sfx>,
    mut last_whole_second: Local<u32>,
) {
    let Some(outage) = &mut active.outage else {
        return;
    };
    outage.tick(time.delta_secs_f64());
    let remaining = active.time_remaining();
    if remaining > TICK_WINDOW_SECONDS {
        // Outside the window: re-arm so re-entering it always ticks.
        *last_whole_second = u32::MAX;
        return;
    }
    let whole = remaining.ceil() as u32;
    if whole < *last_whole_second {
        *last_whole_second = whole;
        sfx.play(&mut commands, crate::audio::SfxKind::Tick);
    }
}

/// Resolves the active outage against the live link budget: a timeout
/// ends the level as a loss, and a repaired budget that lands back in
/// window marks the outage resolved and ends the level as a win. Both
/// outcomes go straight to `Results` — see the module doc comment for
/// why this never routes back to `Playing`.
#[allow(clippy::too_many_arguments)]
fn check_outage_resolution(
    mut commands: Commands,
    mut active: ResMut<ActiveOutage>,
    mut alarm_list: ResMut<AlarmList>,
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    mut outcome: ResMut<LevelOutcome>,
    mut request: ResMut<TransitionRequest>,
    next_state: Res<NextState<GameState>>,
    mut companions: Query<(Entity, &mut CompanionSprite)>,
    sfx: Res<crate::audio::Sfx>,
    api_progress: Res<crate::states::api_console::ApiProgress>,
    triage_progress: Res<crate::states::triage_console::TriageProgress>,
    mut gate: Option<ResMut<crate::anim::ResultsGate>>,
    mut reaction_inbox: Option<ResMut<crate::waifu::reactions::ReactionInbox>>,
) {
    let Some(outage) = active.outage.clone() else {
        return;
    };

    // Request the Results transition exactly once (see
    // `check_win_condition`): re-requesting queues a duplicate
    // Results -> Results transition whose OnExit/OnEnter pair rebuilds
    // the results screen mid-frame.
    let gate_holding = gate.as_ref().is_some_and(|g| g.is_holding());
    let transition_pending =
        request.0.is_some() || matches!(*next_state, NextState::Pending(_)) || gate_holding;

    if !transition_pending && active.is_expired() {
        sfx.play(&mut commands, crate::audio::SfxKind::Lose);
        outcome.won = false;
        if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
            inbox.push(crate::waifu::reactions::ReactionTrigger::LevelFailed);
        }
        match gate.as_deref_mut() {
            Some(g) if !g.is_holding() => g.begin(),
            Some(_) => {}
            None => request.0 = Some(GameState::Results),
        }
        return;
    }

    // API-driver levels additionally require the console sequence —
    // same gate as the plain win check (see playing::check_win_condition).
    let api_ok = match &level.api_sequence {
        Some(seq) => api_progress.is_complete(&seq.expected),
        None => true,
    };
    // Triage levels (Aino's NOC board) additionally require the alarms
    // acked in the expected priority order.
    let triage_ok = match &level.alarm_triage {
        Some(triage) => triage_progress.is_complete(&triage.expected_order),
        None => true,
    };
    if !transition_pending
        && api_ok
        && triage_ok
        && level.is_win_state_with_outage(
            &live.graph,
            live.tx_dbm,
            live.wavelength.0,
            Some(&outage),
        )
    {
        if let Some(active_outage) = active.outage.as_mut() {
            active_outage.resolved = true;
        }
        // Mark the corresponding NOC alarm resolved (most recent New/Acked).
        for alarm in alarm_list.alarms.iter_mut().rev() {
            if matches!(alarm.ack, AlarmAck::New | AlarmAck::Acknowledged { .. }) {
                alarm.ack = AlarmAck::Resolved;
                break;
            }
        }
        sfx.play(&mut commands, crate::audio::SfxKind::Win);
        outcome.won = true;
        // A "nice save" reaction distinct from a plain, drama-free clear
        // (see `states::playing::check_win_condition`'s `Mood::Celebrate`).
        // Guarded: this system keeps running during the 0.3s fade-out, and
        // re-popping every frame would restart the animation forever.
        for (entity, mut sprite) in &mut companions {
            if sprite.mood != Mood::Wink {
                sprite.mood = Mood::Wink;
                sprite.frame = 0;
                trigger_mood_pop(&mut commands, entity);
            }
        }
        if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
            inbox.push(crate::waifu::reactions::ReactionTrigger::LevelComplete);
        }
        match gate.as_deref_mut() {
            Some(g) if !g.is_holding() => g.begin(),
            Some(_) => {}
            None => request.0 = Some(GameState::Results),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{ComponentChoice, LevelNode, MediumDef, WavelengthDef};
    use crate::states::playing::WavelengthWrapper;
    use bevy_ecs::system::RunSystemOnce;
    use osp_sim::component::PlantType;
    use osp_sim::{Component, OutageKind, PathGraph, SpliceType};

    // tx_dbm is deliberately well inside the receive window (rather than
    // the more realistic GPON +3 dBm used elsewhere) so a ~1 km span's
    // tiny attenuation can't push the received power out of
    // [window_min_dbm, window_max_dbm] and make these resolution tests
    // flaky-by-construction.
    fn fixture_level(scripted_outage_edge: (u32, u32)) -> LevelDef {
        LevelDef {
            id: "test".into(),
            title: "Test".into(),
            world: 0,
            briefing: String::new(),
            medium: MediumDef::Fiber,
            tx_dbm: -15.0,
            wavelength: WavelengthDef::Nm1490,
            window_min_dbm: -27.0,
            window_max_dbm: -8.0,
            endpoint_poe_draw_w: None,
            required_bandwidth_mbps: None,
            max_segment_length_m: None,
            coax_noise_floor_dbmv: None,
            min_carrier_to_noise_db: None,
            min_snr_db: None,
            nodes: vec![
                LevelNode {
                    id: 0,
                    label: "A".into(),
                    grid_x: 0.0,
                    grid_y: 0.0,
                },
                LevelNode {
                    id: 1,
                    label: "B".into(),
                    grid_x: 1.0,
                    grid_y: 0.0,
                },
            ],
            fixed_edges: vec![],
            available_components: vec![ComponentChoice {
                from: scripted_outage_edge.0,
                to: scripted_outage_edge.1,
                component: Component::Splice {
                    kind: SpliceType::Fusion,
                    degradation_db: 0.0,
                },
            }],
            source_node: 0,
            target_node: 1,
            scripted_outage: None,
            subscribers: vec![],
            on_enter_line: None,
            on_win_line: None,
            on_fail_line: None,
            api_sequence: None,
            alarm_triage: None,
            quiz: None,
            splice_work_orders: None,
        }
    }

    fn connected_live_graph() -> LiveGraph {
        let mut graph = PathGraph::default();
        graph.add_node(0, "A");
        graph.add_node(1, "B");
        graph.connect(
            0,
            1,
            Component::Span {
                length_km: 1.0,
                plant: PlantType::Buried,
            },
        );
        LiveGraph {
            graph,
            wavelength: WavelengthWrapper(osp_sim::Wavelength::Nm1490),
            tx_dbm: -15.0,
        }
    }

    #[test]
    fn expired_outage_resolves_to_results_as_a_loss() {
        let mut world = World::new();
        let mut outage = Outage::new(OutageKind::ConnectorContamination, 0, 1);
        outage.tick(9999.0);
        world.insert_resource(ActiveOutage {
            outage: Some(outage),
            ..default()
        });
        world.insert_resource(connected_live_graph());
        world.insert_resource(fixture_level((0, 1)));
        world.insert_resource(LevelOutcome { won: true });
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world.init_resource::<AlarmList>();
        world.init_resource::<crate::states::api_console::ApiProgress>();
        world.init_resource::<crate::states::triage_console::TriageProgress>();

        world.run_system_once(check_outage_resolution);

        assert!(!world.resource::<LevelOutcome>().won);
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Results)
        );
    }

    #[test]
    fn in_window_budget_resolves_the_outage_as_a_win() {
        let mut world = World::new();
        let outage = Outage::new(OutageKind::WaterIntrusion, 0, 1);
        world.insert_resource(ActiveOutage {
            outage: Some(outage),
            ..default()
        });
        world.insert_resource(connected_live_graph());
        world.insert_resource(fixture_level((0, 1)));
        world.insert_resource(LevelOutcome { won: false });
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world.init_resource::<AlarmList>();
        world.init_resource::<crate::states::api_console::ApiProgress>();
        world.init_resource::<crate::states::triage_console::TriageProgress>();

        world.run_system_once(check_outage_resolution);

        assert!(world.resource::<LevelOutcome>().won);
        assert!(
            world
                .resource::<ActiveOutage>()
                .outage
                .as_ref()
                .unwrap()
                .resolved
        );
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Results)
        );
    }

    #[test]
    fn no_active_outage_is_a_no_op() {
        let mut world = World::new();
        world.insert_resource(ActiveOutage::default());
        world.insert_resource(connected_live_graph());
        world.insert_resource(fixture_level((0, 1)));
        world.insert_resource(LevelOutcome { won: true });
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.insert_resource(crate::audio::Sfx::for_tests());

        world.run_system_once(check_outage_resolution);

        assert_eq!(world.resource::<TransitionRequest>().0, None);
    }

    // ---- AlarmList: id-based NOC alarm management ----

    fn raise_kind(list: &mut AlarmList, kind: osp_sim::OutageKind, at: f64) -> u64 {
        list.raise(Outage::new(kind, 0, 1), at)
    }

    #[test]
    fn raise_assigns_monotonic_ids_never_reused() {
        let mut list = AlarmList::default();
        let a = raise_kind(&mut list, osp_sim::OutageKind::FiberCut, 0.0);
        let b = raise_kind(&mut list, osp_sim::OutageKind::WaterIntrusion, 10.0);
        assert_eq!((a, b), (0, 1));
        // Resolve and clear the first; the next id still advances.
        list.alarms[0].outage.resolved = true;
        list.alarms[0].sync_from_outage();
        assert!(list.clear(a));
        let c = raise_kind(&mut list, osp_sim::OutageKind::Macrobend, 20.0);
        assert_eq!(c, 2);
        assert!(list.get(a).is_none());
    }

    #[test]
    fn ack_by_id_records_companion_and_timestamp() {
        let mut list = AlarmList::default();
        let id = raise_kind(&mut list, osp_sim::OutageKind::WaterIntrusion, 50.0);
        assert!(list.ack(id, 2, 63.5));
        let alarm = list.get(id).unwrap();
        assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 2 });
        assert_eq!(alarm.acked_at_secs, Some(63.5));
        // Unknown id: no-op, returns false.
        assert!(!list.ack(999, 0, 0.0));
    }

    #[test]
    fn clear_only_removes_resolved_alarms() {
        let mut list = AlarmList::default();
        let id = raise_kind(&mut list, osp_sim::OutageKind::FiberCut, 0.0);
        // Not resolved yet: clear refuses.
        assert!(!list.clear(id));
        assert!(list.get(id).is_some());
        // Resolve, then clear works.
        list.alarms[0].outage.resolved = true;
        list.alarms[0].sync_from_outage();
        assert!(list.clear(id));
        assert!(list.get(id).is_none());
    }

    #[test]
    fn highest_unacked_prefers_severity_then_age() {
        let mut list = AlarmList::default();
        // Warning raised first (oldest), then a Critical.
        let warn = raise_kind(&mut list, osp_sim::OutageKind::WaterIntrusion, 0.0);
        let crit = raise_kind(&mut list, osp_sim::OutageKind::FiberCut, 30.0);
        assert_eq!(list.highest_unacked().unwrap().id, crit);
        // Ack the critical: the older warning surfaces.
        list.ack(crit, 0, 40.0);
        assert_eq!(list.highest_unacked().unwrap().id, warn);
        // Ack everything: banner shows ALL CLEAR.
        list.ack(warn, 0, 50.0);
        assert!(list.highest_unacked().is_none());
    }

    #[test]
    fn sorted_puts_unacked_first() {
        let mut list = AlarmList::default();
        let a = raise_kind(&mut list, osp_sim::OutageKind::FiberCut, 0.0);
        let b = raise_kind(&mut list, osp_sim::OutageKind::WaterIntrusion, 5.0);
        list.ack(a, 0, 10.0);
        let ordered: Vec<u64> = list.sorted().iter().map(|x| x.id).collect();
        // b is still New so it sorts first despite lower severity.
        assert_eq!(ordered, vec![b, a]);
    }

    #[test]
    fn unacked_count_ignores_acknowledged() {
        let mut list = AlarmList::default();
        let a = raise_kind(&mut list, osp_sim::OutageKind::FiberCut, 0.0);
        raise_kind(&mut list, osp_sim::OutageKind::WaterIntrusion, 5.0);
        assert_eq!(list.unacked_count(), 2);
        list.ack(a, 0, 10.0);
        assert_eq!(list.unacked_count(), 1);
    }

    #[test]
    fn alarm_companion_sets_alarmed_mood_and_resets_frame() {
        let mut world = World::new();
        world.spawn(CompanionSprite {
            companion: crate::waifu::Companion::Fiber,
            mood: Mood::Idle,
            anim_timer: Timer::from_seconds(0.18, TimerMode::Repeating),
            frame: 3,
        });

        world.run_system_once(alarm_companion);

        let sprite = world.query::<&CompanionSprite>().single(&world).unwrap();
        assert_eq!(sprite.mood, Mood::Alarmed);
        assert_eq!(sprite.frame, 0);
        // The mood change also triggers the scale pop (Finding 1).
        assert_eq!(
            world.query::<&crate::waifu::MoodPop>().iter(&world).len(),
            1
        );
    }
}

/// Marker for the NOC alarm list panel root.
#[derive(Component)]
struct AlarmListPanel;

/// Marker for a single alarm row, holding its list index.
#[derive(Component)]
struct AlarmRow {
    #[allow(dead_code)]
    index: usize,
}

/// Spawn the NOC alarm list panel on the right side of the screen.
/// Shows all alarms in `AlarmList` with severity color coding.
fn spawn_alarm_list_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    alarm_list: Res<AlarmList>,
) {
    commands
        .spawn((
            AlarmListPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(80.0),
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.9)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("NOC ALARMS ({})", alarm_list.unacked_count())),
                TextFont {
                    font: asset_server.load(crate::fonts::DISPLAY_BOLD).into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Refresh the alarm list panel contents from `AlarmList`.
fn update_alarm_list_panel(
    mut commands: Commands,
    alarm_list: Res<AlarmList>,
    asset_server: Res<AssetServer>,
    panel: Query<Entity, With<AlarmListPanel>>,
    rows: Query<Entity, With<AlarmRow>>,
) {
    if !alarm_list.is_changed() {
        return;
    }
    let Ok(panel_entity) = panel.single() else {
        return;
    };
    // Clear old rows (keep the header which is not an AlarmRow).
    for row in &rows {
        commands.entity(row).despawn();
    }
    commands.entity(panel_entity).with_children(|parent| {
        for (i, alarm) in alarm_list.alarms.iter().enumerate() {
            let sev_color = match alarm.severity {
                osp_sim::AlarmSeverity::Critical => Color::srgb(1.0, 0.2, 0.2),
                osp_sim::AlarmSeverity::Major => Color::srgb(1.0, 0.6, 0.2),
                osp_sim::AlarmSeverity::Minor => Color::srgb(1.0, 1.0, 0.3),
                osp_sim::AlarmSeverity::Warning => Color::srgb(0.5, 0.8, 1.0),
            };
            let ack_text = match &alarm.ack {
                AlarmAck::New => "NEW",
                AlarmAck::Acknowledged { companion_idx } => &format!("ACK(c{})", companion_idx),
                AlarmAck::Resolved => "RESOLVED",
                AlarmAck::Cleared => "CLEARED",
            };
            parent.spawn((
                AlarmRow { index: i },
                Text::new(format!(
                    "#{} [{:?}] {} - {}",
                    i + 1,
                    alarm.severity,
                    alarm.outage.kind.flavor_text(),
                    ack_text
                )),
                TextFont {
                    font: asset_server.load(crate::fonts::BODY).into(),
                    font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(sev_color),
            ));
        }
        // Hint text at the bottom
        parent.spawn((
            Text::new("Press 1-9 to ack/dispatch"),
            TextFont {
                font: asset_server.load(crate::fonts::BODY).into(),
                font_size: FontSize::Px(10.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
        ));
    });
}

/// Keyboard input: number keys 1-9 acknowledge the corresponding alarm in
/// display order (see `AlarmList::sorted`), dispatching companion 0 (the
/// active companion) to it.
fn handle_alarm_ack_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut alarm_list: ResMut<AlarmList>,
    time: Res<Time>,
) {
    let digit_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    // Snapshot the ids first: `ack` mutates the list, and the borrow
    // checker will not let us hold `sorted()` refs across it.
    let ordered_ids: Vec<u64> = alarm_list.sorted().iter().map(|a| a.id).collect();
    for (i, key) in digit_keys.iter().enumerate() {
        if keyboard.just_pressed(*key) {
            if let Some(id) = ordered_ids.get(i) {
                alarm_list.ack(*id, 0, time.elapsed_secs_f64());
            }
        }
    }
}

/// Despawn the alarm list panel on state exit.
fn teardown_alarm_list_panel(
    mut commands: Commands,
    query: Query<Entity, With<AlarmListPanel>>,
    mut alarm_list: ResMut<AlarmList>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    // Archive resolved alarms out of the active list.
    alarm_list.clear_resolved();
}
