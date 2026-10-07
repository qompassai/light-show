//! Companion-select screen: title → here → themed level track. Four
//! cards, one per companion/transmission medium; picking one sets
//! `SelectedCompanion`, jumps `CurrentLevelIndex` to that companion's
//! track start (see `Companion::track_start_index`), and enters
//! `Playing`. This is the screen Matt asked for: the companion pick
//! chooses *what* you learn, not just who comments on it.

use super::GameState;
use crate::anim::TransitionRequest;
use crate::cheat_codes::{
    CodeWordBuffer, KonamiState, UnlockedSpecialists, code_word_to_companion,
};
use crate::fonts::FONT_SIZE_ADJUST;
use crate::level::{self, CurrentLevelIndex, CurrentScenarioId};
use crate::responsive::ArtBackdrop;
use crate::ui::neon::{NEON_CYAN, NEON_DIM, NEON_GOLD, NeonText, spawn_neon_text};
use crate::ui::{BUTTON_BORDER, ButtonPalette, styled_button};
use crate::waifu::{Companion, SelectedCompanion};
use bevy::prelude::*;

pub struct CompanionSelectPlugin;

impl Plugin for CompanionSelectPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SelectAnimTimer::default())
            .init_resource::<UnlockedSpecialists>()
            .init_resource::<KonamiState>()
            .init_resource::<CodeWordBuffer>()
            .add_systems(OnEnter(GameState::CompanionSelect), setup_select)
            .add_systems(
                Update,
                (
                    handle_select_buttons,
                    handle_scenario_buttons,
                    handle_back_button,
                    animate_select_cards,
                    normalize_select_silhouettes,
                    highlight_select_cards,
                    // Words before Konami: `KonamiState::consumes` must see
                    // the pre-feed progress. If Konami fed first, progress
                    // would already have advanced past the key and B/A would
                    // be misclassified as plain letters.
                    detect_code_words.before(detect_konami_code),
                    detect_konami_code,
                )
                    .run_if(in_state(GameState::CompanionSelect)),
            )
            .add_systems(OnExit(GameState::CompanionSelect), teardown_select);
    }
}

/// Watches for the Konami Code: UP UP DOWN DOWN LEFT RIGHT LEFT RIGHT B A START.
/// Unlocks all four specialist companions at once.
fn detect_konami_code(
    mut konami: ResMut<KonamiState>,
    mut unlocked: ResMut<UnlockedSpecialists>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for key in keyboard.get_just_pressed() {
        if konami.feed(*key) {
            unlocked.unlock_all();
            info!("KONAMI: all specialists unlocked!");
        }
    }
}

/// Watches for typed code words: JUSTINBAILEY, ABACABB, BLASTPROCESSING, TRIFORCE.
/// Each unlocks its companion. Backspace clears the buffer. Keys that advance
/// the Konami sequence are skipped (must run before `detect_konami_code`).
fn detect_code_words(
    mut buffer: ResMut<CodeWordBuffer>,
    mut unlocked: ResMut<UnlockedSpecialists>,
    keyboard: Res<ButtonInput<KeyCode>>,
    konami: Res<KonamiState>,
) {
    // Letter keys A-Z
    for key in keyboard.get_just_pressed() {
        if konami.consumes(*key) {
            // The Konami B and A would otherwise land here as letters "BA".
            // Matching is exact-whole-buffer, so that stray prefix would
            // block every later code word until Backspace/Enter.
            continue;
        }
        let c = match key {
            KeyCode::KeyA => 'A',
            KeyCode::KeyB => 'B',
            KeyCode::KeyC => 'C',
            KeyCode::KeyD => 'D',
            KeyCode::KeyE => 'E',
            KeyCode::KeyF => 'F',
            KeyCode::KeyG => 'G',
            KeyCode::KeyH => 'H',
            KeyCode::KeyI => 'I',
            KeyCode::KeyJ => 'J',
            KeyCode::KeyK => 'K',
            KeyCode::KeyL => 'L',
            KeyCode::KeyM => 'M',
            KeyCode::KeyN => 'N',
            KeyCode::KeyO => 'O',
            KeyCode::KeyP => 'P',
            KeyCode::KeyQ => 'Q',
            KeyCode::KeyR => 'R',
            KeyCode::KeyS => 'S',
            KeyCode::KeyT => 'T',
            KeyCode::KeyU => 'U',
            KeyCode::KeyV => 'V',
            KeyCode::KeyW => 'W',
            KeyCode::KeyX => 'X',
            KeyCode::KeyY => 'Y',
            KeyCode::KeyZ => 'Z',
            KeyCode::Backspace => {
                buffer.clear();
                continue;
            }
            KeyCode::Enter => {
                // Check the buffer on Enter
                if let Some(companion) = code_word_to_companion(&buffer.buffer) {
                    if unlocked.unlock(companion) {
                        info!("CODE: {:?} unlocked!", companion);
                    }
                }
                buffer.clear();
                continue;
            }
            _ => continue,
        };
        buffer.push(c);
        // Check after each keypress too (for codes without Enter)
        if let Some(companion) = code_word_to_companion(&buffer.buffer) {
            if unlocked.unlock(companion) {
                info!("CODE: {:?} unlocked!", companion);
            }
            buffer.clear();
        }
    }
}

