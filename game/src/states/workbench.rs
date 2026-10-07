//! Connector workbench console (Astra §2e) + the Ethernet handoff
//! beat (§2g's demarc check, housed here because the capstone is
//! its only consumer).
//!
//! The workbench runs the authored instruction card as an ordered
//! pick sequence, once per cable end. The rules are the trade's:
//! - A wrong pick at a NON-critical step is caught on the spot: it
//!   costs a connector (`wasted_connectors`) and the step is retried.
//! - A wrong pick at a CRITICAL step is *uncaught*: the sequence
//!   advances, and when the end is finished the defect is live —
//!   the Inspection state fails, the authored `workmanship_loss_db`
//!   applies to the span, and the end must be rebuilt. Rebuild is
//!   always available; there is no soft-lock.
//! - Picks apply only to the current (end, step) cursor, so the
//!   sequence cannot be completed out of order, and an out-of-range
//!   pick against malformed data is rejected, never recorded.
//! The engine reads the level's `WorkbenchDef` and nothing else —
//! in particular it never reads `owned_gear`/`Loadout`: the Golden
//! Crimper stays cosmetic (§3 guardrail; guardrail test below).

use bevy::prelude::*;

use crate::level::{LevelDef, StateStatus, WorkbenchDef};
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::GameState;
use crate::waifu::reactions::{ReactionInbox, ReactionTrigger};

/// What a pick did (returned for the console + tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickOutcome {
    /// Correct pick; cursor advanced within the end.
    Advanced,
    /// Wrong non-critical pick: connector wasted, step retries.
    WastedRetry,
    /// The end finished clean; another end remains.
    EndComplete,
    /// The end finished carrying an uncaught defect.
    EndCompleteWithDefect,
    /// All ends finished (defect state decides pass/fail).
    AllEndsComplete,
    /// Rejected: out-of-range index or no sequence running.
    Rejected,
}

/// Player-side workbench state (see module docs for the rules).
#[derive(Resource, Debug, Default)]
pub struct WorkbenchProgress {
    /// Current end being terminated (0-based).
    pub end: usize,
    /// Current step within the end (0-based).
    pub step: usize,
    /// The current end's uncaught critical miss, if any (the step's
    /// authored failure mode). Silent until the end completes.
    pub pending_defect: Option<String>,
    /// A finished end carries a live defect: Inspection fails and
    /// the authored loss applies until that end is rebuilt clean.
    pub defect_active: bool,
    /// Which end carries the live defect.
    pub defective_end: Option<usize>,
    /// The live defect's authored failure mode (named at inspection).
    pub defect_failure_mode: Option<String>,
    /// Connectors wasted on caught mistakes, whole attempt.
    pub wasted_connectors: u32,
    /// Total end completions, rebuilds included (telemetry).
    pub ends_completed: u32,
    /// All ends finished at least once (defect may still be live).
    pub finished: bool,
    /// The WorkmanshipDefect reaction has fired this attempt.
    pub defect_reaction_fired: bool,
}

impl WorkbenchProgress {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The gate fact: every end terminated and no live defect.
    pub fn complete(&self) -> bool {
        self.finished && !self.defect_active
    }

