//! Results screen: shows win/fail, the final link-budget ledger, and the
//! matching companion reaction line (from whichever `DialogueBank` is
//! currently loaded for `SelectedCompanion`), then offers to continue to
//! the next bundled level, retry the current one, or return to the menu.

use super::outage::ActiveOutage;
use super::playing::LiveGraph;
use super::{GameState, LevelOutcome};
use crate::anim::{Ease, TransitionRequest};
use crate::board;
use crate::level::{self, CurrentLevelIndex, LevelDef};
use crate::waifu::dialogue::DialogueBank;
use crate::waifu::{Companion, FavorPoints, SelectedCompanion};
use bevy::prelude::*;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinRingTimer::default());
        app.add_systems(
            OnEnter(GameState::Results),
            (board::teardown_board, show_results),
        )
        .add_systems(
            Update,
            (handle_result_buttons, animate_win_ring, animate_result_entrances)
                .run_if(in_state(GameState::Results)),
        )
        .add_systems(OnExit(GameState::Results), teardown_results);
    }
}

#[derive(Component)]
struct ResultsRoot;

/// Which follow-up action a results-screen button performs when pressed.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResultAction {
    ContinueNextLevel,
    RetrySameLevel,
    ReturnToMenu,
    /// At the end of a companion's track: back to the discipline picker
    /// so the player can start another medium's track.
    ReturnToSelect,
}

/// Frames in the win-ring one-shot (see `gen_board_art.lua`).
const WIN_RING_FRAMES: usize = 6;
/// Seconds per win-ring frame (matches the 0.12 s Aseprite frame timing).
const WIN_RING_FRAME_SECS: f32 = 0.12;

/// Tags the results-screen win ring; `index` advances through the strip
/// frames and the entity despawns after the last one (one-shot).
#[derive(Component)]
struct WinRing {
    frames: [Handle<Image>; WIN_RING_FRAMES],
    index: usize,
}

/// Paces the win-ring one-shot.
#[derive(Resource)]
struct WinRingTimer(Timer);

impl Default for WinRingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            WIN_RING_FRAME_SECS,
            TimerMode::Repeating,
        ))
    }
}

/// Stagger delays (seconds) for the results-screen entrance (Finding 6).
/// The full sequence plays out over ~0.65s.
const ENTRANCE_BANNER_DELAY: f32 = 0.0;
const ENTRANCE_TITLE_DELAY: f32 = 0.1;
const ENTRANCE_LEDGER_DELAY: f32 = 0.2;
const ENTRANCE_DIALOGUE_DELAY: f32 = 0.3;
const ENTRANCE_BUTTONS_DELAY: f32 = 0.4;
/// Banner pop: scale 0.8 -> 1.0 over 0.3s.
const BANNER_POP_SECS: f32 = 0.3;
const BANNER_POP_START_SCALE: f32 = 0.8;
/// Text fade: alpha 0 -> 1 over 0.2s.
const TEXT_FADE_SECS: f32 = 0.2;
/// Button row: slide up 20px + fade over 0.25s. The slide is done by
/// animating the top margin (12px -> 32px start), because UI layout
/// overwrites `Transform.translation` every frame.
const BUTTONS_SLIDE_SECS: f32 = 0.25;
const BUTTONS_SLIDE_PX: f32 = 20.0;
const BUTTONS_REST_MARGIN_TOP_PX: f32 = 12.0;

/// Staggered entrance animation marker for results-screen elements
/// (Finding 6). Each element waits `delay_secs`, then plays its
/// `kind` animation; the component is removed when done.
#[derive(Component)]
struct ResultEntrance {
    delay_secs: f32,
    elapsed_secs: f32,
    kind: EntranceKind,
}

/// The entrance animation flavor per element kind.
enum EntranceKind {
    /// Banner text: scale 0.8 -> 1.0 with BackOut.
    BannerPop,
    /// Body text: alpha 0 -> 1. Holds the original section colors so the
    /// fade can restore them exactly.
    FadeIn { original: Vec<Color> },
    /// Button row: slide up 20px + fade. Holds the button entities and
    /// their original background colors.
    SlideUp { buttons: Vec<(Entity, Color)> },
}

