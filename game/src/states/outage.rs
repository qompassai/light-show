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
                ),
            )
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
}

/// All active NOC alarms. Each outage that fires creates an `Alarm` here;
/// the list supports multiple concurrent alarms with ack/dispatch/clear.
/// The `ActiveOutage` single-outage banner stays for backward compatibility
/// with the existing outage-repair loop.
#[derive(Resource, Default)]
pub struct AlarmList {
    pub alarms: Vec<Alarm>,
}

impl AlarmList {
    /// Push a new alarm from an outage. Returns the alarm index.
    pub fn raise(&mut self, outage: Outage, now_secs: f64) -> usize {
        let id = self.alarms.len() as u64;
        self.alarms.push(Alarm::new(id, outage, now_secs));
        self.alarms.len() - 1
    }

    /// Acknowledge an alarm by index, dispatching a companion.
    /// No-op if out of bounds.
    pub fn acknowledge(&mut self, index: usize, companion_idx: u8) {
        if let Some(alarm) = self.alarms.get_mut(index) {
            alarm.ack = AlarmAck::Acknowledged { companion_idx };
        }
    }

    /// Clear resolved alarms from the list.
    pub fn clear_resolved(&mut self) {
        self.alarms.retain(|a| a.ack != AlarmAck::Cleared);
    }

    /// Count of unacknowledged alarms.
    pub fn unacked_count(&self) -> usize {
        self.alarms.iter().filter(|a| matches!(a.ack, AlarmAck::New)).count()
    }
}

/// Root node of the on-screen alarm banner, despawned `OnExit(OutageActive)`.
#[derive(Component)]
struct OutageBanner;

/// The countdown text child, refreshed every frame by `update_outage_banner`.
#[derive(Component)]
struct OutageBannerText;

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

fn announce_outage(
    mut commands: Commands,
    active: Res<ActiveOutage>,
    sfx: Res<crate::audio::Sfx>,
) {
    sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
    if let Some(outage) = &active.outage {
        info!(
            "OUTAGE: {} (timer: {:.0}s)",
            outage.kind.flavor_text(),
            outage.time_remaining()
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
                Text::new(format!("REPAIR NOW — {:.0}s", outage.time_remaining())),
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
    let Some(outage) = &active.outage else {
        return;
    };
    for mut text in &mut query {
        text.0 = format!("REPAIR NOW — {:.0}s", outage.time_remaining());
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
        commands.entity(entity).insert(BannerSlideOut { elapsed_secs: 0.0 });
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
    let remaining = outage.time_remaining();
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
) {
    let Some(outage) = active.outage.clone() else {
        return;
    };

    // Request the Results transition exactly once (see
    // `check_win_condition`): re-requesting queues a duplicate
    // Results -> Results transition whose OnExit/OnEnter pair rebuilds
    // the results screen mid-frame.
    let transition_pending =
        request.0.is_some() || matches!(*next_state, NextState::Pending(_));

    if !transition_pending && outage.is_expired() {
        sfx.play(&mut commands, crate::audio::SfxKind::Lose);
        outcome.won = false;
        request.0 = Some(GameState::Results);
        return;
    }

    if !transition_pending
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
        request.0 = Some(GameState::Results);
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
            on_enter_line: None,
            on_win_line: None,
            on_fail_line: None,
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
        });
        world.insert_resource(connected_live_graph());
        world.insert_resource(fixture_level((0, 1)));
        world.insert_resource(LevelOutcome { won: true });
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world.init_resource::<AlarmList>();

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
        });
        world.insert_resource(connected_live_graph());
        world.insert_resource(fixture_level((0, 1)));
        world.insert_resource(LevelOutcome { won: false });
        world.init_resource::<TransitionRequest>();
        world.init_resource::<NextState<GameState>>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world.init_resource::<AlarmList>();

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
        assert_eq!(world.query::<&crate::waifu::MoodPop>().iter(&world).len(), 1);
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
                AlarmAck::Acknowledged { companion_idx } => {
                    &format!("ACK(c{})", companion_idx)
                }
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

/// Keyboard input: number keys 1-9 acknowledge the corresponding alarm,
/// dispatching companion 0 (the active companion) to it.
fn handle_alarm_ack_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut alarm_list: ResMut<AlarmList>,
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
    for (i, key) in digit_keys.iter().enumerate() {
        if keyboard.just_pressed(*key) {
            // Dispatch the active companion (index 0) to the alarm.
            alarm_list.acknowledge(i, 0);
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