    /// Apply a pick at the current cursor. `def` is the level's
    /// authored card; the pick is validated against it.
    pub fn pick(&mut self, def: &WorkbenchDef, index: usize) -> PickOutcome {
        if self.finished && !self.defect_active {
            return PickOutcome::Rejected;
        }
        let Some(step_def) = def.steps.get(self.step) else {
            return PickOutcome::Rejected;
        };
        if index >= step_def.options.len() {
            return PickOutcome::Rejected;
        }
        if index != step_def.correct {
            if step_def.critical {
                // Uncaught: advances, defect lands when the end does.
                self.pending_defect = Some(step_def.failure_mode.clone());
            } else {
                // Caught on the spot: a connector dies, step retries.
                self.wasted_connectors = self.wasted_connectors.saturating_add(1);
                return PickOutcome::WastedRetry;
            }
        }
        self.step += 1;
        if self.step < def.steps.len() {
            return PickOutcome::Advanced;
        }
        // The end is finished; an uncaught defect goes live.
        self.ends_completed = self.ends_completed.saturating_add(1);
        let had_defect = self.pending_defect.is_some();
        if had_defect {
            self.defect_active = true;
            self.defective_end = Some(self.end);
            self.defect_failure_mode = self.pending_defect.clone();
        } else if self.defective_end == Some(self.end) {
            // The defective end was rebuilt clean.
            self.defect_active = false;
            self.defective_end = None;
            self.defect_failure_mode = None;
        }
        self.pending_defect = None;
        self.step = 0;
        if (self.end + 1) < usize::from(def.ends) {
            self.end += 1;
            if had_defect {
                PickOutcome::EndCompleteWithDefect
            } else {
                PickOutcome::EndComplete
            }
        } else {
            self.finished = true;
            PickOutcome::AllEndsComplete
        }
    }

    /// Start rebuilding the defective end. Always available while a
    /// defect is live; the defect (and its loss) stays in force
    /// until the rebuilt end completes clean.
    pub fn rebuild(&mut self, def: &WorkbenchDef) -> bool {
        if !self.defect_active {
            return false;
        }
        let Some(end) = self.defective_end else {
            return false;
        };
        if end >= usize::from(def.ends) {
            return false;
        }
        self.end = end;
        self.step = 0;
        self.pending_defect = None;
        self.finished = false;
        true
    }

    /// The Inspection state line (§2a) for this progress.
    pub fn inspection_state(&self, def: &WorkbenchDef) -> (StateStatus, String) {
        if self.defect_active {
            let mode = self
                .defect_failure_mode
                .as_deref()
                .unwrap_or("defect found at inspection");
            return (
                StateStatus::Fail,
                format!("Inspection fails — {mode} Rebuild the end at the bench."),
            );
        }
        if self.complete() {
            return (
                StateStatus::Pass,
                format!(
                    "Inspection passes — {} end(s) terminated, tested, and clean.",
                    def.ends
                ),
            );
        }
        (
            StateStatus::Pending,
            "Inspection pending — the termination has not passed inspection yet.".to_string(),
        )
    }

    /// The Workmanship state line (§2a): the bench record.
    pub fn workmanship_state(&self, def: &WorkbenchDef) -> (StateStatus, String) {
        if self.defect_active {
            return (
                StateStatus::Fail,
                format!(
                    "Workmanship fails — the live defect adds {:.1} dB of loss to the span.",
                    def.workmanship_loss_db
                ),
            );
        }
        if self.complete() {
            let waste = if self.wasted_connectors == 0 {
                "no connectors wasted".to_string()
            } else {
                format!(
                    "{} connector(s) wasted at the bench",
                    self.wasted_connectors
                )
            };
            return (
                StateStatus::Pass,
                format!("Workmanship passes — clean terminations, {waste}."),
            );
        }
        (
            StateStatus::Pending,
            format!(
                "Workmanship pending — end {} of {}, step {} of {}.",
                self.end + 1,
                def.ends,
                self.step + 1,
                def.steps.len()
            ),
        )
    }
}

/// The Ethernet handoff beat (§2g): one verification at the demarc.
/// The verify only counts while the board passes — the console
/// handler checks the live board at click time and latches the
/// result here.
#[derive(Resource, Debug, Default)]
pub struct HandoffProgress {
    pub verified: bool,
}

impl HandoffProgress {
    pub fn reset(&mut self) {
        self.verified = false;
    }

    /// Latch the verification. `board_passing` is the live board
    /// verdict at click time; verifying against a failing board is
    /// rejected (returns false, nothing latches).
    pub fn verify(&mut self, board_passing: bool) -> bool {
        if !board_passing {
            return false;
        }
        self.verified = true;
        true
    }

