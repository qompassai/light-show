//! Main menu: title, companion picker, world/level select.

use super::GameState;
use crate::board;
use crate::waifu::{Companion, SelectedCompanion};
use bevy::prelude::*;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // `board::teardown_board` here is a safe no-op the very first time
        // (Startup) this runs, since nothing has been spawned yet. It's
        // needed for every subsequent MainMenu entry (e.g. Results ->
        // MainMenu) now that board teardown is no longer tied to
        // `OnExit(Playing)` — see `playing::PlayingPlugin::build`.
        app.add_systems(
            OnEnter(GameState::MainMenu),
            (board::teardown_board, setup_menu),
        )
        .add_systems(
            Update,
            (
                handle_start_button,
                handle_companion_buttons,
                update_companion_ui,
            )
                .run_if(in_state(GameState::MainMenu)),
        )
        .add_systems(OnExit(GameState::MainMenu), teardown_menu);
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct StartButton;

/// Tags one of the four companion-picker buttons with the companion it
/// selects when pressed.
#[derive(Component)]
struct CompanionButton(Companion);

/// The "Companion: <name>" label kept in sync with `SelectedCompanion`.
#[derive(Component)]
struct CompanionLabel;

/// The one-line medium description under `CompanionLabel` (e.g. "Fiber-optic
/// OSP splicing"), kept in sync with `SelectedCompanion` the same way.
#[derive(Component)]
struct CompanionTagline;

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
            parent.spawn((
                CompanionLabel,
                TextBundle::from_section(
                    format!("Companion: {}", Companion::default().display_name()),
                    TextStyle {
                        font: asset_server.load("fonts/pixel.ttf"),
                        font_size: 16.0,
                        color: Color::srgb(0.8, 0.8, 0.9),
                    },
                ),
            ));
            parent.spawn((
                CompanionTagline,
                TextBundle::from_section(
                    Companion::default().tagline(),
                    TextStyle {
                        font: asset_server.load("fonts/pixel.ttf"),
                        font_size: 13.0,
                        color: Color::srgb(0.55, 0.55, 0.68),
                    },
                ),
            ));
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    for companion in Companion::ALL {
                        row.spawn((
                            CompanionButton(companion),
                            ButtonBundle {
                                style: Style {
                                    padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                                    ..default()
                                },
                                background_color: companion_button_color(
                                    companion,
                                    companion == Companion::default(),
                                )
                                .into(),
                                ..default()
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn(TextBundle::from_section(
                                companion.display_name(),
                                TextStyle {
                                    font: asset_server.load("fonts/pixel.ttf"),
                                    font_size: 16.0,
                                    color: Color::WHITE,
                                },
                            ));
                        });
                    }
                });
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

/// Accent color per companion (mirrors each one's palette in
/// docs/ART_STYLE.md), dimmed to a neutral slate when not selected.
fn companion_button_color(companion: Companion, active: bool) -> Color {
    if !active {
        return Color::srgb(0.2, 0.2, 0.28);
    }
    match companion {
        Companion::Fiber => Color::srgb(0.9, 0.4, 0.6), // seraphine_magenta-ish
        Companion::Coax => Color::srgb(0.72, 0.45, 0.2), // copper
        Companion::Mobile => Color::srgb(0.49, 0.23, 0.91), // electric violet
        Companion::Ethernet => Color::srgb(0.15, 0.39, 0.92), // networking blue
    }
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

/// Sets `SelectedCompanion` when a companion-picker button is pressed.
/// The actual sprite/dialogue swap happens in
/// `waifu::respawn_on_companion_change`, which reacts to that resource.
fn handle_companion_buttons(
    interactions: Query<(&Interaction, &CompanionButton), Changed<Interaction>>,
    mut selected: ResMut<SelectedCompanion>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            selected.0 = button.0;
        }
    }
}

/// Keeps the companion label text and button highlight colors in sync
/// with `SelectedCompanion` after a pick.
fn update_companion_ui(
    selected: Res<SelectedCompanion>,
    mut buttons: Query<(&CompanionButton, &mut BackgroundColor)>,
    mut labels: Query<&mut Text, (With<CompanionLabel>, Without<CompanionTagline>)>,
    mut taglines: Query<&mut Text, (With<CompanionTagline>, Without<CompanionLabel>)>,
) {
    if !selected.is_changed() {
        return;
    }
    for (button, mut bg) in &mut buttons {
        *bg = companion_button_color(button.0, button.0 == selected.0).into();
    }
    for mut text in &mut labels {
        text.sections[0].value = format!("Companion: {}", selected.0.display_name());
    }
    for mut text in &mut taglines {
        text.sections[0].value = selected.0.tagline().to_string();
    }
}

