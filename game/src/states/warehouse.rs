//! The Warehouse screen: a shelf of real tools, two hosts who teach
//! them, a quiz that pays cores, and a shop that spends them. Data and
//! economy rules live in `crate::warehouse`; this module is the Bevy
//! side. One `WarehouseScreen` resource holds the mode (Shelf / Learn /
//! Quiz / Shop) and the whole UI is rebuilt from it when it changes: a
//! few dozen nodes, rebuilt only on a button press, which is simpler
//! than the per-text sync queries `states::quiz` needs.
//!
//! Every core or inventory change is written to disk immediately with
//! `save::save`, so a purchase is never lost to a crash before the next
//! change-detected sync.

use super::GameState;
use crate::anim::TransitionRequest;
use crate::audio::SfxKind;
use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::save::SaveData;
use crate::ui::neon::{spawn_neon_text, NeonText, NEON_CYAN, NEON_DIM, NEON_GOLD, NEON_INK};
use crate::ui::{styled_button, ButtonPalette, BUTTON_BORDER};
use crate::waifu::Cores;
use crate::warehouse::{self as wh, HostId, Loadout, PurchaseError, QuizQuestion, QuizReward};
use bevy::prelude::*;

pub struct WarehousePlugin;

impl Plugin for WarehousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Loadout>()
            .init_resource::<WarehouseScreen>()
            .add_systems(Startup, seed_loadout)
            .add_systems(OnEnter(GameState::Warehouse), reset_screen)
            .add_systems(
                Update,
                (handle_warehouse_buttons, rebuild_screen)
                    .chain()
                    .run_if(in_state(GameState::Warehouse)),
            )
            .add_systems(OnExit(GameState::Warehouse), teardown_warehouse);
    }
}

/// Portrait panel size. The panel is the fallback when the art is
/// missing, so it is sized for the portrait, not for the name.
const PORTRAIT_W: f32 = 176.0;
const PORTRAIT_H: f32 = 235.0;
/// Gap between the two hosts on the counter stage.
const STAGE_GAP: f32 = 18.0;
/// Height of the counter front painted over the hosts' lower third.
const COUNTER_H: f32 = 62.0;
/// How far the counter front overhangs the portrait pair, per side.
const COUNTER_OVERHANG: f32 = 10.0;
/// Design-width cap for full-row text (dialogue panels, questions,
/// shop cards). Panels take the full row width up to this cap, so a
/// window narrower than the cap shrinks them fluidly instead of the
/// fixed 660 px pushing the panel off both screen edges — the first
/// playtest's clipped dialogue ("ianca · Test bench").
const TEXT_W: f32 = 660.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Mode {
    #[default]
    Shelf,
    Learn,
    Quiz,
    Shop,
}

/// Host reaction to the last answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reaction {
    Correct,
    Wrong,
    /// A miss a spare remote forgave: not counted, answer not revealed,
    /// question still open.
    Forgiven,
}

/// One run through a tool's quiz: the `QuizProgress` shape (see
/// `states::quiz`) cut down to what the Warehouse scores. Pure, so the
/// whole flow is unit-testable without an App.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct WarehouseQuiz {
    /// Question on screen; equals the question count once finished.
    current: usize,
    correct: usize,
    /// Reaction to the current question's last answer.
    reaction: Option<Reaction>,
    /// A spare remote has forgiven a miss this run. One per run: more
    /// would let a full pack brute-force a pass.
    remote_used: bool,
}

impl WarehouseQuiz {
    fn is_finished(&self, total: usize) -> bool {
        self.current >= total
    }

    /// Answer the current question. Returns `None` when no answer is
    /// accepted: the quiz is finished, or a counted reaction is showing
    /// (the player must press Next). A miss is `Forgiven` when a remote
    /// is available and none was used this run; the caller consumes it.
    /// An out-of-range `choice` is simply wrong.
    fn answer(
        &mut self,
        questions: &[QuizQuestion],
        choice: usize,
        remote_available: bool,
    ) -> Option<Reaction> {
        if matches!(self.reaction, Some(Reaction::Correct | Reaction::Wrong)) {
            return None;
        }
        let question = questions.get(self.current)?;
        let reaction = if choice == question.correct_idx {
            self.correct += 1;
            Reaction::Correct
        } else if remote_available && !self.remote_used {
            self.remote_used = true;
            Reaction::Forgiven
        } else {
            Reaction::Wrong
        };
        self.reaction = Some(reaction);
        assert!(self.correct <= self.current + 1, "one point per question");
        Some(reaction)
    }

    /// Move past a counted answer. Returns true exactly once: on the
    /// press that finishes the quiz, when the caller pays out. No-op
    /// while the current question has no counted answer.
    fn next(&mut self, total: usize) -> bool {
        if !matches!(self.reaction, Some(Reaction::Correct | Reaction::Wrong)) {
            return false;
        }
        self.reaction = None;
        self.current += 1;
        self.is_finished(total)
    }
}