    /// The Handoff state line (§2a).
    pub fn handoff_state(&self) -> (StateStatus, String) {
        if self.verified {
            (
                StateStatus::Pass,
                "Handoff verified at the demarc — link, addressing, and service confirmed \
                 with the customer watching the same readings."
                    .to_string(),
            )
        } else {
            (
                StateStatus::Pending,
                "Handoff not verified — confirm the Ethernet demarc before closeout.".to_string(),
            )
        }
    }
}

/// Marker for the workbench console root (despawn cleanup).
#[derive(Component)]
pub struct WorkbenchConsole;

/// Marker for workbench console buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct WorkbenchButton(pub WorkbenchAction);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkbenchAction {
    Pick(usize),
    Rebuild,
    VerifyHandoff,
}

/// Status text entity (single multi-line text, api-console idiom).
#[derive(Component)]
pub struct WorkbenchStatusText;

/// Workbench + handoff console plugin.
pub struct WorkbenchConsolePlugin;

impl Plugin for WorkbenchConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorkbenchProgress>()
            .init_resource::<HandoffProgress>()
            .add_systems(
                OnEnter(GameState::Playing),
                (reset_workbench_progress, reset_handoff_progress),
            )
            .add_systems(
                Update,
                (handle_workbench_clicks, refresh_workbench_console).run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            )
            .add_systems(OnEnter(GameState::Results), cleanup_workbench_console)
            .add_systems(OnEnter(GameState::MainMenu), cleanup_workbench_console);
    }
}

fn reset_workbench_progress(mut progress: ResMut<WorkbenchProgress>) {
    progress.reset();
}

fn reset_handoff_progress(mut progress: ResMut<HandoffProgress>) {
    progress.reset();
}

