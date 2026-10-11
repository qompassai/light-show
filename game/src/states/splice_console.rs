//! Splice-tray console for sp1 ("True to the Color").
//!
//! The tray is where the level's TIA-598 work order is executed: for
//! each beat the player picks a presented fiber (tube page, then
//! strand) and the drop pigtail it feeds, then verifies the light.
//! Scoring lives in [`crate::splice`]; this console only holds the
//! player's in-progress splices and renders the outcome — including
//! the fail ledger that names the swapped legs, so a wrong splice is
//! corrected at the tray instead of guessed at.
//!
//! The TIA-598 chart is one button away the whole time (a paged
//! panel over the full 144 pairs): the level tests fluency, not
//! memorization, and there is no timer on the tray to penalize
//! looking a pair up.
//!
//! The win gate follows the other consoles: beside the board's
//! light-path budget, `SpliceProgress::is_complete` must hold (see
//! `playing::check_win_condition`).

use bevy::prelude::*;

use crate::fonts::{BODY, BODY_MEDIUM, DISPLAY_BOLD, FONT_SIZE_ADJUST};
use crate::level::{LevelDef, SpliceWorkOrdersDef};
use crate::splice::{
    chart_page, describe_fiber, evaluate_beat, fiber_number, presented_tubes, BeatEvaluation,
    SpliceAttempt, FIBERS_PER_TUBE, TUBE_COUNT,
};
use crate::states::GameState;

/// The player's splice-tray work on the current level.
#[derive(Resource, Debug, Default)]
pub struct SpliceProgress {
    /// Level the tray state belongs to (set at console setup; the
    /// win gate only trusts progress recorded against its own id).
    pub level_id: String,
    /// Beat currently on the tray (index into the work orders).
    pub beat_index: usize,
    /// Beats verified true so far, in order.
    pub beats_passed: usize,
    /// Splices made on the current beat.
    pub attempts: Vec<SpliceAttempt>,
    /// Tube page the strand row shows.
    pub selected_tube: u32,
    /// Fiber picked and waiting for a drop (if any).
    pub selected_fiber: Option<u32>,
    /// Whether the TIA-598 reference panel is open.
    pub chart_open: bool,
    /// Tube page the reference panel shows.
    pub chart_tube: u32,
    /// The most recent verification of the current beat.
    pub last_evaluation: Option<BeatEvaluation>,
    /// Lifetime verify presses on this level (telemetry).
    pub verify_attempts: u32,
}

impl SpliceProgress {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Start the tray for a level: fresh progress bound to its id,
    /// opened on the first beat's first presented tube.
    pub fn begin(&mut self, level_id: &str, orders: &SpliceWorkOrdersDef) {
        self.reset();
        self.level_id = level_id.to_string();
        if let Some(beat) = orders.beats.first() {
            self.selected_tube = presented_tubes(orders, beat).first().copied().unwrap_or(1);
            self.chart_tube = self.selected_tube;
        }
    }

    /// The gate the win check reads: every beat of this level's
    /// work orders verified true, recorded against this level.
    pub fn is_complete(&self, level_id: &str, orders: &SpliceWorkOrdersDef) -> bool {
        self.level_id == level_id && self.beats_passed >= orders.beats.len()
    }

    /// Pick a tube page. Only tubes the current beat presents can
    /// be opened; returns whether the page changed.
    pub fn select_tube(&mut self, orders: &SpliceWorkOrdersDef, tube: u32) -> bool {
        let Some(beat) = orders.beats.get(self.beat_index) else {
            return false;
        };
        if !presented_tubes(orders, beat).contains(&tube) || tube == self.selected_tube {
            return false;
        }
        self.selected_tube = tube;
        true
    }

    /// Pick a strand on the current tube page. Picking a fiber that
    /// is already spliced pulls that splice back off the tray.
    /// Returns whether anything changed.
    pub fn select_strand(&mut self, strand_number: u32) -> bool {
        let Some(fiber) = fiber_number(self.selected_tube, strand_number) else {
            return false;
        };
        if let Some(position) = self.attempts.iter().position(|a| a.fiber == fiber) {
            self.attempts.remove(position);
            if self.selected_fiber == Some(fiber) {
                self.selected_fiber = None;
            }
            return true;
        }
        if self.selected_fiber == Some(fiber) {
            self.selected_fiber = None;
            return true;
        }
        self.selected_fiber = Some(fiber);
        true
    }

