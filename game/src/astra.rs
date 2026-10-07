//! Astra shared layer: the win-gate conjunction for the optional
//! mechanic blocks (§2b–§2f), the assembly of mechanic-owned
//! verification states (§2a), attempt telemetry, and — from slice 8 —
//! badge evaluation and closeout assembly (§2g).
//!
//! Every Astra mechanic follows the established extension pattern: an
//! optional `LevelDef` block plus a console under `states/`. This
//! module is the one place the blocks' gates and state lines are
//! combined, so `playing::check_win_condition` and
//! `outage::check_outage_resolution` stay a single conjunction each
//! and the ledger/results surfaces render one identical vector.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use osp_sim::Component;

use crate::board::PlacedChoices;
use crate::level::{state_line, LevelDef, MechanicStates, StateId, StateStatus};
use crate::states::config_console::ConfigProgress;
use crate::states::identification::{identity_state, IdentificationProgress};
use crate::states::intermittent::IntermittentProgress;
use crate::states::jumper::JumperProgress;
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::survey::SurveyProgress;
use crate::states::workbench::{HandoffProgress, WorkbenchProgress};
use crate::states::GameState;

/// All Astra progress resources as one system parameter. The win
/// checks and the results screen are at Bevy's system-param limit;
/// bundling the mechanic resources here means each slice adds a
/// field, never a parameter. Fields are `Option` so headless test
/// worlds that init resources selectively degrade gracefully: a
/// missing resource beside a present block fails that gate closed.
#[derive(SystemParam)]
pub struct AstraProgress<'w> {
    pub ident: Option<Res<'w, IdentificationProgress>>,
    pub jumper: Option<Res<'w, JumperProgress>>,
    pub survey: Option<Res<'w, SurveyProgress>>,
    pub workbench: Option<Res<'w, WorkbenchProgress>>,
    pub handoff: Option<Res<'w, HandoffProgress>>,
    pub config: Option<Res<'w, ConfigProgress>>,
    pub intermittent: Option<Res<'w, IntermittentProgress>>,
}

impl AstraProgress<'_> {
    /// The identification progress, when the resource exists.
    pub fn ident(&self) -> Option<&IdentificationProgress> {
        self.ident.as_deref()
    }

    /// The jumper progress, when the resource exists.
    pub fn jumper(&self) -> Option<&JumperProgress> {
        self.jumper.as_deref()
    }

    /// The survey progress, when the resource exists.
    pub fn survey(&self) -> Option<&SurveyProgress> {
        self.survey.as_deref()
    }

    /// The workbench progress, when the resource exists.
    pub fn workbench(&self) -> Option<&WorkbenchProgress> {
        self.workbench.as_deref()
    }

    /// The handoff progress, when the resource exists.
    pub fn handoff(&self) -> Option<&HandoffProgress> {
        self.handoff.as_deref()
    }

    /// The static-config progress, when the resource exists.
    pub fn config(&self) -> Option<&ConfigProgress> {
        self.config.as_deref()
    }

    /// The intermittent progress, when the resource exists.
    pub fn intermittent(&self) -> Option<&IntermittentProgress> {
        self.intermittent.as_deref()
    }
}

/// The survey half of the win conjunction (§2d): on anchor levels
/// (`required_for_win`), every survey point must pass against the
/// live graph. Elsewhere the survey never gates. Kept apart from
/// `astra_gates_pass` because it needs the live graph.
pub fn survey_gate_pass(
    level: &LevelDef,
    graph: &osp_sim::PathGraph,
    tx_dbm: f64,
    outage: Option<&osp_sim::Outage>,
) -> bool {
    match &level.survey {
        Some(survey) if survey.required_for_win => level.survey_points_pass(graph, tx_dbm, outage),
        _ => true,
    }
}