#[derive(Component)]
struct SelectRoot;

/// Tags a companion card button with the companion it starts a track for.
#[derive(Component)]
pub(crate) struct SelectButton(pub(crate) Companion);

/// Tags a Field School button with the scenario level id it starts
/// (see `level::SCENARIO_IDS`). Scenario levels are out-of-track: the
/// button sets `CurrentScenarioId`, never a track position.
#[derive(Component)]
pub(crate) struct ScenarioButton(pub(crate) &'static str);

/// Whether the Field School row shows on the picker: the scenarios are
/// Séraphine's field school, so the row belongs to her discipline —
/// it shows while Séraphine/Fiber is the selected companion.
fn field_school_visible(selected: Companion) -> bool {
    selected == Companion::Fiber
}

#[derive(Component)]
pub(crate) struct BackButton;

/// Placeholder box for a card silhouette, and the display height
/// every silhouette is normalized to. The shipped frames are 64×64
/// canvases whose drawn content fills a different share per companion
/// (Séraphine/Linka stand ~50 px tall in theirs, Lattice/Ondine the
/// full 64), so the raw canvas stretched into this 2:3 box rendered
/// each companion at a different size, squashed vertically — the
/// first playtest's picker complaint. `normalize_select_silhouettes`
/// crops each card to its content and re-derives the width from the
/// content aspect at this height.
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
    /// Set once `normalize_select_silhouettes` has cropped this card's
    /// silhouette to its drawn content and sized it to the shared
    /// display height.
    normalized: bool,
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

/// The Field School row: a labeled row of one button per scenario
/// level (fj1/fj2/sp1), shown under the companion cards while
/// Séraphine is the selected companion (see `field_school_visible`).
/// Button labels are the scenario levels' own titles, loaded through
/// the scenario registry — no second copy of the strings lives here.
fn spawn_field_school_row(
    parent: &mut ChildSpawnerCommands,
    display: &Handle<Font>,
    body: &Handle<Font>,
    body_medium: &Handle<Font>,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        })
        .with_children(|section| {
            spawn_neon_text(
                section,
                NeonText {
                    marker: (),
                    value: "field school — séraphine",
                    font: display.clone(),
                    font_size: 20.0,
                    core: NEON_CYAN,
                    glow: NEON_GOLD,
                    glow_px: 1.0,
                    glow_inner_alpha: 0.55,
                    glow_outer_alpha: 0.25,
                    width: Val::Auto,
                    justify: Justify::Center,
                },
            );
            section
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|row| {
                    for id in level::SCENARIO_IDS {
                        let label = level::load_scenario(id)
                            .map(|def| def.title)
                            .unwrap_or_else(|| id.to_string());
                        row.spawn((
                            ScenarioButton(id),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                                border: BUTTON_BORDER,
                                border_radius: crate::ui::BUTTON_RADIUS,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.2, 0.28)),
                            styled_button(ButtonPalette::back()),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(label),
                                TextFont {
                                    font: body_medium.clone().into(),
                                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                    }
                });
            section.spawn((
                Text::new("Out-of-track field jobs — they never touch your track progress."),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(NEON_DIM),
            ));
        });
}