fn teardown_menu(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    fn world_with_selected(initial: Companion) -> World {
        let mut world = World::new();
        world.insert_resource(SelectedCompanion(initial));
        world
    }

    #[test]
    fn pressing_a_companion_button_updates_selected_companion() {
        let mut world = world_with_selected(Companion::Fiber);
        world.spawn((CompanionButton(Companion::Coax), Interaction::Pressed));

        world.run_system_once(handle_companion_buttons);

        assert_eq!(world.resource::<SelectedCompanion>().0, Companion::Coax);
    }

    #[test]
    fn hovering_a_companion_button_does_not_change_selection() {
        let mut world = world_with_selected(Companion::Fiber);
        world.spawn((CompanionButton(Companion::Mobile), Interaction::Hovered));

        world.run_system_once(handle_companion_buttons);

        assert_eq!(world.resource::<SelectedCompanion>().0, Companion::Fiber);
    }

    #[test]
    fn update_companion_ui_relabels_and_recolors_after_a_selection_change() {
        let mut world = world_with_selected(Companion::Fiber);
        world.spawn((
            CompanionLabel,
            TextBundle::from_section("placeholder", TextStyle::default()),
        ));
        world.spawn((
            CompanionButton(Companion::Ethernet),
            BackgroundColor(companion_button_color(Companion::Ethernet, false)),
        ));

        // Simulate the menu button press changing the resource, then run
        // the sync system exactly like `Update` would.
        world.resource_mut::<SelectedCompanion>().0 = Companion::Ethernet;
        world.run_system_once(update_companion_ui);

        let label = world
            .query::<&Text>()
            .iter(&world)
            .find(|t| t.sections[0].value.starts_with("Companion:"))
            .expect("companion label should exist");
        assert_eq!(label.sections[0].value, "Companion: Lattice");

        let bg = world
            .query::<(&CompanionButton, &BackgroundColor)>()
            .iter(&world)
            .next()
            .expect("companion button should exist");
        assert_eq!(bg.1 .0, companion_button_color(Companion::Ethernet, true));
    }

    #[test]
    fn update_companion_ui_is_a_no_op_when_selection_is_unchanged() {
        // Resources are always "changed" on the frame they're inserted,
        // so run the system once first to clear that flag before
        // asserting the no-op branch actually short-circuits.
        let mut world = world_with_selected(Companion::Fiber);
        world.run_system_once(update_companion_ui);

        world.spawn((
            CompanionButton(Companion::Coax),
            BackgroundColor(companion_button_color(Companion::Coax, false)),
        ));
        world.run_system_once(update_companion_ui);

        // Selection never changed after the initial clear, so the freshly
        // spawned button should keep its inactive color untouched.
        let bg = world
            .query::<(&CompanionButton, &BackgroundColor)>()
            .iter(&world)
            .next()
            .unwrap();
        assert_eq!(bg.1 .0, companion_button_color(Companion::Coax, false));
    }

    #[test]
    fn update_companion_ui_relabels_the_tagline_after_a_selection_change() {
        let mut world = world_with_selected(Companion::Fiber);
        world.spawn((
            CompanionTagline,
            TextBundle::from_section("placeholder", TextStyle::default()),
        ));

        world.resource_mut::<SelectedCompanion>().0 = Companion::Ethernet;
        world.run_system_once(update_companion_ui);

        let tagline = world
            .query::<(&CompanionTagline, &Text)>()
            .iter(&world)
            .next()
            .expect("tagline label should exist");
        assert_eq!(tagline.1.sections[0].value, Companion::Ethernet.tagline());
    }

    #[test]
    fn companion_button_color_highlights_only_the_active_companion() {
        for companion in Companion::ALL {
            assert_ne!(
                companion_button_color(companion, true),
                companion_button_color(companion, false),
                "active and inactive colors must differ for {companion:?}"
            );
        }
    }
}