/// True when every Astra mechanic gate on this level passes. Levels
/// without a block pass that mechanic vacuously — the conjunction is
/// over *present* blocks only, exactly like the api/triage gates.
/// Identification is superseded on the intermittent level (c1l6),
/// where the §2e composition owns the identity-shaped gate.
pub fn astra_gates_pass(level: &LevelDef, progress: &AstraProgress) -> bool {
    if level.intermittent.is_none() {
        if let Some(def) = &level.identification {
            match progress.ident() {
                Some(ident) if ident.identity_passed(def) => {}
                _ => return false,
            }
        }
    }
    // §2c: while the defective jumper is in the plant, Service
    // cannot pass, so the level cannot be won — substitution is the
    // only path. (The state vector shows *why*; this is the gate.)
    if level.defective_edge.is_some() {
        match progress.jumper() {
            Some(jumper) if jumper.swapped => {}
            _ => return false,
        }
    }
    // §2e: the bench sequence must be complete with no live defect.
    if level.workbench.is_some() {
        match progress.workbench() {
            Some(workbench) if workbench.complete() => {}
            _ => return false,
        }
    }
    // §2g: the capstone's Ethernet handoff must be verified (the
    // verify itself only latched while the board passed).
    if level.handoff_required {
        match progress.handoff() {
            Some(handoff) if handoff.verified => {}
            _ => return false,
        }
    }
    // §2f: each authored config family must be applied and passed —
    // per family, never a shared verdict (dual-stack rule).
    for (block, family) in [
        (&level.static_config_v4, crate::level::ConfigFamily::V4),
        (&level.static_config_v6, crate::level::ConfigFamily::V6),
    ] {
        if block.is_some() {
            match progress.config() {
                Some(config) if config.family_passed(family) => {}
                _ => return false,
            }
        }
    }
    true
}

/// Assemble the mechanic-owned verification states (§2a) from the
/// live progress resources. Board states are derived separately by
/// `LevelDef::verification_states`; this is only the mechanic half,
/// so both surfaces (ledger, results) merge identical lines.
pub fn mechanic_states(level: &LevelDef, progress: &AstraProgress) -> MechanicStates {
    let mut mechanics = MechanicStates::default();
    if level.intermittent.is_none() {
        if let (Some(def), Some(ident)) = (&level.identification, progress.ident()) {
            mechanics.identity = Some(identity_state(def, ident));
        }
    }
    mechanics.service_defect_present =
        level.defect_present(progress.jumper().is_some_and(|j| j.swapped));
    if let (Some(def), Some(workbench)) = (&level.workbench, progress.workbench()) {
        mechanics.inspection = Some(workbench.inspection_state(def));
        mechanics.workmanship = Some(workbench.workmanship_state(def));
        if workbench.defect_active {
            mechanics.workmanship_loss_db = def.workmanship_loss_db;
        }
    }
    if level.handoff_required {
        if let Some(handoff) = progress.handoff() {
            mechanics.handoff = Some(handoff.handoff_state());
        }
    }
    if let Some(config) = progress.config() {
        if let (Some(def), Some(form)) = (
            &level.static_config_v4,
            config.family(crate::level::ConfigFamily::V4),
        ) {
            mechanics.ipv4 = Some(form.state(def));
        }
        if let (Some(def), Some(form)) = (
            &level.static_config_v6,
            config.family(crate::level::ConfigFamily::V6),
        ) {
            mechanics.ipv6 = Some(form.state(def));
        }
        // The DNS line follows the families: it passes only when
        // every authored family has been accepted with its
        // worksheet DNS (each family's verdict checks its own).
        let any_block = level.static_config_v4.is_some() || level.static_config_v6.is_some();
        if any_block {
            let all_passed = [&level.static_config_v4, &level.static_config_v6]
                .into_iter()
                .flatten()
                .all(|def| config.family_passed(def.family));
            mechanics.dns = Some(if all_passed {
                (
                    StateStatus::Pass,
                    "DNS resolves to the worksheet server on every configured family.".to_string(),
                )
            } else {
                (
                    StateStatus::Pending,
                    "DNS not yet verified on every configured family.".to_string(),
                )
            });
        }
    }
    mechanics
}

