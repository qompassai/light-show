//! Level-select screen: companion picker → here → `Playing`. Picking a
//! companion on the picker sets `SelectedCompanion` and lands on this
//! screen; the screen lists every level of her track in play order,
//! each row showing the level's save state (cleared or not, plus any
//! Astra badges the save recorded for it). Choosing a row performs the
//! level launch: `CurrentLevelIndex` is set to the level, any Field
//! School detour is cleared, and `Playing` is requested — the same
//! launch contract the picker used to perform itself.
//!
//! The game has no level-lock rule: completion is recorded in the save
//! but never gates access (a companion pick always restarted her track
//! at level 1 before this screen existed), so every row is enterable
//! and the save only ever *annotates* the list, never filters it.

use super::GameState;
use crate::anim::TransitionRequest;
use crate::fonts::FONT_SIZE_ADJUST;
use crate::level::{self, CurrentLevelIndex, CurrentScenarioId};
use crate::responsive::ArtBackdrop;
use crate::save::SaveData;
use crate::ui::neon::{spawn_neon_text, NeonText, NEON_CYAN, NEON_DIM, NEON_GOLD};
use crate::ui::{styled_button, ButtonPalette, BUTTON_BORDER};
use crate::waifu::{Companion, SelectedCompanion};
use bevy::prelude::*;

pub struct LevelSelectPlugin;

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::LevelSelect), setup_level_select)
            .add_systems(
                Update,
                (
                    handle_level_buttons,
                    handle_level_back_button,
                    handle_level_select_keys,
                )
                    .run_if(in_state(GameState::LevelSelect)),
            )
            .add_systems(OnExit(GameState::LevelSelect), teardown_level_select);
    }
}

/// Root of the level-select screen's UI tree; teardown despawns it.
#[derive(Component)]
struct LevelSelectRoot;

/// One level row. The payload is the level's global index into
/// `level::LEVEL_SOURCES` — the same index the launch writes to
/// `CurrentLevelIndex`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelButton(pub usize);

/// The screen's Back button: returns to the companion picker.
#[derive(Component)]
pub struct LevelBackButton;

/// One row of the level list, derived from the level registry and the
/// save. Pure data so the derivation is testable without a UI tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelEntry {
    /// Global index into `level::LEVEL_SOURCES`.
    pub index: usize,
    /// 1-based position within the companion's track.
    pub position: usize,
    /// The level's stable id (the save's key for progress).
    pub id: String,
    /// The level's display title.
    pub title: String,
    /// True when the save records this level as completed.
    pub completed: bool,
    /// The Astra badge bitmask the save recorded for this level
    /// (see `crate::astra::badge_names`).
    pub badges: u8,
}

/// The level list for `companion`'s track, in play order, annotated
/// with the save's per-level progress. Levels the save has never
/// touched read as not completed with no badges; progress recorded
/// for other tracks never leaks in (progress is keyed by level id,
/// and ids are unique per level).
pub fn track_entries(companion: Companion, save: &SaveData) -> Vec<LevelEntry> {
    companion
        .track_indices()
        .into_iter()
        .enumerate()
        .map(|(offset, index)| {
            let def = level::load_level(index);
            LevelEntry {
                index,
                position: offset + 1,
                completed: save.is_completed(&def.id),
                badges: save.badges_for(&def.id),
                id: def.id,
                title: def.title,
            }
        })
        .collect()
}

/// The right-aligned status text for one row: uncleared levels say so,
/// cleared levels say `CLEARED` plus the display names of any badges
/// the save recorded, in bit order.
pub fn status_label(entry: &LevelEntry) -> String {
    if !entry.completed {
        return "not cleared".to_string();
    }
    let names = crate::astra::badge_names(entry.badges);
    if names.is_empty() {
        "CLEARED".to_string()
    } else {
        format!("CLEARED · {}", names.join(" · "))
    }
}

