//! Companion-select screen: title → here → themed level track. Four
//! cards, one per companion/transmission medium; picking one sets
//! `SelectedCompanion`, jumps `CurrentLevelIndex` to that companion's
//! track start (see `Companion::track_start_index`), and enters
//! `Playing`. This is the screen Matt asked for: the companion pick
//! chooses *what* you learn, not just who comments on it.

use super::GameState;
use crate::level::CurrentLevelIndex;
use crate::ui::neon::{spawn_neon_text, NeonText, NEON_CYAN, NEON_DIM, NEON_GOLD};
use crate::waifu::{Companion, SelectedCompanion};
use bevy::prelude::*;

pub struct CompanionSelectPlugin;

impl Plugin for CompanionSelectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SelectAnimTimer::default())
            .add_systems(OnEnter(GameState::CompanionSelect), setup_select)
            .add_systems(
                Update,
                (
                    handle_select_buttons,
                    handle_back_button,
                    animate_select_cards,
                    highlight_select_cards,
                )
                    .run_if(in_state(GameState::CompanionSelect)),
            )
            .add_systems(OnExit(GameState::CompanionSelect), teardown_select);
    }
}

#[derive(Component)]
struct SelectRoot;

/// Tags a companion card button with the companion it starts a track for.
#[derive(Component)]
pub(crate) struct SelectButton(pub(crate) Companion);

#[derive(Component)]
pub(crate) struct BackButton;

/// One frame of the silhouette strip is 160×256; the card shows it at
/// 96×144.
const SILHOUETTE_DISPLAY: Vec2 = Vec2::new(96.0, 144.0);
/// Frames in each companion-select silhouette animation.
const SELECT_ANIM_FRAMES: usize = 6;
/// Seconds per animation frame (matches the 0.12 s Aseprite frame timing).
const SELECT_ANIM_FRAME_SECS: f32 = 0.12;

/// Tags a companion card with its silhouette animation frames; the image
/// plays only while its card is hovered or pressed (see
/// `animate_select_cards`). At rest every card sits on frame 0. The frames
/// live here (on the card, which carries `Interaction`) rather than on the
/// image because the silhouette is a plain `ImageBundle` child and never
/// gets an `Interaction` component of its own.
#[derive(Component)]
struct SelectAnim {
    frames: [Handle<Image>; SELECT_ANIM_FRAMES],
    index: usize,
}

/// Marks the silhouette `ImageBundle` inside a companion card; the
/// animation system finds it through the card's children.
#[derive(Component)]
struct SelectSilhouette;

/// Staggers silhouette animation frames across highlighted cards.
#[derive(Resource)]
struct SelectAnimTimer(Timer);

impl Default for SelectAnimTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            SELECT_ANIM_FRAME_SECS,
            TimerMode::Repeating,
        ))
    }
}