/// Attempt telemetry (§2g): the evidence the badges and the closeout
/// are evaluated from. Recorded by `watch_attempt` (state-vector
/// observations, placement history) and by the consoles themselves
/// (their progress resources). Nothing here is stored as truth about
/// the board — it is the attempt's *record*.
#[derive(Resource, Debug, Default)]
pub struct AttemptTelemetry {
    /// Placement-change generations seen this attempt.
    pub change_seq: u64,
    /// A passing full evaluation was observed after the final
    /// placement change — the Verification badge's core fact
    /// ("distinguish a repair action from a verified resolution").
    pub verified_after_final_change: bool,
    /// The first state vector's summary + failures (closeout's
    /// "initial observations").
    pub first_states_summary: Option<String>,
    pub first_states_failures: Vec<String>,
    /// The C3 moment occurred: Service failed while Carrier Level
    /// passed, at any point this attempt.
    pub observed_service_fail: bool,
    /// …and it occurred while the defective jumper was still in
    /// place (evidence-led substitution, before the swap).
    pub observed_service_fail_before_swap: bool,
    /// The service_fail reaction has fired this attempt.
    pub service_fail_fired: bool,
    /// The survey_point_fail reaction has fired this attempt.
    pub survey_fail_fired: bool,
    /// Placement changes after the first placement (re-plans).
    pub placement_revisions: u32,
    /// On c1l7, whether the player's first placement was an
    /// amplifier (the physical-plant-first trap, noted for the
    /// Diagnosis badge).
    pub first_placed_was_amplifier: Option<bool>,
    /// Times a placed slot's component was swapped for a different
    /// one (working-parts-replaced proxy for the Workmanship badge).
    pub working_parts_replaced: u32,
    /// Last placement snapshot `(edge, slot)` pairs, sorted.
    pub placed_snapshot: Vec<((u32, u32), usize)>,
    /// The first-placement fact has been recorded.
    pub placed_seen: bool,
}

impl AttemptTelemetry {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Astra plugin: telemetry resource + the attempt watcher.
pub struct AstraPlugin;

impl Plugin for AstraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AttemptTelemetry>()
            .add_systems(OnEnter(GameState::Playing), reset_attempt_telemetry)
            .add_systems(
                Update,
                watch_attempt.run_if(
                    in_state(GameState::Playing).or_else(in_state(GameState::OutageActive)),
                ),
            );
    }
}

fn reset_attempt_telemetry(mut telemetry: ResMut<AttemptTelemetry>) {
    telemetry.reset();
}