/// Everything the screen shows. Changing it rebuilds the UI.
#[derive(Resource, Debug, Default)]
struct WarehouseScreen {
    mode: Mode,
    /// Index into `warehouse::TOOLS`.
    tool: usize,
    learn_page: usize,
    quiz: WarehouseQuiz,
    /// Payout of the finished quiz run, for the summary.
    result: Option<QuizReward>,
    /// A host's reply to the last shop action.
    notice: Option<(HostId, String)>,
    /// Host reaction lines for this visit (haul inspection on entry),
    /// claimed from `PendingHaul` by `reset_screen`. The shelf renders
    /// these through the pill bar INSTEAD of the old greeting panels:
    /// hosts speak as a reaction to the visit's haul, never by default.
    enter_lines: Vec<(HostId, String)>,
}

impl WarehouseScreen {
    fn show(&mut self, mode: Mode) {
        self.mode = mode;
        self.notice = None;
        self.enter_lines.clear();
    }
}

#[derive(Component)]
struct WarehouseRoot;

/// Tags a host portrait panel (fallback panel + name, image on top).
#[derive(Component)]
struct HostPortrait;

/// What a Warehouse button does when pressed.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum WarehouseAction {
    Exit,
    Shelf,
    Shop,
    OpenTool(usize),
    LearnPrev,
    LearnNext,
    StartQuiz,
    Answer(usize),
    QuizNext,
    Buy(&'static str),
}

/// Record a finished quiz run in `save`: pay `quiz_payout` into
/// `cores` and mark a first pass. Returns the payout.
fn record_quiz_result(
    save: &mut SaveData,
    tool_id: &str,
    correct: usize,
    total: usize,
) -> QuizReward {
    let already_passed = wh::owns(&save.warehouse_quiz_passed, tool_id);
    let reward = wh::quiz_payout(correct, total, already_passed);
    save.cores = save.cores.saturating_add(reward.cores);
    if reward.first_pass {
        save.warehouse_quiz_passed.push(tool_id.to_owned());
    }
    reward
}

/// Buy `item_id` with `save.cores`. All-or-nothing: on `Err` the
/// save is untouched.
fn commit_purchase(save: &mut SaveData, item_id: &str) -> Result<(), PurchaseError> {
    let purchase = wh::purchase(item_id, save.cores, &save.owned_gear, &save.consumables)?;
    save.cores = purchase.balance;
    save.owned_gear = purchase.owned_gear;
    save.consumables = purchase.consumables;
    Ok(())
}

/// After `save` changed: mirror its balance into the runtime Cores,
/// recompute gear effects, and write it to disk now. A failed write is
/// logged, never a crash.
fn sync_after_change(save: &SaveData, cores: &mut Cores, loadout: &mut Loadout) {
    cores.0 = save.cores;
    *loadout = Loadout::from_save(save);
    if let Err(e) = crate::save::save(save) {
        warn!("failed to save Warehouse change: {}", e);
    }
}

/// Gear effects from the loaded save, so the first level after launch
/// sees owned gear. Without a save (headless tests) the default stands.
fn seed_loadout(save: Option<Res<SaveData>>, mut loadout: ResMut<Loadout>) {
    if let Some(save) = save {
        *loadout = Loadout::from_save(&save);
    }
}

fn reset_screen(
    mut screen: ResMut<WarehouseScreen>,
    pending_haul: Option<ResMut<crate::salvage::PendingHaul>>,
) {
    *screen = WarehouseScreen::default();
    let haul = pending_haul.and_then(|mut h| h.take());
    screen.enter_lines = wh::enter_reaction(haul.as_ref());
}