impl EntranceKind {
    /// Duration of this entrance animation in seconds.
    fn duration_secs(&self) -> f32 {
        match self {
            EntranceKind::BannerPop => BANNER_POP_SECS,
            EntranceKind::FadeIn { .. } => TEXT_FADE_SECS,
            EntranceKind::SlideUp { .. } => BUTTONS_SLIDE_SECS,
        }
    }
}

/// Plays the staggered results-screen entrance (Finding 6): banner pops
/// 0.8->1.0 (BackOut, 0.3s), then title/ledger/dialogue fade in (0.2s
/// each, staggered 0.1s), then the button row slides up 20px + fades
/// (0.25s).
fn animate_result_entrances(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        Option<&mut Transform>,
        Option<&mut Style>,
        Option<&mut Text>,
        &mut ResultEntrance,
    )>,
    mut button_bg: Query<&mut BackgroundColor>,
) {
    for (entity, mut transform, mut style, mut text, mut entrance) in &mut query {
        entrance.elapsed_secs += time.delta_seconds();
        if entrance.elapsed_secs < entrance.delay_secs {
            continue;
        }
        let local_t = entrance.elapsed_secs - entrance.delay_secs;
        let t = (local_t / entrance.kind.duration_secs()).clamp(0.0, 1.0);
        match &entrance.kind {
            EntranceKind::BannerPop => {
                let eased = Ease::BackOut.sample(t);
                if let Some(tr) = transform.as_mut() {
                    let scale = BANNER_POP_START_SCALE
                        + (1.0 - BANNER_POP_START_SCALE) * eased;
                    tr.scale = Vec3::splat(scale.max(0.01));
                }
            }
            EntranceKind::FadeIn { original } => {
                let eased = Ease::CubicOut.sample(t);
                if let Some(txt) = text.as_mut() {
                    for (section, color) in txt.sections.iter_mut().zip(original.iter()) {
                        section.style.color = color.with_alpha(eased * color.alpha());
                    }
                }
            }
            EntranceKind::SlideUp { buttons } => {
                let eased = Ease::CubicOut.sample(t);
                if let Some(st) = style.as_mut() {
                    st.margin.top = Val::Px(
                        BUTTONS_REST_MARGIN_TOP_PX + BUTTONS_SLIDE_PX * (1.0 - eased),
                    );
                }
                for (button_entity, color) in buttons.iter() {
                    if let Ok(mut bg) = button_bg.get_mut(*button_entity) {
                        bg.0 = color.with_alpha(eased * color.alpha());
                    }
                }
            }
        }
        if t >= 1.0 {
            // Snap to exact final values, then remove the marker.
            match &entrance.kind {
                EntranceKind::BannerPop => {
                    if let Some(tr) = transform.as_mut() {
                        tr.scale = Vec3::splat(1.0);
                    }
                }
                EntranceKind::FadeIn { original } => {
                    if let Some(txt) = text.as_mut() {
                        for (section, color) in txt.sections.iter_mut().zip(original.iter()) {
                            section.style.color = *color;
                        }
                    }
                }
                EntranceKind::SlideUp { buttons } => {
                    if let Some(st) = style.as_mut() {
                        st.margin.top = Val::Px(BUTTONS_REST_MARGIN_TOP_PX);
                    }
                    for (button_entity, color) in buttons.iter() {
                        if let Ok(mut bg) = button_bg.get_mut(*button_entity) {
                            bg.0 = *color;
                        }
                    }
                }
            }
            commands.entity(entity).remove::<ResultEntrance>();
        }
    }
}