/// Per-frame attempt watcher: records placement history, the first
/// state vector, the C3 service-failure moment (firing Ondine's
/// `service_fail` reaction once), and whether a passing evaluation
/// was observed after the final placement change.
#[allow(clippy::too_many_arguments)]
fn watch_attempt(
    level: Option<Res<LevelDef>>,
    live: Res<LiveGraph>,
    placed: Res<PlacedChoices>,
    active_outage: Res<ActiveOutage>,
    progress: AstraProgress,
    mut telemetry: ResMut<AttemptTelemetry>,
    mut reaction_inbox: Option<ResMut<crate::waifu::reactions::ReactionInbox>>,
) {
    let Some(level) = level else {
        return;
    };
    if placed.is_changed() {
        let mut snapshot: Vec<((u32, u32), usize)> =
            placed.0.iter().map(|(edge, slot)| (*edge, *slot)).collect();
        snapshot.sort();
        if telemetry.placed_seen {
            telemetry.placement_revisions = telemetry.placement_revisions.saturating_add(1);
            for (edge, slot) in &snapshot {
                if let Some((_, old_slot)) =
                    telemetry.placed_snapshot.iter().find(|(e, _)| e == edge)
                {
                    if old_slot != slot {
                        telemetry.working_parts_replaced =
                            telemetry.working_parts_replaced.saturating_add(1);
                    }
                }
            }
        } else if !snapshot.is_empty() {
            telemetry.placed_seen = true;
            let (edge, slot) = snapshot[0];
            telemetry.first_placed_was_amplifier = level
                .available_components
                .iter()
                .filter(|c| c.from == edge.0 && c.to == edge.1)
                .nth(slot)
                .map(|c| matches!(c.component, Component::Amplifier { .. }));
        }
        telemetry.placed_snapshot = snapshot;
        telemetry.change_seq = telemetry.change_seq.saturating_add(1);
        telemetry.verified_after_final_change = false;
    }

    let outage = active_outage.outage.as_ref();
    let mechanics = mechanic_states(&level, &progress);
    let lines = level.verification_states(
        &live.graph,
        live.tx_dbm,
        live.wavelength.0,
        outage,
        &mechanics,
    );
    if telemetry.first_states_summary.is_none() {
        telemetry.first_states_summary = Some(crate::level::states_summary(&lines));
        telemetry.first_states_failures = crate::level::states_failures(&lines);
    }
    // The C3 moment: service fails while the carrier level passes.
    let service_failed =
        state_line(&lines, &StateId::ServiceCnr).is_some_and(|l| l.status == StateStatus::Fail);
    let carrier_passed =
        state_line(&lines, &StateId::CarrierLevel).is_some_and(|l| l.status == StateStatus::Pass);
    if service_failed && carrier_passed {
        telemetry.observed_service_fail = true;
        if mechanics.service_defect_present {
            telemetry.observed_service_fail_before_swap = true;
        }
        if !telemetry.service_fail_fired {
            telemetry.service_fail_fired = true;
            if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
                inbox.push(crate::waifu::reactions::ReactionTrigger::ServiceStateFail);
            }
        }
    }
    // A failing Coverage point fires Linka's survey reaction once.
    let coverage_failed = lines
        .iter()
        .any(|l| matches!(l.id, StateId::Coverage(_)) && l.status == StateStatus::Fail);
    if coverage_failed && !telemetry.survey_fail_fired {
        telemetry.survey_fail_fired = true;
        if let Some(mut inbox) = reaction_inbox.as_deref_mut() {
            inbox.push(crate::waifu::reactions::ReactionTrigger::SurveyPointFail);
        }
    }
    // Verification: a board pass (with every survey point, where a
    // survey is authored) observed on a frame with no placement
    // change counts as a re-test after the final change.
    if !placed.is_changed()
        && level.is_win_state_with_outage(&live.graph, live.tx_dbm, live.wavelength.0, outage)
        && level.survey_points_pass(&live.graph, live.tx_dbm, outage)
    {
        telemetry.verified_after_final_change = true;
    }
}

// ---------------------------------------------------------------------------
// §2g: badges, optional objectives, closeout. Badges pay nothing and
// are never awarded on a failed level; optional objectives pay flat
// Cores (unmultiplied, like salvage). Everything below is a pure
// function of the attempt record — the same inputs always produce
// the same verdict, so results and tests agree by construction.
// ---------------------------------------------------------------------------

/// Badge bit: the fault was diagnosed from evidence.
pub const BADGE_DIAGNOSIS: u8 = 0b001;
/// Badge bit: clean workmanship (bench) or configuration (forms).
pub const BADGE_WORKMANSHIP: u8 = 0b010;
/// Badge bit: the fix was re-verified after the final change.
pub const BADGE_VERIFICATION: u8 = 0b100;

/// Display names for earned badge bits, in bit order.
pub fn badge_names(mask: u8) -> Vec<&'static str> {
    let mut names = Vec::new();
    if mask & BADGE_DIAGNOSIS != 0 {
        names.push("Diagnosis");
    }
    if mask & BADGE_WORKMANSHIP != 0 {
        names.push("Workmanship / Configuration");
    }
    if mask & BADGE_VERIFICATION != 0 {
        names.push("Verification");
    }
    names
}