// Bevy systems take one parameter per resource they touch; this one
// handler owns every Warehouse button, so the lint is waived (as in
// `results::show_results`) rather than splitting it across systems.
#[allow(clippy::too_many_arguments)]
fn handle_warehouse_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &WarehouseAction), Changed<Interaction>>,
    mut screen: ResMut<WarehouseScreen>,
    mut cores: ResMut<Cores>,
    mut save: Option<ResMut<SaveData>>,
    mut loadout: ResMut<Loadout>,
    mut request: ResMut<TransitionRequest>,
    sfx: Res<crate::audio::Sfx>,
) {
    for (interaction, action) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let mut sound = SfxKind::Click;
        let tool = wh::TOOLS.get(screen.tool);
        match *action {
            WarehouseAction::Exit => request.0 = Some(GameState::MainMenu),
            WarehouseAction::Shelf => screen.show(Mode::Shelf),
            WarehouseAction::Shop => screen.show(Mode::Shop),
            WarehouseAction::OpenTool(i) => {
                if i < wh::TOOLS.len() {
                    screen.tool = i;
                    screen.learn_page = 0;
                    screen.show(Mode::Learn);
                }
            }
            WarehouseAction::LearnPrev => {
                screen.learn_page = screen.learn_page.saturating_sub(1);
            }
            WarehouseAction::LearnNext => {
                let pages = tool.map_or(0, |t| wh::learn_lines(t).len());
                screen.learn_page = (screen.learn_page + 1).min(pages.saturating_sub(1));
            }
            WarehouseAction::StartQuiz => {
                screen.quiz = WarehouseQuiz::default();
                screen.result = None;
                screen.show(Mode::Quiz);
            }
            WarehouseAction::Answer(choice) => {
                let Some(tool) = tool else { continue };
                let remote_available = save
                    .as_deref()
                    .is_some_and(|s| wh::count(&s.consumables, wh::SPARE_REMOTES) > 0);
                match screen.quiz.answer(tool.quiz, choice, remote_available) {
                    Some(Reaction::Forgiven) => {
                        if let Some(save) = save.as_deref_mut() {
                            save.cores = cores.0;
                            let used = wh::consume_one(&mut save.consumables, wh::SPARE_REMOTES);
                            assert!(used, "forgiven only with a remote in the pack");
                            sync_after_change(save, &mut cores, &mut loadout);
                        }
                    }
                    Some(Reaction::Wrong) => sound = SfxKind::Alarm,
                    Some(Reaction::Correct) | None => {}
                }
            }
            WarehouseAction::QuizNext => {
                let Some(tool) = tool else { continue };
                let total = tool.quiz.len();
                if screen.quiz.next(total) {
                    let correct = screen.quiz.correct;
                    let reward = match save.as_deref_mut() {
                        Some(save) => {
                            save.cores = cores.0;
                            let reward = record_quiz_result(save, tool.id, correct, total);
                            sync_after_change(save, &mut cores, &mut loadout);
                            reward
                        }
                        None => QuizReward {
                            cores: 0,
                            first_pass: false,
                        },
                    };
                    screen.result = Some(reward);
                    sound = if wh::quiz_passed(correct, total) {
                        SfxKind::Win
                    } else {
                        SfxKind::Lose
                    };
                }
            }
            WarehouseAction::Buy(item_id) => {
                let Some(save) = save.as_deref_mut() else {
                    continue;
                };
                save.cores = cores.0;
                screen.notice = Some(match commit_purchase(save, item_id) {
                    Ok(()) => {
                        sync_after_change(save, &mut cores, &mut loadout);
                        sound = SfxKind::Win;
                        let item = wh::item(item_id).expect("purchase succeeded, so it is listed");
                        (item.host, item.pitch.to_owned())
                    }
                    Err(e) => match e {
                        wh::PurchaseError::CannotAfford { price, balance } => (
                            HostId::Bianca,
                            format!(
                                "That one's {price} cores, and you're carrying {balance}.                                  Bring me dead parts — fried gear credits fast."
                            ),
                        ),
                        _ => (HostId::Tessa, e.to_string()),
                    },
                });
            }
        }
        sfx.play(&mut commands, sound);
    }
}

/// Fonts used across the screen, loaded once per rebuild.
struct UiFonts {
    title: Handle<Font>,
    body: Handle<Font>,
    medium: Handle<Font>,
}

fn text(font: &Handle<Font>, value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::Px(size * FONT_SIZE_ADJUST),
            ..default()
        },
        TextColor(color),
    )
}

/// Same as `text`, wrapped at `width` px. The node takes the full
/// width its parent offers, capped at `width`, so text wraps at
/// whichever is narrower — the cap on wide windows, the panel on
/// small ones — and never overflows its panel.
fn wrapped(
    font: &Handle<Font>,
    value: impl Into<String>,
    size: f32,
    color: Color,
    width: f32,
) -> impl Bundle {
    (
        text(font, value, size, color),
        Node {
            width: Val::Percent(100.0),
            max_width: Val::Px(width),
            ..default()
        },
    )
}

fn host_color(id: HostId) -> Color {
    match id {
        HostId::Bianca => NEON_CYAN,
        HostId::Tessa => NEON_GOLD,
    }
}

/// Fallback panel color per host, shown until (or instead of) the art.
fn panel_color(id: HostId) -> Color {
    match id {
        HostId::Bianca => Color::srgb(0.16, 0.12, 0.28),
        HostId::Tessa => Color::srgb(0.30, 0.20, 0.12),
    }
}