fn setup_level_select(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected: Res<SelectedCompanion>,
    save: Option<Res<SaveData>>,
) {
    let companion = selected.0;
    let empty_save = SaveData::default();
    let entries = track_entries(companion, save.as_deref().unwrap_or(&empty_save));
    let cleared_count = entries.iter().filter(|entry| entry.completed).count();

    let display: Handle<Font> = asset_server.load(crate::fonts::DISPLAY);
    let body: Handle<Font> = asset_server.load(crate::fonts::BODY);
    let body_medium: Handle<Font> = asset_server.load(crate::fonts::BODY_MEDIUM);
    commands
        .spawn((
            LevelSelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.12)),
        ))
        .with_children(|parent| {
            // Same title-art backdrop + dimmer as the picker: this
            // screen is the picker's next step, so it keeps its room.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(asset_server.load("sprites/ui/title_artwork.png"))
                    .with_mode(NodeImageMode::Stretch),
                ArtBackdrop::title(),
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.03, 0.03, 0.08, 0.85)),
            ));
            spawn_level_header(
                parent,
                &asset_server,
                &display,
                &body,
                companion,
                cleared_count,
                entries.len(),
            );
            for entry in &entries {
                spawn_level_row(parent, &display, &body, &body_medium, companion, entry);
            }
            parent
                .spawn((
                    LevelBackButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                        margin: UiRect::top(Val::Px(8.0)),
                        border: BUTTON_BORDER,
                        border_radius: crate::ui::BUTTON_RADIUS,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.28)),
                    styled_button(ButtonPalette::back()),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("Back"),
                        TextFont {
                            font: body_medium.clone().into(),
                            font_size: FontSize::Px(18.0 * FONT_SIZE_ADJUST),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

/// The header: her portrait (every shipped portrait is 2:3, so the
/// 84×126 box stretches nothing), her name in the picker's neon
/// pattern, her tagline, and the track's progress from the save.
fn spawn_level_header(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    display: &Handle<Font>,
    body: &Handle<Font>,
    companion: Companion,
    cleared_count: usize,
    track_len: usize,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            margin: UiRect::bottom(Val::Px(6.0)),
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Node {
                    width: Val::Px(84.0),
                    height: Val::Px(126.0),
                    ..default()
                },
                ImageNode::new(asset_server.load(companion.portrait_path())),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|text| {
                    spawn_neon_text(
                        text,
                        NeonText {
                            marker: (),
                            value: companion.display_name(),
                            font: display.clone(),
                            font_size: 26.0,
                            core: NEON_CYAN,
                            glow: NEON_GOLD,
                            glow_px: 1.0,
                            glow_inner_alpha: 0.55,
                            glow_outer_alpha: 0.25,
                            width: Val::Auto,
                            justify: Justify::Left,
                        },
                    );
                    text.spawn((
                        Text::new(companion.tagline()),
                        TextFont {
                            font: body.clone().into(),
                            font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                            ..default()
                        },
                        TextColor(NEON_DIM),
                    ));
                    text.spawn((
                        Text::new(format!("{cleared_count} of {track_len} levels cleared")),
                        TextFont {
                            font: body.clone().into(),
                            font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                            ..default()
                        },
                        TextColor(NEON_GOLD),
                    ));
                });
        });
}

/// One level row: track position in her accent color, the level's
/// title, and its save status on the right. Cleared rows wear a
/// slightly lifted face until the button palette takes over on hover
/// (the palette only writes on interaction change, the same truce the
/// results screen's entrance fade relies on).
fn spawn_level_row(
    parent: &mut ChildSpawnerCommands,
    display: &Handle<Font>,
    body: &Handle<Font>,
    body_medium: &Handle<Font>,
    companion: Companion,
    entry: &LevelEntry,
) {
    let face = if entry.completed {
        Color::srgb(0.16, 0.19, 0.23)
    } else {
        Color::srgb(0.2, 0.2, 0.28)
    };
    parent
        .spawn((
            LevelButton(entry.index),
            Button,
            Node {
                width: Val::Px(620.0),
                // On windows narrower than the design width the row
                // yields to the window instead of clipping off-screen.
                max_width: Val::Percent(92.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(7.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                border: BUTTON_BORDER,
                border_radius: crate::ui::BUTTON_RADIUS,
                ..default()
            },
            BackgroundColor(face),
            styled_button(ButtonPalette::back()),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(format!("{:02}", entry.position)),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(companion.accent()),
            ));
            row.spawn((
                Text::new(entry.title.clone()),
                TextFont {
                    font: body_medium.clone().into(),
                    font_size: FontSize::Px(16.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
            row.spawn((
                Text::new(status_label(entry)),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(if entry.completed { NEON_GOLD } else { NEON_DIM }),
            ));
        });
}

/// A row press launches that level: the launch contract the picker
/// used to perform on a card press. Starting a track level ends any
/// Field School detour — the track index, not a stale scenario id,
/// decides the level (`level::load_current_level`).
fn handle_level_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &LevelButton), Changed<Interaction>>,
    mut index: ResMut<CurrentLevelIndex>,
    mut scenario: ResMut<CurrentScenarioId>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            sfx.play(&mut commands, crate::audio::SfxKind::Pick);
            index.0 = button.0;
            scenario.0 = None;
            request.0 = Some(GameState::Playing);
        }
    }
}

fn handle_level_back_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<LevelBackButton>)>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
            request.0 = Some(GameState::CompanionSelect);
        }
    }
}

/// Escape is the keyboard's way back — the pointer's is the Back
/// button. Menus elsewhere in the game are pointer-only; the key is
/// additive here because this screen sits on the keyboard's natural
/// "back out one step" path from `Playing`.
fn handle_level_select_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut request: ResMut<TransitionRequest>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        request.0 = Some(GameState::CompanionSelect);
    }
}