/// Evaluate the attempt's badges. Rules (each is evidence-specific;
/// there is no participation badge):
/// - Diagnosis: the survey diagnosis pick is correct (and not the
///   trap); OR the intermittent fault was repaired (not rerouted);
///   OR the service failure was observed *before* the jumper swap
///   and the swap was made (evidence-led substitution); OR the
///   identification was completed with zero wrong selections.
/// - Workmanship / Configuration: a workbench level finished with
///   zero wasted connectors and no rebuild needed; OR a config
///   level where every family passed with zero rejected applies;
///   OR (levels with neither) no placed part was ever replaced.
/// - Verification: a passing evaluation was observed after the
///   final placement change (`verified_after_final_change`).
/// A failed level earns nothing, by construction.
pub fn evaluate_badges(
    level: &LevelDef,
    won: bool,
    telemetry: Option<&AttemptTelemetry>,
    progress: &AstraProgress,
) -> u8 {
    if !won {
        return 0;
    }
    let mut mask = 0u8;
    // Diagnosis.
    let mut diagnosis = false;
    if let (Some(survey), Some(sp)) = (&level.survey, progress.survey()) {
        if let Some(def) = &survey.diagnosis {
            diagnosis |= sp.diagnosis_correct(def) && !sp.trap_picked(def);
        }
    }
    if let Some(intermittent) = progress.intermittent() {
        diagnosis |= level.intermittent.is_some() && intermittent.repair_path_complete();
    }
    if let Some(t) = telemetry {
        diagnosis |= level.defective_edge.is_some()
            && t.observed_service_fail_before_swap
            && progress.jumper().is_some_and(|j| j.swapped);
    }
    if let (Some(def), Some(ident)) = (&level.identification, progress.ident()) {
        diagnosis |= level.intermittent.is_none()
            && ident.identity_passed(def)
            && ident.wrong_selections == 0;
    }
    if diagnosis {
        mask |= BADGE_DIAGNOSIS;
    }
    // Workmanship / Configuration.
    let mut workmanship = false;
    if let (Some(def), Some(wb)) = (&level.workbench, progress.workbench()) {
        workmanship |=
            wb.complete() && wb.wasted_connectors == 0 && wb.ends_completed == u32::from(def.ends);
    }
    let has_config = level.static_config_v4.is_some() || level.static_config_v6.is_some();
    if has_config {
        if let Some(config) = progress.config() {
            let families: Vec<&crate::states::config_console::FamilyProgress> = [
                config.family(crate::level::ConfigFamily::V4),
                config.family(crate::level::ConfigFamily::V6),
            ]
            .into_iter()
            .flatten()
            .collect();
            workmanship |= !families.is_empty()
                && families.iter().all(|f| f.passed && f.rejected_applies == 0);
        }
    }
    if level.workbench.is_none() && !has_config {
        if let Some(t) = telemetry {
            workmanship |= t.working_parts_replaced == 0;
        }
    }
    if workmanship {
        mask |= BADGE_WORKMANSHIP;
    }
    // Verification.
    if telemetry.is_some_and(|t| t.verified_after_final_change) {
        mask |= BADGE_VERIFICATION;
    }
    mask
}

/// Whether the level's optional objective was met this attempt
/// (evaluated on a win by the results screen; pays flat Cores).
pub fn objective_met(level: &LevelDef, progress: &AstraProgress) -> bool {
    let Some(objective) = &level.optional_objective else {
        return false;
    };
    match objective.kind {
        crate::level::ObjectiveKind::ZeroNeighborDisruptions => progress
            .ident()
            .is_some_and(|i| i.neighbor_disruptions == 0),
        crate::level::ObjectiveKind::ZeroWastedConnectors => progress
            .workbench()
            .is_some_and(|w| w.complete() && w.wasted_connectors == 0),
        crate::level::ObjectiveKind::RepairNotReroute => progress
            .intermittent()
            .is_some_and(|i| i.repair_path_complete()),
        crate::level::ObjectiveKind::ZeroWrongSelections => {
            let ident_ok = progress.ident().is_some_and(|i| i.wrong_selections == 0);
            let diagnosis_ok = match (&level.survey, progress.survey()) {
                (Some(survey), Some(sp)) => match &survey.diagnosis {
                    Some(def) => sp.diagnosis_correct(def) && !sp.trap_picked(def),
                    None => true,
                },
                _ => true,
            };
            ident_ok && diagnosis_ok
        }
    }
}