fn setup_select(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected: Res<SelectedCompanion>,
) {
    let display: Handle<Font> = asset_server.load(crate::fonts::DISPLAY);
    let body: Handle<Font> = asset_server.load(crate::fonts::BODY);
    let body_medium: Handle<Font> = asset_server.load(crate::fonts::BODY_MEDIUM);
    commands
        .spawn((
            SelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.12)),
        ))
        .with_children(|parent| {
            // Keeper title artwork as a dimmed backdrop: same title card,
            // one step deeper. Absolutely positioned out of the flex flow
            // so it paints behind the cards. `ArtBackdrop` cover-fits it
            // to the real window (the default contain fit is what left
            // the flat "black corners" at non-design aspects).
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
            // Darkens the artwork so the cards pop; translucent so the
            // night city still reads through. Dark enough that the
            // artwork's baked-in "LIGHT SHOW" title recedes behind the
            // header and cards instead of colliding with them.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.03, 0.03, 0.08, 0.85)),
            ));
            spawn_neon_text(
                parent,
                NeonText {
                    marker: (),
                    value: "choose your discipline",
                    font: display.clone(),
                    font_size: 26.0,
                    core: NEON_GOLD,
                    glow: NEON_CYAN,
                    glow_px: 1.0,
                    glow_inner_alpha: 0.55,
                    glow_outer_alpha: 0.25,
                    width: Val::Auto,
                    justify: Justify::Center,
                },
            );
            for companion in Companion::ALL {
                spawn_companion_card(parent, &asset_server, &display, &body, companion);
            }
            if field_school_visible(selected.0) {
                spawn_field_school_row(parent, &display, &body, &body_medium);
            }
            parent
                .spawn((
                    BackButton,
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

fn spawn_companion_card(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    display: &Handle<Font>,
    body: &Handle<Font>,
    companion: Companion,
) {
    let stem = companion.picker_stem();
    let frames: [Handle<Image>; SELECT_ANIM_FRAMES] =
        std::array::from_fn(|i| asset_server.load(format!("sprites/picker/{stem}_select_{i}.png")));
    let first_frame = frames[0].clone();
    parent
        .spawn((
            SelectButton(companion),
            SelectAnim {
                frames,
                index: 0,
                normalized: false,
            },
            // The glow outline starts at zero width; `highlight_select_cards`
            // raises it in the companion's accent color on hover/press.
            Outline {
                width: Val::Px(0.0),
                offset: Val::Px(3.0),
                color: companion.accent(),
            },
            Button,
            Node {
                width: Val::Px(620.0),
                // On windows narrower than the design width the card
                // yields to the window instead of clipping off-screen.
                max_width: Val::Percent(92.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.18, 0.92)),
        ))
        .with_children(|card| {
            // Silhouette: dark shape + discipline-colored outline + FX
            // (see `gen_picker.lua`); animated while the card is
            // highlighted, resting on frame 0 otherwise.
            card.spawn((
                SelectSilhouette,
                Node {
                    width: Val::Px(SILHOUETTE_DISPLAY.x),
                    height: Val::Px(SILHOUETTE_DISPLAY.y),
                    ..default()
                },
                ImageNode::new(first_frame),
            ));
            card.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|text| {
                // Name: cyan core, gold halo — the title card's answer pattern.
                spawn_neon_text(
                    text,
                    NeonText {
                        marker: (),
                        value: companion.display_name(),
                        font: display.clone(),
                        font_size: 24.0,
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
                    Text::new(companion.select_hook()),
                    TextFont {
                        font: body.clone().into(),
                        font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                        ..default()
                    },
                    TextColor(NEON_GOLD),
                ));
                // Trackless specialists (cheat-code unlocks) get a visible
                // badge: pressing the card stays on this screen by design,
                // and the badge says why instead of leaving silence.
                if companion.track_start_index().is_none() {
                    text.spawn((
                        Text::new("TRACK COMING SOON"),
                        TextFont {
                            font: body.clone().into(),
                            font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                            ..default()
                        },
                        TextColor(NEON_DIM),
                    ));
                }
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
    mut silhouettes: Query<&mut ImageNode, With<SelectSilhouette>>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    for (interaction, mut anim, children) in &mut cards {
        if *interaction != Interaction::Hovered && *interaction != Interaction::Pressed {
            // Snap back to frame 0 on unhover instead of freezing mid-cycle
            // (Finding 5).
            if anim.index != 0 {
                anim.index = 0;
                for child in children {
                    if let Ok(mut image) = silhouettes.get_mut(*child) {
                        image.image = anim.frames[0].clone();
                    }
                }
            }
            continue;
        }
        anim.index = (anim.index + 1) % SELECT_ANIM_FRAMES;
        for child in children {
            if let Ok(mut image) = silhouettes.get_mut(*child) {
                image.image = anim.frames[anim.index].clone();
            }
        }
    }
}

/// The drawn-content rectangle of one RGBA8 frame: the bounding box
/// of its non-transparent pixels, in image pixel coordinates (max
/// exclusive, matching `ImageNode::rect`). `None` when the buffer is
/// the wrong size for the claimed dimensions or nothing is drawn.
/// Pure over the pixel buffer so it is unit-testable without assets.
fn content_rect_rgba(data: &[u8], width: u32, height: u32) -> Option<Rect> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if data.len() < expected {
        return None;
    }
    let (mut min_x, mut min_y) = (width, height);
    let (mut max_x, mut max_y) = (0u32, 0u32);
    for y in 0..height {
        for x in 0..width {
            let alpha = data[((y * width + x) * 4 + 3) as usize];
            if alpha > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + 1);
                max_y = max_y.max(y + 1);
            }
        }
    }
    if max_x == 0 {
        return None;
    }
    Some(Rect::new(
        min_x as f32,
        min_y as f32,
        max_x as f32,
        max_y as f32,
    ))
}

/// The drawn-content rectangle of a loaded frame image. Only the
/// RGBA8 formats the PNG loader produces are measured; anything else
/// (or an image whose CPU-side data is gone) yields `None` and the
/// card simply keeps its placeholder box.
fn content_rect(image: &Image) -> Option<Rect> {
    use bevy::render::render_resource::TextureFormat;
    if !matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    ) {
        return None;
    }
    let size = image.size();
    content_rect_rgba(image.data.as_deref()?, size.x, size.y)
}

/// Display size for a silhouette cropped to `content`: every card's
/// content stands `SILHOUETTE_DISPLAY.y` tall, with the width the
/// content's own aspect ratio implies — consistent heights, no
/// squash, whatever canvas the art was exported on.
fn silhouette_display_size(content: Rect) -> Vec2 {
    let height = SILHOUETTE_DISPLAY.y;
    Vec2::new(height * content.width() / content.height(), height)
}

/// One-time per card: once all of a card's frames are resident,
/// compute the union of their content rectangles (the union, so the
/// animation's frame-to-frame motion is never cropped), set it as
/// the silhouette's `ImageNode::rect`, and size the node from the
/// content aspect. Cards whose art is missing or unreadable keep the
/// placeholder box and are retried each frame until it arrives.
fn normalize_select_silhouettes(
    images: Res<Assets<Image>>,
    mut cards: Query<(&mut SelectAnim, &Children)>,
    mut silhouettes: Query<(&mut ImageNode, &mut Node), With<SelectSilhouette>>,
) {
    for (mut anim, children) in &mut cards {
        if anim.normalized {
            continue;
        }
        let mut union: Option<Rect> = None;
        for frame in &anim.frames {
            let Some(rect) = images.get(frame).and_then(content_rect) else {
                union = None;
                break;
            };
            union = Some(match union {
                Some(acc) => Rect::new(
                    acc.min.x.min(rect.min.x),
                    acc.min.y.min(rect.min.y),
                    acc.max.x.max(rect.max.x),
                    acc.max.y.max(rect.max.y),
                ),
                None => rect,
            });
        }
        let Some(content) = union else {
            continue;
        };
        let size = silhouette_display_size(content);
        for child in children {
            if let Ok((mut image, mut node)) = silhouettes.get_mut(*child) {
                image.rect = Some(content);
                node.width = Val::Px(size.x);
                node.height = Val::Px(size.y);
            }
        }
        anim.normalized = true;
    }
}

/// Outline width target when a card is hovered/pressed (px).
const CARD_OUTLINE_PX: f32 = 3.0;
/// Exponential approach rate for the outline lerp: reaches ~95% of the
/// target in ~0.25s, matching the audit's ~12px/s guideline (Finding 5).
const OUTLINE_LERP_RATE: f32 = 12.0;

/// Eases each card's outline width toward its hover target every frame
/// instead of snapping 0<->3px on `Changed<Interaction>`, and fades the
/// accent color alpha in with the width (Finding 5).
fn highlight_select_cards(
    time: Res<Time>,
    mut cards: Query<(&Interaction, &SelectButton, &mut Outline)>,
) {
    // Frame-rate-independent exponential approach.
    let blend = 1.0 - (-OUTLINE_LERP_RATE * time.delta_secs()).exp();
    for (interaction, button, mut outline) in &mut cards {
        let target_px = match interaction {
            Interaction::Hovered | Interaction::Pressed => CARD_OUTLINE_PX,
            Interaction::None => 0.0,
        };
        let current_px = match outline.width {
            Val::Px(px) => px,
            _ => 0.0,
        };
        let new_px = current_px + (target_px - current_px) * blend;
        outline.width = Val::Px(new_px);
        outline.color = button
            .0
            .accent()
            .with_alpha((new_px / CARD_OUTLINE_PX).clamp(0.0, 1.0));
    }
}

/// A card press selects the companion *and* starts its level track: the
/// sprite/dialogue swap still happens in
/// `waifu::respawn_on_companion_change`, which reacts to the resource.
/// A companion without a built track stays on this screen. This is the
/// only gate into `Playing`, so `Results` can rely on a tracked companion.
fn handle_select_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &SelectButton), Changed<Interaction>>,
    mut selected: ResMut<SelectedCompanion>,
    mut index: ResMut<CurrentLevelIndex>,
    mut scenario: ResMut<CurrentScenarioId>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            let Some(start) = button.0.track_start_index() else {
                // No track to start: acknowledge the press audibly so it
                // isn't silent, but stay on the select screen.
                sfx.play(&mut commands, crate::audio::SfxKind::Click);
                continue;
            };
            sfx.play(&mut commands, crate::audio::SfxKind::Pick);
            selected.0 = button.0;
            index.0 = start;
            // Starting a track level ends any Field School detour: the
            // track index, not a stale scenario id, decides the level.
            scenario.0 = None;
            request.0 = Some(GameState::Playing);
        }
    }
}