fn rebuild_screen(
    mut commands: Commands,
    screen: Res<WarehouseScreen>,
    cores: Res<Cores>,
    save: Option<Res<SaveData>>,
    roots: Query<Entity, With<WarehouseRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !screen.is_changed() && !cores.is_changed() {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let fallback = SaveData::default();
    let save = save.as_deref().unwrap_or(&fallback);
    let fonts = UiFonts {
        title: asset_server.load(DISPLAY_BOLD),
        body: asset_server.load(BODY),
        medium: asset_server.load(BODY_MEDIUM),
    };
    // No camera spawn: MainMenu's camera stays alive (menu teardown only
    // despawns `MenuRoot`), as on the Credits screen.
    commands
        .spawn((
            WarehouseRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(40.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.12)),
        ))
        .with_children(|root| {
            // Scene backdrop: the FST warehouse interior (generated
            // art with no hosts in it — Bianca and Tessa are composited
            // in front of it on the stage). A translucent scrim keeps
            // text readable over the art; the root's flat color above
            // is the pre-load fallback, and a missing file just leaves
            // the flat screen, as with the portraits.
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(asset_server.load("art/warehouse_backdrop.jpg")),
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.03, 0.07, 0.45)),
            ));
            spawn_neon_text(
                root,
                NeonText {
                    marker: (),
                    value: "THE WAREHOUSE",
                    font: fonts.title.clone(),
                    font_size: 36.0,
                    core: NEON_GOLD,
                    glow: NEON_CYAN,
                    glow_px: 1.0,
                    glow_inner_alpha: 0.55,
                    glow_outer_alpha: 0.25,
                    width: Val::Auto,
                    justify: Justify::Center,
                },
            );
            root.spawn(text(
                &fonts.medium,
                format!("Cores: {}", cores.0),
                18.0,
                NEON_GOLD,
            ));
            spawn_stage(root, &fonts, &asset_server);
            match screen.mode {
                Mode::Shelf => spawn_shelf(root, &fonts, &asset_server, save, &screen),
                Mode::Learn => spawn_learn(root, &fonts, &asset_server, &screen),
                Mode::Quiz => spawn_quiz(root, &fonts, &asset_server, &screen, save),
                Mode::Shop => spawn_shop(root, &fonts, &asset_server, &screen, cores.0, save),
            }
        });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    action: WarehouseAction,
    label: &str,
    face: Color,
    ink: Color,
) {
    parent
        .spawn((
            action,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                border: BUTTON_BORDER,
                border_radius: crate::ui::BUTTON_RADIUS,
                ..default()
            },
            BackgroundColor(face),
            styled_button(ButtonPalette::from_face(face)),
        ))
        .with_children(|btn| {
            btn.spawn(text(&fonts.medium, label, 16.0, ink));
        });
}

/// A greyed-out label where a button would be: no `Button`, so no
/// `Interaction`, so it cannot be pressed.
fn spawn_disabled(parent: &mut ChildSpawnerCommands, fonts: &UiFonts, label: &str) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                border: BUTTON_BORDER,
                border_radius: crate::ui::BUTTON_RADIUS,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.16)),
            BorderColor::all(Color::srgb(0.3, 0.32, 0.4)),
        ))
        .with_children(|btn| {
            btn.spawn(text(&fonts.body, label, 14.0, NEON_DIM));
        });
}

fn spawn_row(parent: &mut ChildSpawnerCommands, build: impl FnOnce(&mut ChildSpawnerCommands)) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(12.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(build);
}

/// Host portrait over a solid fallback panel. The panel always
/// renders; the image covers it once it loads. A missing file is
/// logged by the asset server and never draws, leaving the panel:
/// art lands separately, and its absence is not a crash. Names are
/// not on the panel — they sit on the counter front (see
/// `spawn_stage`).
fn spawn_portrait(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    id: HostId,
) {
    let host = wh::host(id);
    parent
        .spawn((
            HostPortrait,
            Node {
                width: Val::Px(PORTRAIT_W),
                height: Val::Px(PORTRAIT_H),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            BackgroundColor(panel_color(id)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode::new(asset_server.load(host.portrait)),
            ));
        });
}

/// The Emporium beat: Bianca and Tessa side by side behind the
/// service counter, in frame for every mode — dialogue included. The
/// counter front spawns after the portraits so it paints over their
/// lower thirds; a gold trim line runs along its top edge and each
/// host's name plate sits on the counter in front of her.
fn spawn_stage(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    asset_server: &AssetServer,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(STAGE_GAP),
            ..default()
        })
        .with_children(|stage| {
            spawn_portrait(stage, asset_server, HostId::Bianca);
            spawn_portrait(stage, asset_server, HostId::Tessa);
            stage
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-COUNTER_OVERHANG),
                        right: Val::Px(-COUNTER_OVERHANG),
                        bottom: Val::Px(0.0),
                        height: Val::Px(COUNTER_H),
                        border: UiRect::top(Val::Px(3.0)),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceAround,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.07, 0.06, 0.09)),
                    BorderColor::all(NEON_GOLD),
                ))
                .with_children(|counter| {
                    for id in [HostId::Bianca, HostId::Tessa] {
                        let host = wh::host(id);
                        counter.spawn(text(&fonts.medium, host.name, 15.0, host_color(id)));
                    }
                });
        });
}

/// A host speaking: a dialogue panel under the stage, where both
/// hosts stay in frame behind the counter. The speaker is identified
/// by the name line and the host-colored accent on the panel's left
/// edge — the Emporium arrangement: the characters hold the shot,
/// the box carries the words.
/// Resolve a host line to its universal pill presentation (speaker
/// face + name + talking text surface). Every host line in the
/// Warehouse goes through this — no bare-text speech.
fn pill_presentation(id: HostId, line: &str) -> crate::waifu::pill::PillPresentation {
    crate::waifu::pill::present(crate::waifu::pill::PillSpeaker::Host(id), line)
}