    /// Splice the picked fiber to `drop` (or, with nothing picked,
    /// pull the splice feeding `drop` back off). One fiber has one
    /// home and one pigtail takes one fiber: picking a new home for
    /// a fiber moves it, and feeding an already-fed drop replaces
    /// its feed. Picking the drop a picked fiber already feeds
    /// toggles that splice off. Returns whether anything changed.
    pub fn choose_drop(&mut self, drop: &str) -> bool {
        match self.selected_fiber.take() {
            Some(fiber) => {
                if self
                    .attempts
                    .iter()
                    .any(|a| a.fiber == fiber && a.drop == drop)
                {
                    self.attempts.retain(|a| a.fiber != fiber);
                    return true;
                }
                self.attempts.retain(|a| a.fiber != fiber && a.drop != drop);
                self.attempts.push(SpliceAttempt {
                    fiber,
                    drop: drop.to_string(),
                });
                true
            }
            None => {
                let before = self.attempts.len();
                self.attempts.retain(|a| a.drop != drop);
                self.attempts.len() != before
            }
        }
    }

    /// Pull every splice of the current beat back off the tray.
    pub fn clear_beat(&mut self) -> bool {
        if self.attempts.is_empty() {
            return false;
        }
        self.attempts.clear();
        self.selected_fiber = None;
        true
    }

    /// Verify the current beat's light. On a pass the beat is
    /// banked and the tray advances (splices cleared, next beat's
    /// first tube opened); on a fail the splices stay on the tray
    /// and the ledger in `last_evaluation` names what is wrong.
    /// Returns `None` when no beat remains.
    pub fn verify(&mut self, orders: &SpliceWorkOrdersDef) -> Option<bool> {
        let beat = orders.beats.get(self.beat_index)?;
        let evaluation = evaluate_beat(orders, beat, &self.attempts);
        let passed = evaluation.passed;
        self.last_evaluation = Some(evaluation);
        self.verify_attempts += 1;
        if passed {
            self.beats_passed += 1;
            self.beat_index += 1;
            self.attempts.clear();
            self.selected_fiber = None;
            if let Some(next) = orders.beats.get(self.beat_index) {
                self.selected_tube = presented_tubes(orders, next).first().copied().unwrap_or(1);
            }
        }
        Some(passed)
    }

    /// Open or close the reference panel.
    pub fn toggle_chart(&mut self) -> bool {
        self.chart_open = !self.chart_open;
        true
    }

    /// Page the reference panel by whole tubes, clamped to the chart.
    pub fn page_chart(&mut self, delta: i32) -> bool {
        let next = (i64::from(self.chart_tube) + i64::from(delta)).clamp(1, i64::from(TUBE_COUNT));
        let next = u32::try_from(next).unwrap_or(1);
        if next == self.chart_tube {
            return false;
        }
        self.chart_tube = next;
        true
    }
}

/// Marker on the console root node (for teardown).
#[derive(Component)]
struct SpliceConsoleRoot;

/// Marker on the reference-panel root node (for teardown).
#[derive(Component)]
struct SpliceChartRoot;

/// Marker on the console's beat header line.
#[derive(Component)]
struct SpliceBeatLine;

/// Marker on the console's teaching/presented line.
#[derive(Component)]
struct SpliceTeachingLine;

/// Marker on the console's work-order line.
#[derive(Component)]
struct SpliceOrderLine;

/// Marker on the console's selection/splices line.
#[derive(Component)]
struct SpliceTrayLine;

/// Marker on the console's ledger line (last verification, or the
/// standing win rule before the first one).
#[derive(Component)]
struct SpliceLedgerLine;

/// Marker on a strand button's label text; value is the strand's
/// slot (0-based) on the current tube page.
#[derive(Component)]
struct SpliceStrandText(usize);

/// Marker on a drop button's label text; value is the drop's slot
/// (0-based) in the current beat's work order.
#[derive(Component)]
struct SpliceDropText(usize);

/// Marker on the reference panel's page text.
#[derive(Component)]
struct SpliceChartText;

/// What a splice console button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceAction {
    ChartClose,
    ChartNext,
    ChartPrev,
    ChartToggle,
    ClearBeat,
    Drop(usize),
    Strand(u32),
    Tube(u32),
    Verify,
}

