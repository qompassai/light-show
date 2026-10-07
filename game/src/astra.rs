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