/// The dual-stack acceptance line (§2f/§2g): one clause per family,
/// each family's own verdict — IPv4 passing never verifies IPv6.
/// The mixed case is the spec's exact sentence.
pub fn dual_stack_line(level: &LevelDef, config: Option<&ConfigProgress>) -> Option<String> {
    if level.static_config_v4.is_none() || level.static_config_v6.is_none() {
        return None;
    }
    let passed =
        |family: crate::level::ConfigFamily| config.is_some_and(|c| c.family_passed(family));
    let v4 = passed(crate::level::ConfigFamily::V4);
    let v6 = passed(crate::level::ConfigFamily::V6);
    Some(match (v4, v6) {
        (true, true) => "IPv4 passes; IPv6 passes — dual-stack verified.".to_string(),
        (true, false) => "IPv4 passes; IPv6 has not yet been verified".to_string(),
        (false, true) => "IPv4 has not yet been verified; IPv6 passes".to_string(),
        (false, false) => {
            "IPv4 has not yet been verified; IPv6 has not yet been verified".to_string()
        }
    })
}

/// The auto-filled closeout (§2g): complaint, initial observations,
/// diagnosed fault with its evidence, changes made, and the final
/// verification. The player confirms it on the results screen; the
/// Documentation state passes only when every field is on record.
#[derive(Debug, Clone, Default)]
pub struct Closeout {
    pub complaint: String,
    pub initial_observations: String,
    pub diagnosed_fault: String,
    pub changes_made: String,
    pub final_verification: String,
}

impl Closeout {
    /// Every field on record (final verification excluded — it is
    /// rendered from the results screen's own state vector).
    pub fn documentation_ready(&self) -> bool {
        !self.complaint.is_empty()
            && !self.initial_observations.is_empty()
            && !self.diagnosed_fault.is_empty()
            && !self.changes_made.is_empty()
    }

    /// The rendered closeout block for the results screen.
    pub fn render(&self) -> String {
        format!(
            "Closeout — complaint: {}\nInitial observations: {}\nDiagnosed fault: {}\n\
             Changes made: {}\nFinal verification: {}",
            self.complaint,
            self.initial_observations,
            self.diagnosed_fault,
            self.changes_made,
            self.final_verification
        )
    }
}