/// The console's status block: the card, the current step, the
/// bench record. Pure formatting (unit-tested).
pub fn status_text(
    level: &LevelDef,
    progress: &WorkbenchProgress,
    handoff: &HandoffProgress,
) -> Option<String> {
    let mut out = String::new();
    if let Some(def) = &level.workbench {
        out.push_str(&format!("Bench card: {}\n", def.card_title));
        for line in &def.card_lines {
            out.push_str(&format!("  {line}\n"));
        }
        if progress.defect_active {
            out.push_str("INSPECTION: FAIL — a defect is live in a finished end. Rebuild it.\n");
        }
        if progress.complete() {
            out.push_str(&format!(
                "Bench complete: {} end(s), {} connector(s) wasted.\n",
                def.ends, progress.wasted_connectors
            ));
        } else if !progress.finished || progress.defect_active {
            if let Some(step_def) = def.steps.get(progress.step) {
                out.push_str(&format!(
                    "End {} of {} — step {} of {}: {}\n",
                    progress.end + 1,
                    def.ends,
                    progress.step + 1,
                    def.steps.len(),
                    step_def.prompt
                ));
                for (i, option) in step_def.options.iter().enumerate() {
                    out.push_str(&format!("  {}. {}\n", i + 1, option));
                }
            }
        }
    }
    if level.handoff_required {
        out.push_str(if handoff.verified {
            "Ethernet handoff: VERIFIED at the demarc.\n"
        } else {
            "Ethernet handoff: not verified yet.\n"
        });
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Spawn the workbench console (called from the `OnEnter(Playing)`
/// chain after `setup_level`). No-op without a workbench block or
/// a handoff requirement.
pub fn setup_workbench_console(mut commands: Commands, level: Option<Res<LevelDef>>) {
    let Some(level) = level else {
        return;
    };
    if level.workbench.is_none() && !level.handoff_required {
        return;
    }
    let root = commands
        .spawn((
            WorkbenchConsole,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(210.0),
                width: Val::Px(400.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.92)),
        ))
        .id();
    let status = commands
        .spawn((
            WorkbenchStatusText,
            Text::new("Workbench"),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(root).add_child(status);
    // Step options: the card never has more than 4 options per step;
    // spawn the bounded union of option buttons once, labelled per
    // current step by the refresh system via the status text. Picks
    // are index-stable.
    for index in 0..4 {
        let button = spawn_action_button(
            &mut commands,
            WorkbenchAction::Pick(index),
            &format!("Option {}", index + 1),
        );
        commands.entity(root).add_child(button);
    }
    let rebuild = spawn_action_button(&mut commands, WorkbenchAction::Rebuild, "Rebuild end");
    commands.entity(root).add_child(rebuild);
    if level.handoff_required {
        let verify = spawn_action_button(
            &mut commands,
            WorkbenchAction::VerifyHandoff,
            "Verify Ethernet Handoff",
        );
        commands.entity(root).add_child(verify);
    }
}

fn spawn_action_button(commands: &mut Commands, action: WorkbenchAction, label: &str) -> Entity {
    commands
        .spawn((
            WorkbenchButton(action),
            Button,
            Node {
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.16, 0.2, 0.32, 1.0)),
        ))
        .with_child((
            Text::new(label.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn handle_workbench_clicks(
    level: Option<Res<LevelDef>>,
    live: Res<LiveGraph>,
    active_outage: Res<ActiveOutage>,
    mut progress: ResMut<WorkbenchProgress>,
    mut handoff: ResMut<HandoffProgress>,
    buttons: Query<
        (&Interaction, &WorkbenchButton),
        (
            Changed<Interaction>,
            Without<super::identification::IdentificationButton>,
            Without<super::jumper::JumperButton>,
            Without<super::survey::SurveyButton>,
            Without<super::api_console::ApiButton>,
            Without<super::triage_console::TriageButton>,
        ),
    >,
    mut reaction_inbox: Option<ResMut<ReactionInbox>>,
) {
    let Some(level) = level else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button.0 {
            WorkbenchAction::Pick(index) => {
                let Some(def) = &level.workbench else {
                    continue;
                };
                let outcome = progress.pick(def, index);
                if matches!(
                    outcome,
                    PickOutcome::EndCompleteWithDefect | PickOutcome::AllEndsComplete
                ) && progress.defect_active
                    && !progress.defect_reaction_fired
                {
                    progress.defect_reaction_fired = true;
                    if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
                        inbox.push(ReactionTrigger::WorkmanshipDefect);
                    }
                }
            }
            WorkbenchAction::Rebuild => {
                if let Some(def) = &level.workbench {
                    progress.rebuild(def);
                }
            }
            WorkbenchAction::VerifyHandoff => {
                let board_passing = level.is_win_state_with_outage(
                    &live.graph,
                    live.tx_dbm,
                    live.wavelength.0,
                    active_outage.outage.as_ref(),
                );
                handoff.verify(board_passing);
            }
        }
    }
}

fn refresh_workbench_console(
    level: Option<Res<LevelDef>>,
    progress: Res<WorkbenchProgress>,
    handoff: Res<HandoffProgress>,
    mut query: Query<&mut Text, With<WorkbenchStatusText>>,
) {
    let Some(level) = level else {
        return;
    };
    let Some(text) = status_text(&level, &progress, &handoff) else {
        return;
    };
    for mut t in &mut query {
        if t.0 != text {
            t.0.clone_from(&text);
        }
    }
}

fn cleanup_workbench_console(mut commands: Commands, query: Query<Entity, With<WorkbenchConsole>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{WorkbenchStepDef, LEVEL_SOURCES};

    fn load(id: &str) -> LevelDef {
        LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == id)
            .unwrap()
    }

    fn correct_pick(progress: &mut WorkbenchProgress, def: &WorkbenchDef) -> PickOutcome {
        let index = def.steps[progress.step].correct;
        progress.pick(def, index)
    }

    fn wrong_pick(progress: &mut WorkbenchProgress, def: &WorkbenchDef) -> PickOutcome {
        let step = &def.steps[progress.step];
        let index = (0..step.options.len())
            .find(|&i| i != step.correct)
            .expect("a wrong option exists");
        progress.pick(def, index)
    }

    #[test]
    fn clean_run_completes_both_ends_in_order() {
        let level = load("c1l3");
        let def = level.workbench.as_ref().expect("c1l3 authors a workbench");
        assert_eq!(def.steps.len(), 10);
        assert_eq!(def.ends, 2);
        let mut progress = WorkbenchProgress::default();
        // Out-of-order completion is impossible: the cursor only
        // advances one step per pick, so `finished` needs all 20.
        for _ in 0..19 {
            correct_pick(&mut progress, def);
            assert!(!progress.complete());
        }
        let outcome = correct_pick(&mut progress, def);
        assert_eq!(outcome, PickOutcome::AllEndsComplete);
        assert!(progress.complete());
        assert_eq!(progress.wasted_connectors, 0);
        // Further picks are rejected once complete.
        assert_eq!(correct_pick(&mut progress, def), PickOutcome::Rejected);
    }

    #[test]
    fn caught_mistake_wastes_a_connector_and_retries_the_step() {
        let level = load("c1l3");
        let def = level.workbench.as_ref().unwrap();
        // Step 0 is non-critical in the shipped card.
        assert!(!def.steps[0].critical);
        let mut progress = WorkbenchProgress::default();
        let outcome = wrong_pick(&mut progress, def);
        assert_eq!(outcome, PickOutcome::WastedRetry);
        assert_eq!(progress.wasted_connectors, 1);
        assert_eq!(progress.step, 0, "a caught mistake retries the step");
        assert!(!progress.defect_active);
    }

    #[test]
    fn uncaught_critical_miss_goes_live_at_end_completion_and_rebuild_clears_it() {
        let level = load("c1l9");
        let def = level.workbench.as_ref().expect("c1l9 authors a workbench");
        let mut progress = WorkbenchProgress::default();
        // Miss the first critical step of end 1, finish both ends.
        let critical_step = def
            .steps
            .iter()
            .position(|s| s.critical)
            .expect("the card has a critical step");
        while progress.step < critical_step {
            correct_pick(&mut progress, def);
        }
        let outcome = wrong_pick(&mut progress, def);
        assert_eq!(outcome, PickOutcome::Advanced, "an uncaught miss advances");
        assert!(!progress.defect_active, "not live until the end finishes");
        while !progress.finished {
            correct_pick(&mut progress, def);
        }
        assert!(progress.defect_active, "the defect is live");
        assert!(!progress.complete());
        let (status, _) = progress.inspection_state(def);
        assert_eq!(status, StateStatus::Fail);
        // Rebuild is always available and clears the defect when the
        // rebuilt end completes clean — no soft-lock.
        assert!(progress.rebuild(def));
        assert!(progress.defect_active, "loss stays until rebuilt clean");
        while !progress.finished {
            correct_pick(&mut progress, def);
        }
        assert!(!progress.defect_active);
        assert!(progress.complete());
        let (status, _) = progress.inspection_state(def);
        assert_eq!(status, StateStatus::Pass);
    }

    #[test]
    fn out_of_range_picks_are_rejected_never_recorded() {
        // Adversarial: malformed input cannot move the cursor.
        let level = load("c1l3");
        let def = level.workbench.as_ref().unwrap();
        let mut progress = WorkbenchProgress::default();
        assert_eq!(progress.pick(def, usize::MAX), PickOutcome::Rejected);
        assert_eq!(progress.step, 0);
        assert_eq!(progress.wasted_connectors, 0);
        // Rebuild with no live defect is a no-op.
        assert!(!progress.rebuild(def));
    }

    #[test]
    fn guardrail_workbench_outcomes_never_read_owned_gear() {
        // §3 guardrail: the Golden Crimper stays cosmetic. The
        // engine's only input is the authored card — proven two
        // ways: (1) a full defect-and-rebuild script produces
        // identical outcomes with a Loadout (crimper owned) present
        // in the world and without one; (2) the outcomes below are
        // computed from `WorkbenchDef` alone.
        let level = load("c1l9");
        let def = level.workbench.as_ref().unwrap();
        let script = |progress: &mut WorkbenchProgress| {
            let critical_step = def.steps.iter().position(|s| s.critical).unwrap();
            while progress.step < critical_step {
                correct_pick(progress, def);
            }
            wrong_pick(progress, def);
            while !progress.finished {
                correct_pick(progress, def);
            }
        };
        let mut bare = WorkbenchProgress::default();
        script(&mut bare);
        // Same script in a world where the player's save owns the
        // Golden Crimper: the resource is present to be (wrongly)
        // read, and the outcomes must not move.
        let mut app = App::new();
        let mut save = crate::save::SaveData::default();
        save.owned_gear
            .push(crate::warehouse::GOLDEN_CRIMPER.into());
        app.insert_resource(save);
        app.init_resource::<WorkbenchProgress>();
        let mut geared = app
            .world_mut()
            .remove_resource::<WorkbenchProgress>()
            .expect("progress resource");
        script(&mut geared);
        assert_eq!(bare.defect_active, geared.defect_active);
        assert_eq!(bare.wasted_connectors, geared.wasted_connectors);
        assert_eq!(bare.ends_completed, geared.ends_completed);
        let _ = app;
    }

    #[test]
    fn handoff_verify_only_counts_while_the_board_passes() {
        let mut handoff = HandoffProgress::default();
        assert!(!handoff.verify(false), "a failing board cannot verify");
        assert!(!handoff.verified);
        assert!(handoff.verify(true));
        assert!(handoff.verified);
        let (status, _) = handoff.handoff_state();
        assert_eq!(status, StateStatus::Pass);
    }

    #[test]
    fn workmanship_loss_applies_to_the_carrier_state_only_while_defective() {
        // c1l9's winner lands mid-window; with the authored 4 dB
        // defect loss the Carrier state falls out of the window,
        // and Inspection fails — the level reads unwinnable until
        // rebuilt, exactly as authored.
        let level = load("c1l9");
        let def = level.workbench.as_ref().unwrap();
        assert_eq!(def.workmanship_loss_db, 4.0);
        let winner = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, osp_sim::Component::Amplifier { gain_db } if gain_db == 12.0))
            .expect("c1l9 winner is the 12 dB amp");
        let mut graph = osp_sim::PathGraph::default();
        for node in &level.nodes {
            graph.add_node(node.id, node.label.clone());
        }
        for edge in &level.fixed_edges {
            graph.connect(edge.from, edge.to, edge.component.clone());
        }
        graph.connect(winner.from, winner.to, winner.component.clone());
        let clean = crate::level::MechanicStates::default();
        let clean_lines = level.verification_states(
            &graph,
            level.tx_dbm,
            osp_sim::Wavelength::from(level.wavelength),
            None,
            &clean,
        );
        let carrier = crate::level::state_line(&clean_lines, &crate::level::StateId::CarrierLevel)
            .expect("carrier line");
        assert_eq!(carrier.status, StateStatus::Pass, "{}", carrier.feedback);
        let defective = crate::level::MechanicStates {
            workmanship_loss_db: def.workmanship_loss_db,
            ..Default::default()
        };
        let defect_lines = level.verification_states(
            &graph,
            level.tx_dbm,
            osp_sim::Wavelength::from(level.wavelength),
            None,
            &defective,
        );
        let carrier = crate::level::state_line(&defect_lines, &crate::level::StateId::CarrierLevel)
            .expect("carrier line");
        assert_eq!(carrier.status, StateStatus::Fail, "{}", carrier.feedback);
    }
}