/// A host speaking: the universal pill as an in-flow bar — square
/// mugshot window at the left end carrying the speaker's close-up,
/// name + line to its right, rounded bar anchored in the content
/// column under the stage (both hosts stay in frame behind the
/// counter). Same presentation data as the in-level pill.
fn spawn_host_line(
    parent: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    asset_server: &AssetServer,
    id: HostId,
    line: &str,
) {
    let host = wh::host(id);
    let pres = pill_presentation(id, line);
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                width: Val::Percent(100.0),
                max_width: Val::Px(TEXT_W),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(18.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.07, 0.85)),
            BorderColor::all(host_color(id)),
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BorderColor::all(host_color(id)),
                ImageNode::new(asset_server.load(pres.face_path)),
            ));
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|col| {
                col.spawn(text(
                    &fonts.medium,
                    format!("{} · {}", pres.speaker_name, host.side),
                    16.0,
                    host_color(id),
                ));
                col.spawn(wrapped(
                    &fonts.body,
                    pres.text.clone(),
                    15.0,
                    Color::srgb(0.88, 0.88, 0.94),
                    TEXT_W - 110.0,
                ));
            });
        });
}
fn spawn_shelf(
    root: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    asset_server: &AssetServer,
    save: &SaveData,
    screen: &WarehouseScreen,
) {
    // Entry reaction only: the hosts speak to the haul the player
    // hauled in (or one short open-for-business line) — the old
    // always-on greeting panels are retired.
    for (id, line) in &screen.enter_lines {
        spawn_host_line(root, fonts, asset_server, *id, line);
    }
    root.spawn(text(&fonts.title, "THE SHELF", 20.0, NEON_CYAN));
    for (i, tool) in wh::TOOLS.iter().enumerate() {
        let passed = if wh::owns(&save.warehouse_quiz_passed, tool.id) {
            "  ✓"
        } else {
            ""
        };
        let label = format!("{} · {}{passed}", tool.name, tool.tagline);
        spawn_button(
            root,
            fonts,
            WarehouseAction::OpenTool(i),
            &label,
            Color::srgb(0.13, 0.15, 0.22),
            NEON_CYAN,
        );
    }
    spawn_row(root, |row| {
        spawn_button(
            row,
            fonts,
            WarehouseAction::Shop,
            "Shop",
            NEON_GOLD,
            NEON_INK,
        );
        spawn_button(
            row,
            fonts,
            WarehouseAction::Exit,
            "Exit",
            Color::srgb(0.2, 0.2, 0.28),
            Color::WHITE,
        );
    });
}

fn spawn_learn(
    root: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    asset_server: &AssetServer,
    screen: &WarehouseScreen,
) {
    let Some(tool) = wh::TOOLS.get(screen.tool) else {
        return;
    };
    let lines = wh::learn_lines(tool);
    let Some(line) = lines.get(screen.learn_page) else {
        return;
    };
    root.spawn(text(&fonts.title, tool.name, 22.0, Color::WHITE));
    root.spawn(text(&fonts.body, tool.maker, 14.0, NEON_DIM));
    root.spawn(text(
        &fonts.medium,
        format!(
            "{} ({}/{})",
            line.heading,
            screen.learn_page + 1,
            lines.len()
        ),
        16.0,
        host_color(line.host),
    ));
    spawn_host_line(root, fonts, asset_server, line.host, line.text);
    let slate = Color::srgb(0.2, 0.2, 0.28);
    spawn_row(root, |row| {
        if screen.learn_page > 0 {
            spawn_button(
                row,
                fonts,
                WarehouseAction::LearnPrev,
                "Back",
                slate,
                Color::WHITE,
            );
        }
        if screen.learn_page + 1 < lines.len() {
            spawn_button(
                row,
                fonts,
                WarehouseAction::LearnNext,
                "Next",
                NEON_CYAN,
                NEON_INK,
            );
        }
        spawn_button(
            row,
            fonts,
            WarehouseAction::StartQuiz,
            "Test me",
            NEON_GOLD,
            NEON_INK,
        );
        spawn_button(
            row,
            fonts,
            WarehouseAction::Shelf,
            "Shelf",
            slate,
            Color::WHITE,
        );
    });
}

/// The host and line for a finished quiz run.
fn result_line(reward: QuizReward, correct: usize, total: usize) -> (HostId, String) {
    if !wh::quiz_passed(correct, total) {
        return (
            HostId::Tessa,
            format!(
                "{correct} of {total}. You need {} of {} to pass, partner. Hit the manual and \
                 come on back.",
                wh::QUIZ_PASS_NUMERATOR,
                wh::QUIZ_PASS_DENOMINATOR
            ),
        );
    }
    let line = if reward.first_pass {
        format!(
            "{correct} of {total}. Passed. +{} cores, graded and in your file.",
            reward.cores
        )
    } else if reward.cores > 0 {
        format!("Perfect again. +{} cores. Show-off.", reward.cores)
    } else {
        "Passed again. Only a perfect replay pays; this one was practice.".to_owned()
    };
    (HostId::Bianca, line)
}

