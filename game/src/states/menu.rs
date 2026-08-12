//! Main menu: title, world/level select, Séraphine idle greeting.

use super::GameState;
use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), setup_menu)
            .add_systems(
                Update,
                handle_start_button.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), teardown_menu);
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct StartButton;

fn setup_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn((
            MenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(24.0),
                    ..default()
                },
                background_color: Color::srgb(0.05, 0.05, 0.12).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "LIGHT SHOW",
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 56.0,
                    color: Color::srgb(0.6, 0.95, 1.0),
                },
            ));
            parent.spawn(TextBundle::from_section(
                "route the light. hit the window. survive the storm.",
                TextStyle {
                    font: asset_server.load("fonts/pixel.ttf"),
                    font_size: 18.0,
                    color: Color::srgb(0.8, 0.8, 0.9),
                },
            ));
            parent
                .spawn((
                    StartButton,
                    ButtonBundle {
                        style: Style {
                            padding: UiRect::axes(Val::Px(28.0), Val::Px(14.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.9, 0.4, 0.6).into(),
                        ..default()
                    },
                ))
                .with_children(|btn| {
                    btn.spawn(TextBundle::from_section(
                        "Start Splicing",
                        TextStyle {
                            font: asset_server.load("fonts/pixel.ttf"),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn handle_start_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Playing);
        }
    }
}

fn teardown_menu(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
