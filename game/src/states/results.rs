//! Results screen: shows win/fail, the final link-budget ledger, and the
//! matching companion reaction line (from whichever `DialogueBank` is
//! currently loaded for `SelectedCompanion`), then offers to continue to
//! the next bundled level, retry the current one, or return to the menu.

use super::outage::ActiveOutage;
use super::playing::LiveGraph;
use super::{GameState, LevelOutcome};
use crate::board;
use crate::level::{self, CurrentLevelIndex, LevelDef};
use crate::waifu::dialogue::DialogueBank;
use crate::waifu::FavorPoints;
use bevy::prelude::*;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Results),
            (board::teardown_board, show_results),
        )
        .add_systems(
            Update,
            handle_result_buttons.run_if(in_state(GameState::Results)),
        )
        .add_systems(OnExit(GameState::Results), teardown_results);
    }
}

#[derive(Component)]
struct ResultsRoot;

/// Which follow-up action a results-screen button performs when pressed.
#[derive(Component, Clone, Copy)]
enum ResultAction {
    ContinueNextLevel,
    RetrySameLevel,
    ReturnToMenu,
}

// Bevy systems idiomatically take one parameter per resource/query they
// need; splitting this into sub-systems to dodge the arg-count lint would
// only fragment one cohesive "build the results screen" operation across
// artificial resource bags, so the lint is waived here rather than the
// system.
#[allow(clippy::too_many_arguments)]
fn show_results(
    mut commands: Commands,
    outcome: Res<LevelOutcome>,
    level: Res<LevelDef>,
    live: Res<LiveGraph>,
    active_outage: Res<ActiveOutage>,
    index: Res<CurrentLevelIndex>,
    dialogue: Res<DialogueBank>,
    asset_server: Res<AssetServer>,
    mut favor: ResMut<FavorPoints>,
) {
    info!("Level complete — won={}", outcome.won);
    crate::test_log!("level_result won={}", outcome.won);

    // A clean win earns favor points, spendable later on optional hints
    // (see `crate::waifu::FavorPoints`); losses earn nothing.
    const FAVOR_PER_WIN: u32 = 10;
    if outcome.won {
        favor.0 = favor.0.saturating_add(FAVOR_PER_WIN);
    }

    let ledger = live.graph.compute_link_budget_with_outage(
        level.source_node,
        level.target_node,
        live.tx_dbm,
        live.wavelength.0,
        level.receive_window(),
        active_outage.outage.as_ref(),
    );

    let (banner_text, banner_color) = if outcome.won {
        ("SERVICE RESTORED", Color::srgb(0.4, 0.9, 0.5))
    } else {
        ("OUTAGE TIMED OUT", Color::srgb(1.0, 0.302, 0.302)) // #ff4d4d
    };

    let ledger_text = match ledger {
        Ok(result) => format!(
            "Loss: {:.2} dB  |  Rx: {:.2} dBm  |  Margin: {:.2} dB",
            result.total_loss_db, result.received_dbm, result.margin_db
        ),
        Err(_) => "Link disconnected — no route completed.".to_string(),
    };

    let dialogue_key = if outcome.won {
        level.on_win_line.as_deref().unwrap_or("level_win")
    } else {
        level.on_fail_line.as_deref().unwrap_or("level_fail_cold")
    };
    let dialogue_line = dialogue
        .random_line(dialogue_key)
        .unwrap_or("...")
        .to_string();

    let has_next_level = index.0 + 1 < level::LEVEL_SOURCES.len();

    commands
        .spawn((
            ResultsRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(18.0),
                    ..default()
                },
                background_color: Color::srgb(0.05, 0.05, 0.12).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                banner_text,
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 48.0,
                    color: banner_color,
                },
            ));
            parent.spawn(TextBundle::from_section(
                format!("World {} — {}", level.world, level.title),
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 20.0,
                    color: Color::srgb(0.8, 0.8, 0.9),
                },
            ));
            parent.spawn(TextBundle::from_section(
                ledger_text,
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 16.0,
                    color: Color::srgb(0.357, 0.753, 0.922), // #5bc0eb
                },
            ));
            parent.spawn(TextBundle::from_section(
                format!("\u{201c}{dialogue_line}\u{201d}"),
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 16.0,
                    color: Color::srgb(1.0, 0.435, 0.682), // #ff6fae
                },
            ));

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    if outcome.won {
                        if has_next_level {
                            spawn_result_button(
                                row,
                                &asset_server,
                                ResultAction::ContinueNextLevel,
                                "Continue",
                                Color::srgb(0.9, 0.4, 0.6),
                            );
                        } else {
                            row.spawn(TextBundle::from_section(
                                "All bundled levels complete!",
                                TextStyle {
                                    font: asset_server.load("fonts/pixel.ttf"),
                                    font_size: 16.0,
                                    color: Color::srgb(0.8, 0.8, 0.9),
                                },
                            ));
                        }
                    } else {
                        spawn_result_button(
                            row,
                            &asset_server,
                            ResultAction::RetrySameLevel,
                            "Retry",
                            Color::srgb(0.9, 0.4, 0.6),
                        );
                    }
                    spawn_result_button(
                        row,
                        &asset_server,
                        ResultAction::ReturnToMenu,
                        "Main Menu",
                        Color::srgb(0.2, 0.2, 0.28),
                    );
                });
        });
}

fn spawn_result_button(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    action: ResultAction,
    label: &str,
    color: Color,
) {
    parent
        .spawn((
            action,
            ButtonBundle {
                style: Style {
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                    ..default()
                },
                background_color: color.into(),
                ..default()
            },
        ))
        .with_children(|btn| {
            btn.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ));
        });
}

fn handle_result_buttons(
    interactions: Query<(&Interaction, &ResultAction), Changed<Interaction>>,
    mut index: ResMut<CurrentLevelIndex>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            ResultAction::ContinueNextLevel => {
                index.0 = index
                    .0
                    .saturating_add(1)
                    .min(level::LEVEL_SOURCES.len() - 1);
                next_state.set(GameState::Playing);
            }
            ResultAction::RetrySameLevel => {
                next_state.set(GameState::Playing);
            }
            ResultAction::ReturnToMenu => {
                index.0 = 0;
                next_state.set(GameState::MainMenu);
            }
        }
    }
}

fn teardown_results(mut commands: Commands, query: Query<Entity, With<ResultsRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