/// Advances the win ring one frame per tick and despawns it after the
/// last frame.
fn animate_win_ring(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<WinRingTimer>,
    mut rings: Query<(Entity, Option<&Parent>, &mut WinRing, &mut UiImage, &mut Transform)>,
) {
    if rings.is_empty() {
        return;
    }
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }
    for (entity, parent, mut ring, mut image, mut transform) in &mut rings {
        ring.index += 1;
        if ring.index >= WIN_RING_FRAMES {
            // Detach from the results root first: a plain despawn leaves a
            // stale entry in the parent Children list, which teardown_results
            // would then trip over (B0003).
            if let Some(parent) = parent {
                commands.entity(parent.get()).remove_children(&[entity]);
            }
            commands.entity(entity).despawn();
        } else {
            image.texture = ring.frames[ring.index].clone();
            // Ease scale 0.5 -> 1.2 across the 0.72s lifetime (Finding 6):
            // the ring blooms outward as it plays. UiImage has no alpha in
            // Bevy 0.14, so scale carries the whole effect.
            let progress = ring.index as f32 / WIN_RING_FRAMES as f32;
            let scale = 0.5 + 0.7 * Ease::CubicOut.sample(progress);
            // UI layout preserves scale (it only overwrites translation).
            transform.scale = Vec3::splat(scale);
        }
    }
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
    selected: Res<SelectedCompanion>,
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

    let ledger_text = level.signal_ledger(
        &live.graph,
        live.tx_dbm,
        live.wavelength.0,
        active_outage.outage.as_ref(),
    );

    let (banner_text, banner_color) = if outcome.won {
        ("SERVICE RESTORED", Color::srgb(0.4, 0.9, 0.5))
    } else {
        ("OUTAGE TIMED OUT", Color::srgb(1.0, 0.302, 0.302)) // #ff4d4d
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

    let track_end = selected.0.track_start_index() + Companion::TRACK_LEN - 1;
    let has_next_level = index.0 < track_end;

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
            if outcome.won {
                // Expanding gold ring, played once: the win FX.
                let frames: [Handle<Image>; WIN_RING_FRAMES] = std::array::from_fn(|i| {
                    asset_server.load(format!("sprites/fx/win_ring_{i}.png"))
                });
                let first_frame = frames[0].clone();
                parent.spawn((
                    WinRing { frames, index: 0 },
                    ImageBundle {
                        style: Style {
                            width: Val::Px(256.0),
                            height: Val::Px(256.0),
                            ..default()
                        },
                        image: UiImage::new(first_frame),
                        ..default()
                    },
                ));
            }
            {
                let mut bundle = TextBundle::from_section(
                    banner_text,
                    TextStyle {
                        font: asset_server.load(crate::fonts::DISPLAY_BOLD),
                        font_size: 48.0,
                        color: banner_color,
                    },
                );
                // UI layout overwrites translation but preserves scale, so
                // the pop scale survives layout (Finding 6).
                bundle.transform = Transform::from_scale(Vec3::splat(BANNER_POP_START_SCALE));
                parent.spawn((
                    ResultEntrance {
                        delay_secs: ENTRANCE_BANNER_DELAY,
                        elapsed_secs: 0.0,
                        kind: EntranceKind::BannerPop,
                    },
                    bundle,
                ));
            }
            {
                let color = Color::srgb(0.8, 0.8, 0.9);
                parent.spawn((
                    ResultEntrance {
                        delay_secs: ENTRANCE_TITLE_DELAY,
                        elapsed_secs: 0.0,
                        kind: EntranceKind::FadeIn {
                            original: vec![color],
                        },
                    },
                    TextBundle::from_section(
                        format!("World {} — {}", level.world, level.title),
                        TextStyle {
                            font: asset_server.load(crate::fonts::DISPLAY),
                            font_size: 20.0,
                            color: color.with_alpha(0.0),
                        },
                    ),
                ));
            }
            {
                let color = Color::srgb(0.357, 0.753, 0.922); // #5bc0eb
                parent.spawn((
                    ResultEntrance {
                        delay_secs: ENTRANCE_LEDGER_DELAY,
                        elapsed_secs: 0.0,
                        kind: EntranceKind::FadeIn {
                            original: vec![color],
                        },
                    },
                    TextBundle::from_section(
                        ledger_text,
                        TextStyle {
                            font: asset_server.load(crate::fonts::DISPLAY),
                            font_size: 16.0,
                            color: color.with_alpha(0.0),
                        },
                    ),
                ));
            }
            {
                let color = Color::srgb(1.0, 0.435, 0.682); // #ff6fae
                parent.spawn((
                    ResultEntrance {
                        delay_secs: ENTRANCE_DIALOGUE_DELAY,
                        elapsed_secs: 0.0,
                        kind: EntranceKind::FadeIn {
                            original: vec![color],
                        },
                    },
                    TextBundle::from_section(
                        format!("\u{201c}{dialogue_line}\u{201d}"),
                        TextStyle {
                            font: asset_server.load(crate::fonts::BODY),
                            font_size: 16.0,
                            color: color.with_alpha(0.0),
                        },
                    ),
                ));
            }

            {
                // The row starts 20px lower (via top margin) with transparent
                // buttons; `animate_result_entrances` slides/fades it in.
                let mut row_cmds = parent.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        margin: UiRect::top(Val::Px(
                            BUTTONS_REST_MARGIN_TOP_PX + BUTTONS_SLIDE_PX,
                        )),
                        ..default()
                    },
                    ..default()
                });
                let mut button_fades: Vec<(Entity, Color)> = Vec::new();
                row_cmds.with_children(|row| {
                    // Buttons spawn transparent; the entrance system fades
                    // them to their real colors.
                    let mut push_button =
                        |action: ResultAction, label: &str, color: Color| {
                            let entity = spawn_result_button(
                                row,
                                &asset_server,
                                action,
                                label,
                                color.with_alpha(0.0),
                            );
                            button_fades.push((entity, color));
                        };
                    if outcome.won {
                        if has_next_level {
                            push_button(
                                ResultAction::ContinueNextLevel,
                                "Continue",
                                Color::srgb(0.9, 0.4, 0.6),
                            );
                        } else {
                            // Track complete — offer the other disciplines
                            // instead of a dead end.
                            push_button(
                                ResultAction::ReturnToSelect,
                                "Pick Another Track",
                                Color::srgb(0.9, 0.4, 0.6),
                            );
                        }
                    } else {
                        push_button(
                            ResultAction::RetrySameLevel,
                            "Retry",
                            Color::srgb(0.9, 0.4, 0.6),
                        );
                    }
                    push_button(
                        ResultAction::ReturnToMenu,
                        "Main Menu",
                        Color::srgb(0.2, 0.2, 0.28),
                    );
                });
                row_cmds.insert(ResultEntrance {
                    delay_secs: ENTRANCE_BUTTONS_DELAY,
                    elapsed_secs: 0.0,
                    kind: EntranceKind::SlideUp {
                        buttons: button_fades,
                    },
                });
            }
        });
}