fn spawn_quiz(
    root: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    asset_server: &AssetServer,
    screen: &WarehouseScreen,
    save: &SaveData,
) {
    let Some(tool) = wh::TOOLS.get(screen.tool) else {
        return;
    };
    let total = tool.quiz.len();
    let quiz = &screen.quiz;
    let slate = Color::srgb(0.2, 0.2, 0.28);
    root.spawn(text(
        &fonts.title,
        format!("{}: TEST ME", tool.name),
        22.0,
        Color::WHITE,
    ));
    if quiz.is_finished(total) {
        let reward = screen.result.unwrap_or(QuizReward {
            cores: 0,
            first_pass: false,
        });
        let (host, line) = result_line(reward, quiz.correct, total);
        spawn_host_line(root, fonts, asset_server, host, &line);
        spawn_row(root, |row| {
            spawn_button(
                row,
                fonts,
                WarehouseAction::StartQuiz,
                "Retake",
                NEON_GOLD,
                NEON_INK,
            );
            spawn_button(
                row,
                fonts,
                WarehouseAction::Shelf,
                "Shelf",
                slate,
                Color::WHITE,
            );
        });
        return;
    }
    let Some(question) = tool.quiz.get(quiz.current) else {
        return;
    };
    root.spawn(text(
        &fonts.body,
        format!("Question {}/{total}", quiz.current + 1),
        14.0,
        NEON_DIM,
    ));
    root.spawn(wrapped(
        &fonts.medium,
        question.question,
        18.0,
        Color::WHITE,
        TEXT_W,
    ));
    match quiz.reaction {
        Some(Reaction::Correct) => {
            spawn_host_line(
                root,
                fonts,
                asset_server,
                HostId::Bianca,
                question.explain_correct,
            );
        }
        Some(Reaction::Wrong) => {
            spawn_host_line(
                root,
                fonts,
                asset_server,
                HostId::Tessa,
                question.explain_wrong,
            );
        }
        Some(Reaction::Forgiven) | None => {
            if quiz.reaction == Some(Reaction::Forgiven) {
                spawn_host_line(
                    root,
                    fonts,
                    asset_server,
                    HostId::Bianca,
                    "Spare remotes. That miss does not count. I did not see it. Try again.",
                );
            }
            for (i, choice) in question.choices.iter().enumerate() {
                let label = format!("{}. {choice}", (b'A' + i as u8) as char);
                spawn_button(
                    root,
                    fonts,
                    WarehouseAction::Answer(i),
                    &label,
                    Color::srgb(0.14, 0.12, 0.22),
                    Color::WHITE,
                );
            }
            let remotes = wh::count(&save.consumables, wh::SPARE_REMOTES);
            if remotes > 0 && !quiz.remote_used {
                root.spawn(text(
                    &fonts.body,
                    format!("Spare remotes: {remotes} (one miss forgiven per quiz)"),
                    13.0,
                    NEON_DIM,
                ));
            }
            return;
        }
    }
    let label = if quiz.current + 1 >= total {
        "Finish"
    } else {
        "Next"
    };
    spawn_row(root, |row| {
        spawn_button(
            row,
            fonts,
            WarehouseAction::QuizNext,
            label,
            NEON_CYAN,
            NEON_INK,
        );
    });
}

fn spawn_shop(
    root: &mut ChildSpawnerCommands,
    fonts: &UiFonts,
    asset_server: &AssetServer,
    screen: &WarehouseScreen,
    balance: u32,
    save: &SaveData,
) {
    let (host, line) = screen.notice.clone().unwrap_or((
        HostId::Tessa,
        "Gear's on the wall, prices on the tags. Cores only — dead parts in, good gear out. \
         No IOUs."
            .to_owned(),
    ));
    spawn_host_line(root, fonts, asset_server, host, &line);
    for item in wh::SHOP {
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            width: Val::Percent(100.0),
            max_width: Val::Px(TEXT_W),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        })
        .insert(BackgroundColor(Color::srgb(0.09, 0.09, 0.15)))
        .with_children(|card| {
            card.spawn(text(
                &fonts.medium,
                format!("{} · {} cores", item.name, item.price),
                17.0,
                host_color(item.host),
            ));
            let carried = match item.kind {
                wh::ItemKind::Permanent => String::new(),
                wh::ItemKind::Consumable => {
                    format!(" Carrying: {}.", wh::count(&save.consumables, item.id))
                }
            };
            card.spawn(wrapped(
                &fonts.body,
                format!("{}{carried}", item.blurb),
                14.0,
                Color::srgb(0.8, 0.8, 0.9),
                TEXT_W - 16.0,
            ));
            // The real purchase check decides the button, so what the
            // shop offers can never disagree with what it would sell.
            match wh::purchase(item.id, balance, &save.owned_gear, &save.consumables) {
                Ok(_) => spawn_button(
                    card,
                    fonts,
                    WarehouseAction::Buy(item.id),
                    &format!("Buy · {}", item.price),
                    NEON_GOLD,
                    NEON_INK,
                ),
                Err(PurchaseError::AlreadyOwned) => spawn_disabled(card, fonts, "Owned"),
                Err(e) => spawn_disabled(card, fonts, &e.to_string()),
            }
        });
    }
    spawn_row(root, |row| {
        let slate = Color::srgb(0.2, 0.2, 0.28);
        spawn_button(
            row,
            fonts,
            WarehouseAction::Shelf,
            "Shelf",
            slate,
            Color::WHITE,
        );
        spawn_button(
            row,
            fonts,
            WarehouseAction::Exit,
            "Exit",
            slate,
            Color::WHITE,
        );
    });
}

