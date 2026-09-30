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
use crate::level::LevelDef;
use crate::waifu::{CompanionSprite, Mood};
use bevy::prelude::*;
use osp_sim::Outage;

pub struct OutagePlugin;

impl Plugin for OutagePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveOutage::default())
            .add_systems(
                OnEnter(GameState::OutageActive),
                (announce_outage, spawn_outage_banner, alarm_companion),
            )
            .add_systems(
                Update,
                (tick_outage, update_outage_banner, check_outage_resolution)
                    .chain()
                    .run_if(in_state(GameState::OutageActive)),
            )
            .add_systems(OnExit(GameState::OutageActive), teardown_outage_banner);
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

/// Root node of the on-screen alarm banner, despawned `OnExit(OutageActive)`.
#[derive(Component)]
struct OutageBanner;

/// The countdown text child, refreshed every frame by `update_outage_banner`.
#[derive(Component)]
struct OutageBannerText;

fn announce_outage(active: Res<ActiveOutage>) {
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
fn alarm_companion(mut query: Query<&mut CompanionSprite>) {
    for mut sprite in &mut query {
        sprite.mood = Mood::Alarmed;
        sprite.frame = 0;
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
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::vertical(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                background_color: Color::srgba(0.65, 0.0, 0.05, 0.85).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                outage.kind.flavor_text(),
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 16.0,
                    color: Color::WHITE,
                },
            ));
            parent.spawn((
                OutageBannerText,
                TextBundle::from_section(
                    format!("REPAIR NOW — {:.0}s", outage.time_remaining()),
                    TextStyle {
                        font: asset_server.load("fonts/pixel.ttf"),
                        font_size: 20.0,
                        color: Color::srgb(1.0, 0.82, 0.4), // #ffd166
                    },
                ),
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
        text.sections[0].value = format!("REPAIR NOW — {:.0}s", outage.time_remaining());
    }
}

fn teardown_outage_banner(mut commands: Commands, query: Query<Entity, With<OutageBanner>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn tick_outage(time: Res<Time>, mut active: ResMut<ActiveOutage>) {
    if let Some(outage) = &mut active.outage {
        outage.tick(time.delta_seconds_f64());
    }
}

/// Resolves the active outage against the live link budget: a timeout
/// ends the level as a loss, and a repaired budget that lands back in
/// window marks the outage resolved and ends the level as a win. Both
/// outcomes go straight to `Results` — see the module doc comment for
/// why this never routes back to `Playing`.
fn check_outage_resolution(
    mut active: ResMut<ActiveOutage>,
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    mut outcome: ResMut<LevelOutcome>,
    mut next_state: ResMut<NextState<GameState>>,
    mut companions: Query<&mut CompanionSprite>,
) {
    let Some(outage) = active.outage.clone() else {
        return;
    };

    if outage.is_expired() {
        outcome.won = false;
        next_state.set(GameState::Results);
        return;
    }

    let Ok(result) = live.graph.compute_link_budget_with_outage(
        level.source_node,
        level.target_node,
        live.tx_dbm,
        live.wavelength.0,
        level.receive_window(),
        Some(&outage),
    ) else {
        return;
    };

    if result.in_window {
        if let Some(active_outage) = active.outage.as_mut() {
            active_outage.resolved = true;
        }
        outcome.won = true;
        // A "nice save" reaction distinct from a plain, drama-free clear
        // (see `states::playing::check_win_condition`'s `Mood::Celebrate`).
        for mut sprite in &mut companions {
            sprite.mood = Mood::Wink;
            sprite.frame = 0;
        }
        next_state.set(GameState::Results);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{ComponentChoice, LevelNode, WavelengthDef};
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
            tx_dbm: -15.0,
            wavelength: WavelengthDef::Nm1490,
            window_min_dbm: -27.0,
            window_max_dbm: -8.0,
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
        world.init_resource::<NextState<GameState>>();

        world.run_system_once(check_outage_resolution);

        assert!(!world.resource::<LevelOutcome>().won);
        assert!(matches!(
            world.resource::<NextState<GameState>>(),
            NextState::Pending(GameState::Results)
        ));
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
        world.init_resource::<NextState<GameState>>();

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
        assert!(matches!(
            world.resource::<NextState<GameState>>(),
            NextState::Pending(GameState::Results)
        ));
    }

    #[test]
    fn no_active_outage_is_a_no_op() {
        let mut world = World::new();
        world.insert_resource(ActiveOutage::default());
        world.insert_resource(connected_live_graph());
        world.insert_resource(fixture_level((0, 1)));
        world.insert_resource(LevelOutcome { won: true });
        world.init_resource::<NextState<GameState>>();

        world.run_system_once(check_outage_resolution);

        assert!(matches!(
            world.resource::<NextState<GameState>>(),
            NextState::Unchanged
        ));
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

        let sprite = world.query::<&CompanionSprite>().single(&world);
        assert_eq!(sprite.mood, Mood::Alarmed);
        assert_eq!(sprite.frame, 0);
    }
}