fn setup_select(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/pixel.ttf");
    commands
        .spawn((
            SelectRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: Color::srgb(0.05, 0.05, 0.12).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Keeper title artwork as a dimmed backdrop: same title card,
            // one step deeper. Absolutely positioned out of the flex flow
            // so it paints behind the cards.
            parent.spawn(ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                image: UiImage::new(asset_server.load("sprites/ui/title_artwork.png")),
                ..default()
            });
            // Darkens the artwork so the cards pop; translucent so the
            // night city still reads through. Dark enough that the
            // artwork's baked-in "LIGHT SHOW" title recedes behind the
            // header and cards instead of colliding with them.
            parent.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: Color::srgba(0.03, 0.03, 0.08, 0.85).into(),
                ..default()
            });
            spawn_neon_text(
                parent,
                NeonText {
                    marker: (),
                    value: "choose your discipline",
                    font: font.clone(),
                    font_size: 26.0,
                    core: NEON_GOLD,
                    glow: NEON_CYAN,
                    glow_px: 1.0,
                    glow_inner_alpha: 0.55,
                    glow_outer_alpha: 0.25,
                    width: Val::Auto,
                    justify: JustifyText::Center,
                    },
            );
            for companion in Companion::ALL {
                spawn_companion_card(parent, &asset_server, &font, companion);
            }
            parent
                .spawn((
                    BackButton,
                    ButtonBundle {
                        style: Style {
                            padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.2, 0.28).into(),
                        ..default()
                    },
                ))
                .with_children(|btn| {
                    btn.spawn(TextBundle::from_section(
                        "Back",
                        TextStyle {
                            font: font.clone(),
                            font_size: 18.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn spawn_companion_card(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    companion: Companion,
) {
    let stem = companion.picker_stem();
    let frames: [Handle<Image>; SELECT_ANIM_FRAMES] =
        std::array::from_fn(|i| asset_server.load(format!("sprites/picker/{stem}_select_{i}.png")));
    let first_frame = frames[0].clone();
    parent
        .spawn((
            SelectButton(companion),
            SelectAnim { frames, index: 0 },
            // The glow outline starts at zero width; `highlight_select_cards`
            // raises it in the companion's accent color on hover/press.
            Outline {
                width: Val::Px(0.0),
                offset: Val::Px(3.0),
                color: companion.accent(),
            },
            ButtonBundle {
                style: Style {
                    width: Val::Px(620.0),
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(18.0),
                    ..default()
                },
                background_color: Color::srgba(0.08, 0.1, 0.18, 0.92).into(),
                ..default()
            },
        ))
        .with_children(|card| {
            // Silhouette: dark shape + discipline-colored outline + FX
            // (see `gen_picker.lua`); animated while the card is
            // highlighted, resting on frame 0 otherwise.
            card.spawn((
                SelectSilhouette,
                ImageBundle {
                    style: Style {
                        width: Val::Px(SILHOUETTE_DISPLAY.x),
                        height: Val::Px(SILHOUETTE_DISPLAY.y),
                        ..default()
                    },
                    image: UiImage::new(first_frame),
                    ..default()
                },
            ));
            card.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|text| {
                // Name: cyan core, gold halo — the title card's answer pattern.
                spawn_neon_text(
                    text,
                    NeonText {
                        marker: (),
                        value: companion.display_name(),
                        font: font.clone(),
                        font_size: 24.0,
                        core: NEON_CYAN,
                        glow: NEON_GOLD,
                        glow_px: 1.0,
                        glow_inner_alpha: 0.55,
                        glow_outer_alpha: 0.25,
                        width: Val::Auto,
                        justify: JustifyText::Left,
                    },
                );
                text.spawn(TextBundle::from_section(
                    companion.tagline(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 14.0,
                        color: NEON_DIM,
                    },
                ));
                text.spawn(TextBundle::from_section(
                    companion.select_hook(),
                    TextStyle {
                        font: font.clone(),
                        font_size: 14.0,
                        color: NEON_GOLD,
                    },
                ));
            });
        });
}

/// Advances the silhouette animation on highlighted cards: each tick moves
/// every hovered/pressed card's `SelectAnim` one frame and swaps its
/// silhouette's `UiImage` texture.
fn animate_select_cards(
    time: Res<Time>,
    mut timer: ResMut<SelectAnimTimer>,
    mut cards: Query<(&Interaction, &mut SelectAnim, &Children)>,
    mut silhouettes: Query<&mut UiImage, With<SelectSilhouette>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    for (interaction, mut anim, children) in &mut cards {
        if *interaction != Interaction::Hovered && *interaction != Interaction::Pressed {
            continue;
        }
        anim.index = (anim.index + 1) % SELECT_ANIM_FRAMES;
        for child in children {
            if let Ok(mut image) = silhouettes.get_mut(*child) {
                image.texture = anim.frames[anim.index].clone();
            }
        }
    }
}

/// Raises the card's glow outline in the companion's accent color while it
/// is hovered or pressed, and drops it back to zero width otherwise.
fn highlight_select_cards(
    mut cards: Query<(&Interaction, &SelectButton, &mut Outline), Changed<Interaction>>,
) {
    for (interaction, button, mut outline) in &mut cards {
        outline.width = match interaction {
            Interaction::Hovered | Interaction::Pressed => Val::Px(3.0),
            Interaction::None => Val::Px(0.0),
        };
        outline.color = button.0.accent();
    }
}

/// A card press selects the companion *and* starts its level track: the
/// sprite/dialogue swap still happens in
/// `waifu::respawn_on_companion_change`, which reacts to the resource.
fn handle_select_buttons(
    interactions: Query<(&Interaction, &SelectButton), Changed<Interaction>>,
    mut selected: ResMut<SelectedCompanion>,
    mut index: ResMut<CurrentLevelIndex>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            selected.0 = button.0;
            index.0 = button.0.track_start_index();
            next_state.set(GameState::Playing);
        }
    }
}

fn handle_back_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
        }
    }
}