/// Marker on each console button; carries its action.
#[derive(Component)]
pub struct SpliceButton(pub SpliceAction);

pub struct SpliceConsolePlugin;

impl Plugin for SpliceConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpliceProgress>()
            .add_systems(
                Update,
                (handle_splice_buttons, sync_splice_console).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_splice_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_splice_console);
    }
}

/// Spawn the console when the level carries splice work orders.
pub(crate) fn setup_splice_console(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<SpliceProgress>,
    asset_server: Res<AssetServer>,
) {
    let Some(orders) = &level.splice_work_orders else {
        progress.reset();
        return;
    };
    progress.begin(&level.id, orders);
    let display: Handle<Font> = asset_server.load(DISPLAY_BOLD);
    let body: Handle<Font> = asset_server.load(BODY);
    let body_medium: Handle<Font> = asset_server.load(BODY_MEDIUM);

    commands
        .spawn((
            SpliceConsoleRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                right: Val::Px(12.0),
                width: Val::Px(560.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.14, 0.95)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("SPLICE TRAY — TIA-598 WORK ORDER"),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(15.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.65, 0.0)),
            ));
            root.spawn((
                SpliceBeatLine,
                Text::new(String::new()),
                TextFont {
                    font: body_medium.clone().into(),
                    font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.85, 1.0)),
            ));
            root.spawn((
                SpliceTeachingLine,
                Text::new(String::new()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn((
                SpliceOrderLine,
                Text::new(String::new()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.95, 0.85)),
            ));
            root.spawn(button_row()).with_children(|row| {
                for tube in 1..=TUBE_COUNT {
                    let color = orders
                        .color_sequence
                        .get(usize::try_from(tube - 1).unwrap_or(0))
                        .map(String::as_str)
                        .unwrap_or("?");
                    spawn_splice_button(
                        row,
                        &body_medium,
                        SpliceAction::Tube(tube),
                        &format!("T{tube} {color}"),
                    );
                }
            });
            root.spawn(button_row()).with_children(|row| {
                for slot in 0..FIBERS_PER_TUBE {
                    spawn_splice_button_with_text_marker(
                        row,
                        &body_medium,
                        SpliceAction::Strand(slot + 1),
                        SpliceStrandText(usize::try_from(slot).unwrap_or(0)),
                    );
                }
            });
            root.spawn((
                SpliceTrayLine,
                Text::new(String::new()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn(button_row()).with_children(|row| {
                for slot in 0..FIBERS_PER_TUBE {
                    spawn_splice_button_with_text_marker(
                        row,
                        &body_medium,
                        SpliceAction::Drop(usize::try_from(slot).unwrap_or(0)),
                        SpliceDropText(usize::try_from(slot).unwrap_or(0)),
                    );
                }
            });
            root.spawn(button_row()).with_children(|row| {
                spawn_splice_button(row, &body_medium, SpliceAction::Verify, "Verify light");
                spawn_splice_button(row, &body_medium, SpliceAction::ClearBeat, "Clear beat");
                spawn_splice_button(
                    row,
                    &body_medium,
                    SpliceAction::ChartToggle,
                    "TIA-598 chart",
                );
            });
            root.spawn((
                SpliceLedgerLine,
                Text::new(String::new()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(12.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.6)),
            ));
        });

    // The reference panel: the full chart, paged by tube, over the
    // game until closed. Hidden until the player opens it.
    commands
        .spawn((
            SpliceChartRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(70.0),
                left: Val::Px(24.0),
                width: Val::Px(430.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.06, 0.12, 0.97)),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("TIA-598 COLOR CHART — fiber = (tube - 1) x 12 + strand"),
                TextFont {
                    font: display.clone().into(),
                    font_size: FontSize::Px(14.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.65, 0.0)),
            ));
            root.spawn((
                SpliceChartText,
                Text::new(String::new()),
                TextFont {
                    font: body.clone().into(),
                    font_size: FontSize::Px(13.0 * FONT_SIZE_ADJUST),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            root.spawn(button_row()).with_children(|row| {
                spawn_splice_button(row, &body_medium, SpliceAction::ChartPrev, "< Tube");
                spawn_splice_button(row, &body_medium, SpliceAction::ChartNext, "Tube >");
                spawn_splice_button(row, &body_medium, SpliceAction::ChartClose, "Close chart");
            });
        });
}

fn button_row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(4.0),
        flex_wrap: FlexWrap::Wrap,
        row_gap: Val::Px(4.0),
        ..default()
    }
}