/// A Field School press starts that scenario under Séraphine: the
/// scenario id (not the track index) decides which level loads, via
/// `level::load_current_level`. An id the registry doesn't know is
/// acknowledged with a click and otherwise ignored — the buttons are
/// spawned from `SCENARIO_IDS`, so this is unreachable in normal play.
fn handle_scenario_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &ScenarioButton), Changed<Interaction>>,
    mut selected: ResMut<SelectedCompanion>,
    mut scenario: ResMut<CurrentScenarioId>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            if level::load_scenario(button.0).is_none() {
                sfx.play(&mut commands, crate::audio::SfxKind::Click);
                continue;
            }
            sfx.play(&mut commands, crate::audio::SfxKind::Pick);
            selected.0 = Companion::Fiber;
            scenario.0 = Some(button.0.to_string());
            request.0 = Some(GameState::Playing);
        }
    }
}

fn handle_back_button(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<BackButton>)>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
            request.0 = Some(GameState::MainMenu);
        }
    }
}

fn teardown_select(mut commands: Commands, query: Query<Entity, With<SelectRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
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
        world.insert_resource(CurrentScenarioId::default());
        world.init_resource::<TransitionRequest>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world
    }

    #[test]
    fn field_school_row_shows_only_for_seraphine() {
        assert!(field_school_visible(Companion::Fiber));
        for other in [
            Companion::Coax,
            Companion::Mobile,
            Companion::Ethernet,
            Companion::Clara,
            Companion::Aino,
            Companion::Hikari,
            Companion::Lea,
        ] {
            assert!(!field_school_visible(other), "{other:?} shows no row");
        }
    }

    #[test]
    fn pressing_a_scenario_button_starts_that_scenario_under_seraphine() {
        let mut world = world_with_select_state();
        world.spawn((ScenarioButton("fj2"), Interaction::Pressed));

        world
            .run_system_once(handle_scenario_buttons)
            .expect("scenario system runs");

        assert_eq!(world.resource::<SelectedCompanion>().0, Companion::Fiber);
        assert_eq!(
            world.resource::<CurrentScenarioId>().0.as_deref(),
            Some("fj2")
        );
        // The track index is untouched: the scenario id loads the level.
        assert_eq!(world.resource::<CurrentLevelIndex>().0, 0);
        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Playing)
        ));
    }

    #[test]
    fn pressing_a_card_clears_a_stale_scenario() {
        let mut world = world_with_select_state();
        world.resource_mut::<CurrentScenarioId>().0 = Some("sp1".to_string());
        world.spawn((SelectButton(Companion::Coax), Interaction::Pressed));

        world
            .run_system_once(handle_select_buttons)
            .expect("select system runs");

        assert_eq!(world.resource::<SelectedCompanion>().0, Companion::Coax);
        assert_eq!(world.resource::<CurrentLevelIndex>().0, 10);
        assert_eq!(world.resource::<CurrentScenarioId>().0, None);
    }

    #[test]
    fn pressing_a_card_selects_the_companion_and_starts_its_track() {
        let mut world = world_with_select_state();
        world.spawn((SelectButton(Companion::Ethernet), Interaction::Pressed));

        world.run_system_once(handle_select_buttons);

        assert_eq!(world.resource::<SelectedCompanion>().0, Companion::Ethernet);
        assert_eq!(world.resource::<CurrentLevelIndex>().0, 30); // Ethernet track starts at 30 in 50-level layout
        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Playing)
        ));
    }

    #[test]
    fn pressing_leas_card_starts_her_quiz_track() {
        // Léa's study track is live (indices 70-79): pressing her card
        // selects her and jumps to the first quiz level.
        let mut world = world_with_select_state();
        world.spawn((SelectButton(Companion::Lea), Interaction::Pressed));

        world
            .run_system_once(handle_select_buttons)
            .expect("select system runs");

        assert_eq!(world.resource::<SelectedCompanion>().0, Companion::Lea);
        assert_eq!(world.resource::<CurrentLevelIndex>().0, 70);
        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::Playing)
        ));
    }

    #[test]
    fn pressing_back_returns_to_the_main_menu() {
        let mut world = world_with_select_state();
        world.spawn((BackButton, Interaction::Pressed));

        world.run_system_once(handle_back_button);

        assert!(matches!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::MainMenu)
        ));
    }

    #[test]
    fn hovering_a_card_raises_its_accent_outline() {
        use std::time::Duration;

        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_secs_f32(1.0));
        world.insert_resource(time);
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

        // The outline lerps toward 3px (not a snap): after a 1s delta it
        // has effectively converged, and the accent color faded in.
        let outline = world.get::<Outline>(card).expect("card keeps its outline");
        let width_px = match outline.width {
            Val::Px(px) => px,
            _ => panic!("expected px width"),
        };
        assert!((width_px - 3.0).abs() < 0.01, "width: {width_px}");
        assert!(outline.color.alpha() > 0.99);
    }

    #[test]
    fn unhovering_a_card_drops_its_outline() {
        use std::time::Duration;

        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_secs_f32(1.0));
        world.insert_resource(time);
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
        let width_px = match outline.width {
            Val::Px(px) => px,
            _ => panic!("expected px width"),
        };
        assert!(width_px < 0.01, "width: {width_px}");
        assert!(outline.color.alpha() < 0.01);
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
            .spawn((SelectSilhouette, ImageNode::new(Handle::default())))
            .id();
        let card = world
            .spawn((
                SelectButton(Companion::Fiber),
                SelectAnim {
                    frames: std::array::from_fn(|_| Handle::default()),
                    index: 0,
                    normalized: false,
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
    fn every_companion_starts_a_distinct_track() {
        let starts: Vec<Option<usize>> = Companion::ALL
            .iter()
            .map(|c| c.track_start_index())
            .collect();
        assert_eq!(starts, vec![Some(0), Some(10), Some(20), Some(30)]);
        // All eight playable companions run ten-level tracks in contiguous
        // registry blocks: Fiber 0-9, Coax 10-19, Mobile 20-29,
        // Ethernet 30-39, Clara 40-49, Aino 50-59, Hikari 60-69, Léa 70-79.
        for companion in Companion::ALL {
            assert_eq!(companion.track_len(), 10);
        }
        assert_eq!(Companion::Clara.track_start_index(), Some(40));
        assert_eq!(Companion::Clara.track_len(), 10);
        assert_eq!(Companion::Aino.track_start_index(), Some(50));
        assert_eq!(Companion::Aino.track_len(), 10);
        assert_eq!(Companion::Hikari.track_start_index(), Some(60));
        assert_eq!(Companion::Hikari.track_len(), 10);
        assert_eq!(Companion::Lea.track_start_index(), Some(70));
        assert_eq!(Companion::Lea.track_len(), 10);
    }

    /// A 4×4 RGBA buffer with the given opaque pixels set.
    fn rgba_with_opaque(width: u32, height: u32, opaque: &[(u32, u32)]) -> Vec<u8> {
        let mut data = vec![0u8; (width * height * 4) as usize];
        for &(x, y) in opaque {
            data[((y * width + x) * 4 + 3) as usize] = 255;
        }
        data
    }

    #[test]
    fn content_rect_bounds_the_drawn_pixels() {
        // One pixel at (3, 2): the rect is that pixel, max exclusive.
        let data = rgba_with_opaque(4, 4, &[(3, 2)]);
        assert_eq!(
            content_rect_rgba(&data, 4, 4),
            Some(Rect::new(3.0, 2.0, 4.0, 3.0))
        );
        // Content touching every edge crops to the whole canvas.
        let data = rgba_with_opaque(4, 4, &[(0, 0), (3, 0), (0, 3), (3, 3)]);
        assert_eq!(
            content_rect_rgba(&data, 4, 4),
            Some(Rect::new(0.0, 0.0, 4.0, 4.0))
        );
        // The shipped Séraphine pose: 44×50 of content at (10, 14).
        let mut pixels: Vec<(u32, u32)> = Vec::new();
        for y in 14..64 {
            for x in 10..54 {
                pixels.push((x, y));
            }
        }
        let data = rgba_with_opaque(64, 64, &pixels);
        assert_eq!(
            content_rect_rgba(&data, 64, 64),
            Some(Rect::new(10.0, 14.0, 54.0, 64.0))
        );
    }

    #[test]
    fn content_rect_rejects_empty_and_truncated_buffers() {
        // Fully transparent: nothing drawn, no crop.
        assert_eq!(content_rect_rgba(&vec![0u8; 4 * 4 * 4], 4, 4), None);
        // A buffer too small for the claimed dimensions is corrupt,
        // not a small image.
        assert_eq!(content_rect_rgba(&vec![255u8; 8], 4, 4), None);
        assert_eq!(content_rect_rgba(&[], 0, 0), None);
    }

    #[test]
    fn silhouette_display_size_equalizes_height_and_keeps_aspect() {
        // The two shipped content shapes (Séraphine 44×50, Lattice
        // 55×64) must stand the same height with undistorted widths.
        let short = silhouette_display_size(Rect::new(10.0, 14.0, 54.0, 64.0));
        let tall = silhouette_display_size(Rect::new(3.0, 0.0, 58.0, 64.0));
        assert_eq!(short.y, SILHOUETTE_DISPLAY.y);
        assert_eq!(tall.y, SILHOUETTE_DISPLAY.y);
        assert!((short.x - SILHOUETTE_DISPLAY.y * 44.0 / 50.0).abs() < 0.01);
        assert!((tall.x - SILHOUETTE_DISPLAY.y * 55.0 / 64.0).abs() < 0.01);
    }
}