fn teardown_select(mut commands: Commands, query: Query<Entity, With<SelectRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::system::RunSystemOnce;

    fn world_with_select_state() -> World {
        let mut world = World::new();
        world.insert_resource(SelectedCompanion(Companion::Fiber));
        world.insert_resource(CurrentLevelIndex(0));
        world.insert_resource(NextState::<GameState>::default());
        world
    }

    #[test]
    fn pressing_a_card_selects_the_companion_and_starts_its_track() {
        let mut world = world_with_select_state();
        world.spawn((SelectButton(Companion::Ethernet), Interaction::Pressed));

        world.run_system_once(handle_select_buttons);

        assert_eq!(
            world.resource::<SelectedCompanion>().0,
            Companion::Ethernet
        );
        assert_eq!(
            world.resource::<CurrentLevelIndex>().0,
            Companion::Ethernet.track_start_index()
        );
        assert!(matches!(
            world.resource::<NextState<GameState>>(),
            NextState::Pending(GameState::Playing)
        ));
    }

    #[test]
    fn pressing_back_returns_to_the_main_menu() {
        let mut world = world_with_select_state();
        world.spawn((BackButton, Interaction::Pressed));

        world.run_system_once(handle_back_button);

        assert!(matches!(
            world.resource::<NextState<GameState>>(),
            NextState::Pending(GameState::MainMenu)
        ));
    }

    #[test]
    fn hovering_a_card_raises_its_accent_outline() {
        let mut world = World::new();
        let card = world
            .spawn((
                SelectButton(Companion::Coax),
                Outline {
                    width: Val::Px(0.0),
                    offset: Val::Px(3.0),
                    color: Color::WHITE,
                },
                Interaction::Hovered,
            ))
            .id();

        world.run_system_once(highlight_select_cards);

        let outline = world.get::<Outline>(card).expect("card keeps its outline");
        assert_eq!(outline.width, Val::Px(3.0));
        assert_eq!(outline.color, Companion::Coax.accent());
    }

    #[test]
    fn unhovering_a_card_drops_its_outline() {
        let mut world = World::new();
        let card = world
            .spawn((
                SelectButton(Companion::Mobile),
                Outline {
                    width: Val::Px(3.0),
                    offset: Val::Px(3.0),
                    color: Companion::Mobile.accent(),
                },
                Interaction::None,
            ))
            .id();

        world.run_system_once(highlight_select_cards);

        let outline = world.get::<Outline>(card).expect("card keeps its outline");
        assert_eq!(outline.width, Val::Px(0.0));
    }

    #[test]
    fn hovering_advances_the_silhouette_frame() {
        use std::time::Duration;

        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_secs_f32(1.0));
        world.insert_resource(time);
        world.insert_resource(SelectAnimTimer::default());
        let silhouette = world
            .spawn((SelectSilhouette, UiImage::new(Handle::default())))
            .id();
        let card = world
            .spawn((
                SelectButton(Companion::Fiber),
                SelectAnim {
                    frames: std::array::from_fn(|_| Handle::default()),
                    index: 0,
                },
                Interaction::Hovered,
            ))
            .id();
        world.entity_mut(card).add_child(silhouette);

        world.run_system_once(animate_select_cards);

        assert_eq!(
            world
                .get::<SelectAnim>(card)
                .expect("card keeps its animation")
                .index,
            1
        );
    }

    #[test]
    fn companion_accents_are_distinct() {
        let accents: Vec<Color> = Companion::ALL.iter().map(Companion::accent).collect();
        for (i, a) in accents.iter().enumerate() {
            for b in &accents[i + 1..] {
                assert_ne!(a, b, "each companion needs a distinct accent color");
            }
        }
    }

    #[test]
    fn every_companion_starts_a_distinct_two_level_track() {
        let starts: Vec<usize> = Companion::ALL
            .iter()
            .map(|c| c.track_start_index())
            .collect();
        assert_eq!(starts, vec![0, 2, 4, 6]);
        assert_eq!(Companion::TRACK_LEN, 2);
    }
}