/// Assemble the closeout from the attempt record. Fields with no
/// evidence stay empty — the Documentation state then honestly
/// stays Pending instead of inventing a record.
pub fn assemble_closeout(
    level: &LevelDef,
    telemetry: Option<&AttemptTelemetry>,
    progress: &AstraProgress,
) -> Closeout {
    let mut out = Closeout {
        complaint: format!("{} ({})", level.title, level.id),
        ..Default::default()
    };
    let Some(t) = telemetry else {
        return out;
    };
    out.initial_observations = if t.first_states_failures.is_empty() {
        t.first_states_summary.clone().unwrap_or_default()
    } else {
        t.first_states_failures.join("; ")
    };
    // Diagnosed fault: the strongest evidence the attempt produced.
    out.diagnosed_fault = if level.defective_edge.is_some()
        && progress.jumper().is_some_and(|j| j.swapped)
    {
        "Degraded jumper in the plant — service noise floor raised; substituted with the \
         bench-tested spare."
            .to_string()
    } else if level.intermittent.is_some() && progress.intermittent().is_some_and(|i| i.repaired) {
        "Loose fitting on the primary run — tightened to the card and re-verified under \
         disturbance."
            .to_string()
    } else if let (Some(survey), Some(sp)) = (&level.survey, progress.survey()) {
        match &survey.diagnosis {
            Some(def) if sp.diagnosis_correct(def) => {
                format!(
                    "Diagnosed from the survey history: {}",
                    def.options[def.correct]
                )
            }
            _ => String::new(),
        }
    } else if level.workbench.is_some()
        && progress.workbench().is_some_and(|w| w.ends_completed > 0)
    {
        "Termination workmanship verified at the bench (inspection record on file).".to_string()
    } else if level.static_config_v4.is_some() || level.static_config_v6.is_some() {
        "Static configuration corrected to the assigned worksheet.".to_string()
    } else {
        String::new()
    };
    // Changes made: the attempt's action record.
    let mut changes: Vec<String> = Vec::new();
    if t.placement_revisions > 0 {
        changes.push(format!("{} placement revision(s)", t.placement_revisions));
    }
    if progress.jumper().is_some_and(|j| j.swapped) {
        changes.push("jumper substituted".to_string());
    }
    if let Some(wb) = progress.workbench() {
        if wb.ends_completed > 0 {
            changes.push(format!("{} bench end(s) terminated", wb.ends_completed));
        }
        if wb.wasted_connectors > 0 {
            changes.push(format!("{} connector(s) wasted", wb.wasted_connectors));
        }
    }
    if let Some(config) = progress.config() {
        let applies: u32 = [
            config.family(crate::level::ConfigFamily::V4),
            config.family(crate::level::ConfigFamily::V6),
        ]
        .into_iter()
        .flatten()
        .map(|f| f.applies)
        .sum();
        if applies > 0 {
            changes.push(format!("{applies} configuration apply(ies)"));
        }
    }
    if progress.intermittent().is_some_and(|i| i.repaired) {
        changes.push("loose fitting tightened".to_string());
    }
    out.changes_made = changes.join("; ");
    out.final_verification = if t.verified_after_final_change {
        "Passing evaluation observed after the final change.".to_string()
    } else {
        "No post-change verification observed.".to_string()
    };
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LEVEL_SOURCES;

    fn load(id: &str) -> LevelDef {
        LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == id)
            .unwrap()
    }

    #[derive(Resource, Default)]
    struct BadgeResult(u8);

    #[derive(Resource)]
    struct WonFlag(bool);

    fn eval_badges_system(
        level: Res<LevelDef>,
        won: Res<WonFlag>,
        telemetry: Option<Res<AttemptTelemetry>>,
        progress: AstraProgress,
        mut out: ResMut<BadgeResult>,
    ) {
        out.0 = evaluate_badges(&level, won.0, telemetry.as_deref(), &progress);
    }

    fn badge_app(level_id: &str, won: bool) -> App {
        let mut app = App::new();
        app.insert_resource(load(level_id));
        app.insert_resource(WonFlag(won));
        app.init_resource::<BadgeResult>();
        app.add_systems(Update, eval_badges_system);
        app
    }

    #[test]
    fn badges_never_award_on_a_failed_level() {
        // Adversarial: a perfect evidence record on a loss earns
        // nothing — the won=false arm is unconditional.
        let mut app = badge_app("c1l5", false);
        app.insert_resource(AttemptTelemetry {
            observed_service_fail_before_swap: true,
            verified_after_final_change: true,
            ..Default::default()
        });
        app.insert_resource(crate::states::jumper::JumperProgress {
            swapped: true,
            compared: true,
            swap_attempts: 1,
        });
        app.update();
        assert_eq!(app.world().resource::<BadgeResult>().0, 0);
    }

    #[test]
    fn evidence_led_swap_earns_diagnosis_and_verification() {
        let mut app = badge_app("c1l5", true);
        app.insert_resource(AttemptTelemetry {
            observed_service_fail_before_swap: true,
            verified_after_final_change: true,
            ..Default::default()
        });
        app.insert_resource(crate::states::jumper::JumperProgress {
            swapped: true,
            compared: true,
            swap_attempts: 1,
        });
        app.update();
        let mask = app.world().resource::<BadgeResult>().0;
        assert!(mask & BADGE_DIAGNOSIS != 0, "{mask:03b}");
        assert!(mask & BADGE_VERIFICATION != 0, "{mask:03b}");
    }

    #[test]
    fn swap_without_observed_failure_earns_no_diagnosis() {
        // Swapping blind (no C3 moment observed first) is a guess,
        // not a diagnosis.
        let mut app = badge_app("c1l5", true);
        app.insert_resource(AttemptTelemetry::default());
        app.insert_resource(crate::states::jumper::JumperProgress {
            swapped: true,
            compared: false,
            swap_attempts: 1,
        });
        app.update();
        let mask = app.world().resource::<BadgeResult>().0;
        assert_eq!(mask & BADGE_DIAGNOSIS, 0, "{mask:03b}");
    }

    #[test]
    fn survey_diagnosis_pick_earns_diagnosis_but_the_trap_does_not() {
        let level = load("m1l2");
        let def = level.survey.as_ref().unwrap().diagnosis.as_ref().unwrap();
        let mut app = badge_app("m1l2", true);
        let mut survey = crate::states::survey::SurveyProgress::default();
        assert!(survey.pick_diagnosis(def, def.correct));
        app.insert_resource(survey);
        app.insert_resource(AttemptTelemetry::default());
        app.update();
        assert!(app.world().resource::<BadgeResult>().0 & BADGE_DIAGNOSIS != 0);

        let mut app = badge_app("m1l2", true);
        let mut survey = crate::states::survey::SurveyProgress::default();
        assert!(survey.pick_diagnosis(def, def.trap_option.unwrap()));
        app.insert_resource(survey);
        app.insert_resource(AttemptTelemetry::default());
        app.update();
        assert_eq!(app.world().resource::<BadgeResult>().0 & BADGE_DIAGNOSIS, 0);
    }

    #[test]
    fn dual_stack_line_names_each_family_verdict() {
        let level = load("m1l10");
        assert!(level.static_config_v4.is_some() && level.static_config_v6.is_some());
        // Nothing applied yet.
        let progress = ConfigProgress::default();
        assert_eq!(
            dual_stack_line(&level, Some(&progress)).as_deref(),
            Some("IPv4 has not yet been verified; IPv6 has not yet been verified")
        );
        // V4 passed only: the spec's exact sentence.
        let mut seeded = ConfigProgress::default();
        seeded.seed(&level);
        let def = level.static_config_v4.as_ref().unwrap().clone();
        let mut form = seeded.v4.take().unwrap();
        form.address = def.worksheet_address.clone();
        form.prefix_len = def.worksheet_prefix_len;
        form.gateway = def.worksheet_gateway.clone();
        form.dns = def.worksheet_dns.clone();
        assert!(form.apply(&def).is_ok());
        seeded.v4 = Some(form);
        assert_eq!(
            dual_stack_line(&level, Some(&seeded)).as_deref(),
            Some("IPv4 passes; IPv6 has not yet been verified")
        );
        // Single-family levels have no dual-stack line.
        let v4_only = load("m1l5");
        assert_eq!(dual_stack_line(&v4_only, Some(&seeded)), None);
    }

    #[derive(Resource, Default)]
    struct CloseoutResult(Option<Closeout>);

    fn assemble_system(
        level: Res<LevelDef>,
        telemetry: Option<Res<AttemptTelemetry>>,
        progress: AstraProgress,
        mut out: ResMut<CloseoutResult>,
    ) {
        out.0 = Some(assemble_closeout(&level, telemetry.as_deref(), &progress));
    }

    #[test]
    fn closeout_stays_unready_without_telemetry() {
        // Fail-closed documentation: no attempt record, no
        // Documentation pass — the evidence fields stay empty.
        let mut app = App::new();
        app.insert_resource(load("c1l1"));
        app.init_resource::<CloseoutResult>();
        app.add_systems(Update, assemble_system);
        app.update();
        let closeout = app
            .world()
            .resource::<CloseoutResult>()
            .0
            .clone()
            .expect("assembled");
        assert!(!closeout.documentation_ready());
        assert!(
            !closeout.complaint.is_empty(),
            "the work order always names the job"
        );
    }
}