fn spawn_result_button(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    action: ResultAction,
    label: &str,
    color: Color,
) -> Entity {
    let mut button = parent.spawn((
        action,
        ButtonBundle {
            style: Style {
                padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                ..default()
            },
            background_color: color.into(),
            ..default()
        },
    ));
    button.with_children(|btn| {
        btn.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font: asset_server.load(crate::fonts::BODY_MEDIUM),
                font_size: 20.0,
                color: Color::WHITE,
            },
        ));
    });
    button.id()
}

fn handle_result_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &ResultAction), Changed<Interaction>>,
    mut index: ResMut<CurrentLevelIndex>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        sfx.play(&mut commands, crate::audio::SfxKind::Click);
        match action {
            ResultAction::ContinueNextLevel => {
                index.0 = index
                    .0
                    .saturating_add(1)
                    .min(level::LEVEL_SOURCES.len() - 1);
                request.0 = Some(GameState::Playing);
            }
            ResultAction::RetrySameLevel => {
                request.0 = Some(GameState::Playing);
            }
            ResultAction::ReturnToMenu => {
                index.0 = 0;
                request.0 = Some(GameState::MainMenu);
            }
            ResultAction::ReturnToSelect => {
                request.0 = Some(GameState::CompanionSelect);
            }
        }
    }
}

fn teardown_results(mut commands: Commands, query: Query<Entity, With<ResultsRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