fn teardown_level_select(mut commands: Commands, query: Query<Entity, With<LevelSelectRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    fn world_with_level_state(companion: Companion) -> World {
        let mut world = World::new();
        world.insert_resource(SelectedCompanion(companion));
        world.insert_resource(CurrentLevelIndex(0));
        world.insert_resource(CurrentScenarioId::default());
        world.init_resource::<TransitionRequest>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world
    }

    #[test]
    fn empty_save_lists_the_whole_track_in_order() {
        let entries = track_entries(Companion::Fiber, &SaveData::default());
        assert_eq!(entries.len(), Companion::Fiber.track_len());
        for (offset, entry) in entries.iter().enumerate() {
            let def = level::load_level(entry.index);
            assert_eq!(entry.index, Companion::Fiber.track_indices()[offset]);
            assert_eq!(entry.position, offset + 1);
            assert_eq!(entry.id, def.id);
            assert_eq!(entry.title, def.title);
            assert!(!entry.completed, "an untouched save clears nothing");
            assert_eq!(entry.badges, 0);
            assert_eq!(status_label(entry), "not cleared");
        }
    }

    #[test]
    fn completions_and_badges_come_from_the_save() {
        let mut save = SaveData::default();
        let plain = level::load_level(10); // c1l1
        let badged = level::load_level(11); // c1l2
        assert!(save.complete_level(&plain.id));
        assert!(save.complete_level(&badged.id));
        assert!(save.record_badges(&badged.id, 0b101));

        let entries = track_entries(Companion::Coax, &save);
        let plain_entry = &entries[0];
        assert!(plain_entry.completed);
        assert_eq!(plain_entry.badges, 0);
        assert_eq!(status_label(plain_entry), "CLEARED");
        let badged_entry = &entries[1];
        assert!(badged_entry.completed);
        assert_eq!(badged_entry.badges, 0b101);
        assert_eq!(
            status_label(badged_entry),
            "CLEARED · Diagnosis · Verification"
        );
        assert!(entries[2..].iter().all(|entry| !entry.completed));
    }

    #[test]
    fn other_track_progress_never_leaks_into_the_list() {
        // Adversarial: a save rich in Fiber progress must not annotate
        // a single Coax row — progress is keyed by level id and ids
        // are unique per level, so any leak is a derivation bug.
        let mut save = SaveData::default();
        for index in Companion::Fiber.track_indices() {
            let def = level::load_level(index);
            assert!(save.complete_level(&def.id));
            assert!(save.record_badges(&def.id, 0b111));
        }
        let entries = track_entries(Companion::Coax, &save);
        assert_eq!(entries.len(), Companion::Coax.track_len());
        assert!(entries.iter().all(|entry| !entry.completed));
        assert!(entries.iter().all(|entry| entry.badges == 0));
    }

    #[test]
    fn fully_completed_save_marks_every_entry() {
        let mut save = SaveData::default();
        for index in Companion::Mobile.track_indices() {
            let def = level::load_level(index);
            assert!(save.complete_level(&def.id));
        }
        let entries = track_entries(Companion::Mobile, &save);
        assert!(entries.iter().all(|entry| entry.completed));
        assert!(entries
            .iter()
            .all(|entry| status_label(entry).starts_with("CLEARED")));
    }

    #[test]
    fn pressing_a_level_row_launches_it_and_clears_any_scenario() {
        let mut world = world_with_level_state(Companion::Coax);
        // A stale Field School detour must not survive a track launch.
        world.resource_mut::<CurrentScenarioId>().0 = Some("fj1".to_string());
        world.spawn((LevelButton(13), Interaction::Pressed));

        world
            .run_system_once(handle_level_buttons)
            .expect("level button system runs");

        assert_eq!(world.resource::<CurrentLevelIndex>().0, 13);
        assert_eq!(world.resource::<CurrentScenarioId>().0, None);
        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Playing)
        ));
    }

    #[test]
    fn pressing_back_returns_to_the_picker_without_launching() {
        let mut world = world_with_level_state(Companion::Fiber);
        world.spawn((LevelBackButton, Interaction::Pressed));

        world
            .run_system_once(handle_level_back_button)
            .expect("back button system runs");

        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::CompanionSelect)
        ));
        assert_eq!(
            world.resource::<CurrentLevelIndex>().0,
            0,
            "going back must not move the level index"
        );
    }

    #[test]
    fn escape_returns_to_the_picker() {
        let mut world = world_with_level_state(Companion::Fiber);
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(KeyCode::Escape);
        world.insert_resource(keyboard);

        world
            .run_system_once(handle_level_select_keys)
            .expect("key system runs");

        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::CompanionSelect)
        ));
    }
}