fn spawn_splice_button(
    row: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    action: SpliceAction,
    label: &str,
) {
    row.spawn((
        SpliceButton(action),
        Button,
        Node {
            padding: UiRect::axes(Val::Px(7.0), Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.12, 0.18, 0.30)),
    ))
    .with_children(|btn| {
        btn.spawn((
            Text::new(label.to_string()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(11.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

/// A button whose label the sync system rewrites (strand and drop
/// buttons: their meaning changes with the tube page and the beat).
fn spawn_splice_button_with_text_marker<M: Component>(
    row: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    action: SpliceAction,
    marker: M,
) {
    row.spawn((
        SpliceButton(action),
        Button,
        Node {
            padding: UiRect::axes(Val::Px(7.0), Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.12, 0.18, 0.30)),
    ))
    .with_children(|btn| {
        btn.spawn((
            marker,
            Text::new(String::new()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(11.0 * FONT_SIZE_ADJUST),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

fn handle_splice_buttons(
    mut commands: Commands,
    level: Res<LevelDef>,
    mut progress: ResMut<SpliceProgress>,
    buttons: Query<
        (&Interaction, &SpliceButton),
        (
            Changed<Interaction>,
            Without<crate::states::api_console::ApiButton>,
            Without<crate::states::triage_console::TriageButton>,
            Without<crate::states::quiz::QuizChoice>,
            Without<crate::states::identification::IdentificationButton>,
            Without<crate::states::jumper::JumperButton>,
        ),
    >,
    sfx: Res<crate::audio::Sfx>,
) {
    let Some(orders) = &level.splice_work_orders else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let changed = match button.0 {
            SpliceAction::ChartClose => {
                if progress.chart_open {
                    progress.toggle_chart()
                } else {
                    false
                }
            }
            SpliceAction::ChartNext => progress.page_chart(1),
            SpliceAction::ChartPrev => progress.page_chart(-1),
            SpliceAction::ChartToggle => progress.toggle_chart(),
            SpliceAction::ClearBeat => progress.clear_beat(),
            SpliceAction::Drop(slot) => match orders
                .beats
                .get(progress.beat_index)
                .and_then(|beat| beat.work_order.get(slot))
            {
                Some(entry) => {
                    let drop = entry.drop.clone();
                    progress.choose_drop(&drop)
                }
                None => false,
            },
            SpliceAction::Strand(strand) => progress.select_strand(strand),
            SpliceAction::Tube(tube) => progress.select_tube(orders, tube),
            SpliceAction::Verify => progress.verify(orders).unwrap_or(false),
        };
        if changed {
            sfx.play(&mut commands, crate::audio::SfxKind::Click);
        } else {
            sfx.play(&mut commands, crate::audio::SfxKind::Alarm);
        }
    }
}

/// The beat header: position, name, and the tray's standing state.
pub fn beat_line(orders: &SpliceWorkOrdersDef, progress: &SpliceProgress) -> String {
    match orders.beats.get(progress.beat_index) {
        Some(beat) => format!(
            "Beat {} of {} — {}",
            beat.beat,
            orders.beats.len(),
            beat.name
        ),
        None => format!(
            "All {} beats verified — the tray is true. Finish the light path to close the ticket.",
            orders.beats.len()
        ),
    }
}

/// The work-order line: every ordered drop and the fiber that must
/// light it (the designated spare stands in for a damaged fiber).
pub fn order_line(orders: &SpliceWorkOrdersDef, progress: &SpliceProgress) -> String {
    let Some(beat) = orders.beats.get(progress.beat_index) else {
        return String::new();
    };
    let mut parts: Vec<String> = beat
        .work_order
        .iter()
        .map(|entry| {
            let (expected, damaged) = crate::splice::expected_fiber(beat, entry.fiber);
            match damaged {
                Some(_) => format!(
                    "{} <- {} (spare for damaged {})",
                    entry.drop,
                    describe_fiber(orders, expected),
                    describe_fiber(orders, entry.fiber)
                ),
                None => format!("{} <- {}", entry.drop, describe_fiber(orders, expected)),
            }
        })
        .collect();
    for damage in &beat.damage_beats {
        parts.push(format!(
            "EXCLUDE {} — {} Splice {} instead.",
            describe_fiber(orders, damage.fiber),
            damage.condition,
            describe_fiber(orders, damage.spare.fiber)
        ));
    }
    parts.join("\n")
}

/// The tray line: the current selection and the splices on the tray.
pub fn tray_line(orders: &SpliceWorkOrdersDef, progress: &SpliceProgress) -> String {
    let selected = match progress.selected_fiber {
        Some(fiber) => format!(
            "Picked: {} — now pick its drop.",
            describe_fiber(orders, fiber)
        ),
        None => {
            "Pick a fiber, then its drop. Picking a spliced fiber or a fed drop pulls that splice."
                .to_string()
        }
    };
    if progress.attempts.is_empty() {
        return format!("{selected}\nTray: no splices yet.");
    }
    let splices: Vec<String> = progress
        .attempts
        .iter()
        .map(|a| format!("{} -> {}", describe_fiber(orders, a.fiber), a.drop))
        .collect();
    format!("{selected}\nTray: {}", splices.join("; "))
}

/// Rewrite every dynamic label from the current tray state. Runs
/// when the progress changes (setup counts: `begin` marks it).
#[allow(clippy::too_many_arguments)]
fn sync_splice_console(
    level: Res<LevelDef>,
    progress: Res<SpliceProgress>,
    mut beat_text: Query<
        &mut Text,
        (
            With<SpliceBeatLine>,
            Without<SpliceTeachingLine>,
            Without<SpliceOrderLine>,
            Without<SpliceTrayLine>,
            Without<SpliceLedgerLine>,
            Without<SpliceChartText>,
            Without<SpliceStrandText>,
            Without<SpliceDropText>,
        ),
    >,
    mut teaching_text: Query<
        &mut Text,
        (
            With<SpliceTeachingLine>,
            Without<SpliceBeatLine>,
            Without<SpliceOrderLine>,
            Without<SpliceTrayLine>,
            Without<SpliceLedgerLine>,
            Without<SpliceChartText>,
            Without<SpliceStrandText>,
            Without<SpliceDropText>,
        ),
    >,
    mut order_text: Query<
        &mut Text,
        (
            With<SpliceOrderLine>,
            Without<SpliceBeatLine>,
            Without<SpliceTeachingLine>,
            Without<SpliceTrayLine>,
            Without<SpliceLedgerLine>,
            Without<SpliceChartText>,
            Without<SpliceStrandText>,
            Without<SpliceDropText>,
        ),
    >,
    mut tray_text: Query<
        &mut Text,
        (
            With<SpliceTrayLine>,
            Without<SpliceBeatLine>,
            Without<SpliceTeachingLine>,
            Without<SpliceOrderLine>,
            Without<SpliceLedgerLine>,
            Without<SpliceChartText>,
            Without<SpliceStrandText>,
            Without<SpliceDropText>,
        ),
    >,
    mut ledger_text: Query<
        &mut Text,
        (
            With<SpliceLedgerLine>,
            Without<SpliceBeatLine>,
            Without<SpliceTeachingLine>,
            Without<SpliceOrderLine>,
            Without<SpliceTrayLine>,
            Without<SpliceChartText>,
            Without<SpliceStrandText>,
            Without<SpliceDropText>,
        ),
    >,
    mut chart_text: Query<
        &mut Text,
        (
            With<SpliceChartText>,
            Without<SpliceBeatLine>,
            Without<SpliceTeachingLine>,
            Without<SpliceOrderLine>,
            Without<SpliceTrayLine>,
            Without<SpliceLedgerLine>,
            Without<SpliceStrandText>,
            Without<SpliceDropText>,
        ),
    >,
    mut strand_labels: Query<(&SpliceStrandText, &mut Text), Without<SpliceDropText>>,
    mut drop_labels: Query<(&SpliceDropText, &mut Text), Without<SpliceStrandText>>,
    mut buttons: Query<(&SpliceButton, &mut BackgroundColor, &mut Visibility)>,
    mut chart_root: Query<&mut Visibility, (With<SpliceChartRoot>, Without<SpliceButton>)>,
) {
    let Some(orders) = &level.splice_work_orders else {
        return;
    };
    if !progress.is_changed() {
        return;
    }
    let beat = orders.beats.get(progress.beat_index);
    for mut text in &mut beat_text {
        **text = beat_line(orders, &progress);
    }
    for mut text in &mut teaching_text {
        **text = match beat {
            Some(beat) => format!("{}\n{}", beat.teaching, beat.presented),
            None => String::new(),
        };
    }
    for mut text in &mut order_text {
        **text = order_line(orders, &progress);
    }
    for mut text in &mut tray_text {
        **text = tray_line(orders, &progress);
    }
    for mut text in &mut ledger_text {
        **text = match (&progress.last_evaluation, beat) {
            (Some(evaluation), _) => evaluation.ledger.join("\n"),
            (None, Some(beat)) => beat.win_rule.clone(),
            (None, None) => String::new(),
        };
    }
    for mut text in &mut chart_text {
        let tube_color = orders
            .color_sequence
            .get(usize::try_from(progress.chart_tube - 1).unwrap_or(0))
            .map(String::as_str)
            .unwrap_or("?");
        let mut lines = vec![format!(
            "Tube {} ({}) — fibers {}-{}",
            progress.chart_tube,
            tube_color,
            (progress.chart_tube - 1) * FIBERS_PER_TUBE + 1,
            progress.chart_tube * FIBERS_PER_TUBE
        )];
        lines.extend(chart_page(orders, progress.chart_tube));
        **text = lines.join("\n");
    }
    for (marker, mut text) in &mut strand_labels {
        let strand = u32::try_from(marker.0 + 1).unwrap_or(1);
        **text = match fiber_number(progress.selected_tube, strand) {
            Some(fiber) => {
                let entry = crate::splice::chart_entry(orders, fiber);
                let color = entry.map(|e| e.strand_color.as_str()).unwrap_or("?");
                let state = if progress.selected_fiber == Some(fiber) {
                    " *"
                } else if progress.attempts.iter().any(|a| a.fiber == fiber) {
                    " [spliced]"
                } else {
                    ""
                };
                format!("{color} f{fiber}{state}")
            }
            None => String::new(),
        };
    }
    for (marker, mut text) in &mut drop_labels {
        **text = match beat.and_then(|b| b.work_order.get(marker.0)) {
            Some(entry) => {
                let fed = progress
                    .attempts
                    .iter()
                    .find(|a| a.drop == entry.drop)
                    .map(|a| a.fiber);
                match fed {
                    Some(fiber) => format!("{} [f{}]", entry.drop, fiber),
                    None => entry.drop.clone(),
                }
            }
            None => String::new(),
        };
    }
    let presented = beat.map(|b| presented_tubes(orders, b)).unwrap_or_default();
    let drop_count = beat.map(|b| b.work_order.len()).unwrap_or(0);
    for (button, mut color, mut visibility) in &mut buttons {
        let (visible, active) = match button.0 {
            SpliceAction::Tube(tube) => (presented.contains(&tube), tube == progress.selected_tube),
            SpliceAction::Drop(slot) => (slot < drop_count, false),
            SpliceAction::Strand(strand) => (
                true,
                fiber_number(progress.selected_tube, strand) == progress.selected_fiber,
            ),
            SpliceAction::ChartClose
            | SpliceAction::ChartNext
            | SpliceAction::ChartPrev
            | SpliceAction::ChartToggle
            | SpliceAction::ClearBeat
            | SpliceAction::Verify => (true, false),
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        *color = if active {
            BackgroundColor(Color::srgb(0.30, 0.42, 0.62))
        } else {
            BackgroundColor(Color::srgb(0.12, 0.18, 0.30))
        };
    }
    for mut visibility in &mut chart_root {
        *visibility = if progress.chart_open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn cleanup_splice_console(
    mut commands: Commands,
    consoles: Query<Entity, With<SpliceConsoleRoot>>,
    charts: Query<Entity, With<SpliceChartRoot>>,
    mut progress: ResMut<SpliceProgress>,
) {
    for entity in &consoles {
        commands.entity(entity).despawn();
    }
    for entity in &charts {
        commands.entity(entity).despawn();
    }
    progress.reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::load_scenario;

    fn sp1() -> (crate::level::LevelDef, SpliceWorkOrdersDef) {
        let level = load_scenario("sp1").expect("sp1 scenario must load");
        let orders = level
            .splice_work_orders
            .clone()
            .expect("sp1 must carry splice work orders");
        (level, orders)
    }

    /// Splice the current beat exactly as ordered (spares standing
    /// in for damaged fibers) through the public tray verbs.
    fn splice_beat_true(progress: &mut SpliceProgress, orders: &SpliceWorkOrdersDef) {
        let beat = orders.beats[progress.beat_index].clone();
        for entry in &beat.work_order {
            let (expected, _) = crate::splice::expected_fiber(&beat, entry.fiber);
            let (tube, strand) = crate::splice::fiber_pair(expected).expect("expected fiber");
            progress.selected_tube = tube;
            assert!(progress.select_strand(strand), "pick fiber {expected}");
            assert!(
                progress.choose_drop(&entry.drop),
                "splice to {}",
                entry.drop
            );
        }
    }

    #[test]
    fn playing_all_three_beats_true_completes_the_gate() {
        let (level, orders) = sp1();
        let mut progress = SpliceProgress::default();
        progress.begin(&level.id, &orders);
        assert!(!progress.is_complete(&level.id, &orders));
        for _ in 0..orders.beats.len() {
            splice_beat_true(&mut progress, &orders);
            assert_eq!(progress.verify(&orders), Some(true));
        }
        assert!(progress.is_complete(&level.id, &orders));
        assert_eq!(progress.verify(&orders), None, "no beat remains");
        // Progress recorded against another level id never counts.
        assert!(!progress.is_complete("fj1", &orders));
    }

    #[test]
    fn the_fiber_2_fiber_13_transposition_fails_and_names_the_swap() {
        let (level, orders) = sp1();
        let mut progress = SpliceProgress::default();
        progress.begin(&level.id, &orders);
        splice_beat_true(&mut progress, &orders);
        assert_eq!(progress.verify(&orders), Some(true));
        // Beat 2, played wrong on purpose: fiber 2 and fiber 13 —
        // the same two colors transposed — each feed the other's
        // drop; every other splice is true.
        let beat = orders.beats[1].clone();
        for entry in &beat.work_order {
            let fiber = match entry.fiber {
                2 => 13,
                13 => 2,
                other => other,
            };
            progress.attempts.push(SpliceAttempt {
                fiber,
                drop: entry.drop.clone(),
            });
        }
        assert_eq!(progress.verify(&orders), Some(false));
        assert_eq!(progress.beats_passed, 1, "a failed beat is not banked");
        let evaluation = progress
            .last_evaluation
            .as_ref()
            .expect("the failed verify leaves its ledger");
        assert!(!evaluation.passed);
        let ledger = evaluation.ledger.join("\n");
        assert!(
            ledger.contains("Swapped legs")
                && ledger.contains("fiber 2 (blue tube, orange strand)")
                && ledger.contains("fiber 13 (orange tube, blue strand)"),
            "ledger must name the swapped legs:\n{ledger}"
        );
        assert!(
            ledger.contains("WRONG DROP"),
            "the wrong drops must be named:\n{ledger}"
        );
    }

    #[test]
    fn forcing_a_damaged_strand_fails_until_the_spare_is_spliced() {
        let (level, orders) = sp1();
        let mut progress = SpliceProgress::default();
        progress.begin(&level.id, &orders);
        for _ in 0..2 {
            splice_beat_true(&mut progress, &orders);
            assert_eq!(progress.verify(&orders), Some(true));
        }
        // Beat 3 played true, plus the tray hazard mishandled: the
        // damaged fiber 55 forced onto a drop nobody ordered.
        splice_beat_true(&mut progress, &orders);
        progress.attempts.push(SpliceAttempt {
            fiber: 55,
            drop: "Spare pigtail".to_string(),
        });
        assert_eq!(progress.verify(&orders), Some(false));
        let ledger = progress
            .last_evaluation
            .as_ref()
            .expect("ledger")
            .ledger
            .join("\n");
        assert!(
            ledger.contains("damaged") && ledger.contains("never forced"),
            "the forced strand must be called out:\n{ledger}"
        );
        // Excluding it (the authored play) passes.
        progress.attempts.retain(|a| a.fiber != 55);
        assert_eq!(progress.verify(&orders), Some(true));
        assert!(progress.is_complete(&level.id, &orders));
    }
}