fn teardown_warehouse(mut commands: Commands, query: Query<Entity, With<WarehouseRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use bevy_ecs::system::RunSystemOnce;

    fn scout() -> &'static wh::ToolDef {
        wh::tool("scout_pro_3").expect("Scout Pro 3 is on the shelf")
    }

    fn correct(i: usize) -> usize {
        scout().quiz[i].correct_idx
    }

    fn wrong(i: usize) -> usize {
        (correct(i) + 1) % 4
    }

    /// Answer every question, getting the ones in `misses` wrong, with no
    /// remotes. Returns the run and how many `next` presses finished it.
    fn run_quiz(misses: &[usize]) -> (WarehouseQuiz, usize) {
        let questions = scout().quiz;
        let mut quiz = WarehouseQuiz::default();
        let mut finishes = 0;
        for i in 0..questions.len() {
            let choice = if misses.contains(&i) {
                wrong(i)
            } else {
                correct(i)
            };
            assert!(quiz.answer(questions, choice, false).is_some());
            if quiz.next(questions.len()) {
                finishes += 1;
            }
        }
        (quiz, finishes)
    }

    // ---- Quiz flow: validation ----

    #[test]
    fn perfect_run_scores_every_question_and_finishes_once() {
        let (quiz, finishes) = run_quiz(&[]);
        assert_eq!(quiz.correct, 8);
        assert!(quiz.is_finished(8));
        assert_eq!(finishes, 1);
    }

    #[test]
    fn one_miss_still_passes_two_misses_fail() {
        let (one, _) = run_quiz(&[3]);
        assert_eq!(one.correct, 7);
        assert!(wh::quiz_passed(one.correct, 8));
        let (two, _) = run_quiz(&[0, 7]);
        assert!(!wh::quiz_passed(two.correct, 8));
    }

    #[test]
    fn a_spare_remote_forgives_one_miss_without_counting_it() {
        let questions = scout().quiz;
        let mut quiz = WarehouseQuiz::default();
        assert_eq!(
            quiz.answer(questions, wrong(0), true),
            Some(Reaction::Forgiven)
        );
        // The question stays open: Next does nothing, a retry is allowed.
        assert!(!quiz.next(8));
        assert_eq!(quiz.current, 0);
        assert_eq!(
            quiz.answer(questions, correct(0), true),
            Some(Reaction::Correct)
        );
        assert_eq!(quiz.correct, 1);
    }

    #[test]
    fn first_pass_pays_fifteen_and_is_recorded() {
        let mut save = SaveData {
            cores: 5,
            ..SaveData::default()
        };
        let reward = record_quiz_result(&mut save, "scout_pro_3", 7, 8);
        assert_eq!(reward.cores, 15);
        assert_eq!(save.cores, 20);
        assert_eq!(save.warehouse_quiz_passed, vec!["scout_pro_3".to_owned()]);
    }

    #[test]
    fn replays_pay_only_when_perfect_and_never_re_record() {
        let mut save = SaveData::default();
        record_quiz_result(&mut save, "scout_pro_3", 8, 8);
        assert_eq!(record_quiz_result(&mut save, "scout_pro_3", 7, 8).cores, 0);
        assert_eq!(record_quiz_result(&mut save, "scout_pro_3", 8, 8).cores, 3);
        assert_eq!(save.cores, 18);
        assert_eq!(save.warehouse_quiz_passed.len(), 1);
    }

    #[test]
    fn commit_purchase_debits_and_stocks_the_save() {
        let mut save = SaveData {
            cores: 75,
            ..SaveData::default()
        };
        commit_purchase(&mut save, wh::HEADLAMP).unwrap();
        commit_purchase(&mut save, wh::FIELD_COFFEE).unwrap();
        assert_eq!(save.cores, 40);
        assert_eq!(save.owned_gear, vec![wh::HEADLAMP.to_owned()]);
        assert_eq!(save.consumables, vec![wh::FIELD_COFFEE.to_owned()]);
        assert!(Loadout::from_save(&save).hint_free);
    }

    #[test]
    fn pressing_exit_returns_to_the_main_menu() {
        let mut world = World::new();
        world.init_resource::<TransitionRequest>();
        world.init_resource::<WarehouseScreen>();
        world.init_resource::<Cores>();
        world.init_resource::<Loadout>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world.spawn((WarehouseAction::Exit, Interaction::Pressed));
        world.run_system_once(handle_warehouse_buttons).unwrap();
        assert_eq!(
            world.resource::<TransitionRequest>().0,
            Some(GameState::MainMenu)
        );
    }

    // ---- Adversarial ----

    #[test]
    fn answers_are_refused_while_a_reaction_shows_and_after_the_end() {
        let questions = scout().quiz;
        let mut quiz = WarehouseQuiz::default();
        quiz.answer(questions, wrong(0), false);
        // Re-answering the shown question cannot turn a miss into a point.
        assert_eq!(quiz.answer(questions, correct(0), false), None);
        assert_eq!(quiz.correct, 0);
        let (mut done, _) = run_quiz(&[]);
        assert_eq!(done.answer(questions, 0, true), None);
        assert!(!done.next(8), "finishing pays out once, not per press");
        assert_eq!(done.correct, 8);
    }

    #[test]
    fn out_of_range_choice_is_wrong_not_a_panic() {
        let questions = scout().quiz;
        let mut quiz = WarehouseQuiz::default();
        assert_eq!(
            quiz.answer(questions, usize::MAX, false),
            Some(Reaction::Wrong)
        );
        assert_eq!(quiz.correct, 0);
    }

    #[test]
    fn remotes_forgive_only_once_per_run() {
        let questions = scout().quiz;
        let mut quiz = WarehouseQuiz::default();
        assert_eq!(
            quiz.answer(questions, wrong(0), true),
            Some(Reaction::Forgiven)
        );
        // A full pack must not brute-force the answer.
        assert_eq!(
            quiz.answer(questions, wrong(0), true),
            Some(Reaction::Wrong)
        );
        assert!(!quiz.next(8));
        assert_eq!((quiz.current, quiz.correct), (1, 0));
    }

    #[test]
    fn no_remote_means_no_forgiveness_and_next_without_answer_is_inert() {
        let questions = scout().quiz;
        let mut quiz = WarehouseQuiz::default();
        assert!(!quiz.next(8));
        assert_eq!(quiz.current, 0);
        assert_eq!(
            quiz.answer(questions, wrong(0), false),
            Some(Reaction::Wrong)
        );
        assert!(!quiz.remote_used);
    }

    #[test]
    fn empty_quiz_is_finished_and_accepts_nothing() {
        let mut quiz = WarehouseQuiz::default();
        assert!(quiz.is_finished(0));
        assert_eq!(quiz.answer(&[], 0, true), None);
        assert!(!quiz.next(0));
        assert!(!wh::quiz_passed(quiz.correct, 0));
    }

    #[test]
    fn refused_purchases_leave_the_save_untouched() {
        let mut save = SaveData {
            cores: 79,
            owned_gear: vec![wh::HEADLAMP.to_owned()],
            ..SaveData::default()
        };
        let before = (
            save.cores,
            save.owned_gear.clone(),
            save.consumables.clone(),
        );
        for id in [wh::HEADLAMP, wh::GOLDEN_CRIMPER, "../save.json", ""] {
            assert!(commit_purchase(&mut save, id).is_err(), "{id:?}");
        }
        assert_eq!(
            (
                save.cores,
                save.owned_gear.clone(),
                save.consumables.clone()
            ),
            before
        );
    }

    #[test]
    fn buy_without_a_save_does_nothing() {
        let mut world = World::new();
        world.init_resource::<TransitionRequest>();
        world.init_resource::<WarehouseScreen>();
        world.insert_resource(Cores(500));
        world.init_resource::<Loadout>();
        world.insert_resource(crate::audio::Sfx::for_tests());
        world.spawn((WarehouseAction::Buy(wh::HEADLAMP), Interaction::Pressed));
        world.run_system_once(handle_warehouse_buttons).unwrap();
        assert_eq!(world.resource::<Cores>().0, 500);
        assert_eq!(*world.resource::<Loadout>(), Loadout::default());
    }

    #[test]
    fn warehouse_art_paths_exist_in_the_game_asset_tree() {
        // Regression for the first Warehouse playtest: the host
        // portraits sat in the repo-root `assets/` tree (which no
        // asset-root resolution ever reads) instead of the game's, and
        // the installed asset tree was never synced — every load
        // failed silently and only the fallback panels showed. The
        // paths the screen requests must exist in the tree the game
        // actually loads from (`game/assets`, the manifest dir).
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for host in wh::HOSTS {
            let path = assets.join(host.portrait);
            assert!(path.is_file(), "host portrait missing: {}", path.display());
        }
        let backdrop = assets.join("art/warehouse_backdrop.jpg");
        assert!(
            backdrop.is_file(),
            "warehouse backdrop missing: {}",
            backdrop.display()
        );
    }

    #[test]
    fn missing_portraits_fall_back_to_panels_without_panicking() {
        // No portrait art exists in this tree and no image loader is
        // registered, so every portrait load fails: the screen must still
        // build, with a panel per host.
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            StatesPlugin,
        ));
        app.init_asset::<Image>();
        app.init_asset::<Font>();
        app.init_state::<GameState>();
        app.init_resource::<TransitionRequest>();
        app.init_resource::<Cores>();
        app.insert_resource(crate::audio::Sfx::for_tests());
        app.add_plugins(WarehousePlugin);
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Warehouse);
        for _ in 0..5 {
            app.update();
        }
        let world = app.world_mut();
        let panels = world
            .query_filtered::<(), With<HostPortrait>>()
            .iter(world)
            .count();
        assert_eq!(panels, 2, "one fallback panel per host on the shelf");
    }
}
