//! Level data model — deserialized from `assets/levels/*.json`. Each level
//! describes a starting `PathGraph` layout (some edges pre-placed, some left
//! for the player to complete), a target receive window, and optionally a
//! scripted outage that fires after N seconds of play.

use bevy::prelude::*;
use osp_sim::{
    CoaxViolation, Component, EthernetViolation, Medium, Outage, OutageKind, PathGraph,
    ReceiveWindow, Wavelength, WirelessViolation,
};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum MediumDef {
    Fiber,
    Coax,
    Wireless,
    Ethernet,
    /// Study/quiz levels (Léa's track): no physical medium is simulated.
    /// The board is unused; the quiz UI owns the win condition.
    Study,
}

impl From<MediumDef> for Medium {
    fn from(m: MediumDef) -> Self {
        match m {
            MediumDef::Fiber => Medium::Fiber,
            MediumDef::Coax => Medium::Coax,
            MediumDef::Wireless => Medium::Wireless,
            MediumDef::Ethernet => Medium::Ethernet,
            // Quiz levels never simulate the board; the mapping is
            // unreachable in practice. Fiber is the arbitrary fallback.
            MediumDef::Study => Medium::Fiber,
        }
    }
}

/// A Calix service profile: the service tier an ONT is provisioned for.
/// Clara's track. Profiles are a fixed catalog (not per-level data):
/// levels name a demanded tier per subscriber, and the win check verifies
/// every ONT lands inside its tier's receive window.
#[derive(Debug, Clone, Copy)]
pub struct ServiceProfile {
    /// Catalog key used in level JSON, e.g. "GPON-100".
    pub name: &'static str,
    pub down_mbps: u32,
    pub up_mbps: u32,
    /// Most sensitive the ONT may need (window floor, dBm).
    pub min_rx_dbm: f32,
    /// Overload point (window ceiling, dBm).
    pub max_rx_dbm: f32,
    /// Widest split this tier supports (port-count planning).
    pub max_split_ratio: u32,
}

/// The provisionable tiers. GPON tiers share the classic -27/-8 dBm
/// window; XGS-PON runs hotter and tighter.
pub const SERVICE_PROFILES: &[ServiceProfile] = &[
    ServiceProfile {
        name: "GPON-100",
        down_mbps: 100,
        up_mbps: 25,
        min_rx_dbm: -27.0,
        max_rx_dbm: -8.0,
        max_split_ratio: 32,
    },
    ServiceProfile {
        name: "GPON-500",
        down_mbps: 500,
        up_mbps: 125,
        min_rx_dbm: -27.0,
        max_rx_dbm: -8.0,
        max_split_ratio: 32,
    },
    ServiceProfile {
        name: "XGS-1000",
        down_mbps: 1000,
        up_mbps: 500,
        min_rx_dbm: -26.0,
        max_rx_dbm: -9.0,
        max_split_ratio: 64,
    },
];

/// Look up a profile by catalog name. `None` for unknown tiers — callers
/// fail closed (an undemanded tier is a level-authoring bug, not a pass).
pub fn service_profile(name: &str) -> Option<&'static ServiceProfile> {
    SERVICE_PROFILES.iter().find(|p| p.name == name)
}

/// A subscriber on a provisioning level: an ONT with a demanded tier and
/// a drop distance. Deserialized from the level JSON; `node` is the ONT's
/// board node id (for highlight/selection), `distance_km` drives the
/// budget math.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriberDef {
    pub name: String,
    pub node: u32,
    pub profile: String,
    pub distance_km: f64,
    /// EXOS Registration ID (see docs/reference/tds/cms/guides/exos-provisioning.md):
    /// unique per ONT, at most 10 chars, alphanumeric. The office
    /// pre-provisions it; the field tech (player) verifies it at turn-up.
    /// Validation (uniqueness, length, charset) runs in the level tests.
    #[serde(default)]
    pub reg_id: String,
}

/// An NBI/SMx/BxE API operation the player can issue. Clara's API-puzzle
/// levels use the NBI/SMx variants (see `docs/reference/tds/cms/guides/`);
/// Aino's NOC levels use the AMS NBI variants; Hikari's field levels use
/// the BxE portal variants (see `docs/reference/tds/bxe/`).
/// Deserialized from level JSON by variant name; unknown names fail the
/// parse loudly. NBI (CMS/AMS) speaks SOAP/XML; SMx speaks REST/JSON;
/// BxE is the field portal workflow -- the enum covers all three, and
/// each level's `api` tag says which door you're knocking on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ApiOp {
    /// Authenticate. NBI: `<auth><login>` returning a SessionID;
    /// SMx: HTTP Basic on every request (modeled as one login step).
    Login,
    /// Read before you write. NBI: `<action><action-type>show-ont</action-type>`;
    /// SMx: GET the ONT/subscriber status.
    ShowOnt,
    /// Provision the service. NBI: `edit-config` with `operation="create"`;
    /// SMx: POST the service.
    CreateService,
    /// Change an existing record. NBI: `edit-config` with `operation="merge"`.
    UpdateService,
    /// Bounce the box. NBI: `<action>` reboot; SMx: ONT reboot endpoint.
    RebootOnt,
    /// Read the service back to confirm it stuck.
    VerifyService,
    /// NBI session logout. SMx is stateless — included as a distractor
    /// on SMx levels (a real tech would know there's nothing to log out of).
    Logout,
    /// SMx REST: POST /ems/subscriber.
    CreateSubscriber,
    /// SMx REST: pre-provision the ONT (serial, profile, PON, reg ID).
    CreateOnt,
    /// SMx REST: DELETE the subscriber (churn / cleanup).
    DeleteSubscriber,
    // --- AMS NBI (Aino NOC track): SOAP operations from the
    // `10-nbi-soap-reference.md` endpoint catalog. ---
    /// AMS NBI: `getAllManagedElements` -- list every supervised NE.
    GetAllManagedElements,
    /// AMS NBI: `getManagedElement` -- pull one NE record.
    GetManagedElement,
    /// AMS NBI: `startSupervision` -- begin polling an NE.
    StartSupervision,
    /// AMS NBI: `stopSupervision` -- pause polling (pre-maintenance).
    StopSupervision,
    /// AMS NBI: `getStatus` -- read the NE current alarm state.
    GetNeStatus,
    /// AMS NBI: `enableMaintenanceMode` -- silence alarms during work.
    EnableMaintenanceMode,
    /// AMS NBI: `disableMaintenanceMode` -- resume normal alarming.
    DisableMaintenanceMode,
    /// AMS NBI: `executeAction` -- run a test action on the NE.
    ExecuteNeAction,
    // --- BxE portal (Hikari field track): portal workflows from
    // `docs/reference/tds/bxe/`. The portal is slow, so the fast path
    // matters as much as the right path. ---
    /// BxE: search the portal by street address (the slow GUI path).
    SearchAddress,
    /// BxE: list the ONT/router inventory on the customer account.
    ListDevices,
    /// BxE: pull live diagnostics -- light levels, errors, logs.
    /// Also the direct-API shortcut when the device ID is cached.
    ReadDiagnostics,
    /// BxE: trigger a remote speed test against the ONT.
    RunSpeedTest,
    /// BxE: certify the install -- all tests passed, record it.
    CertifyInstall,
}

impl ApiOp {
    /// Short label for the console button.
    pub fn label(self) -> &'static str {
        match self {
            ApiOp::Login => "Login",
            ApiOp::ShowOnt => "Show ONT",
            ApiOp::CreateService => "Create Service",
            ApiOp::UpdateService => "Update Service",
            ApiOp::RebootOnt => "Reboot ONT",
            ApiOp::VerifyService => "Verify Service",
            ApiOp::Logout => "Logout",
            ApiOp::CreateSubscriber => "Create Subscriber",
            ApiOp::CreateOnt => "Create ONT",
            ApiOp::DeleteSubscriber => "Delete Subscriber",
            ApiOp::GetAllManagedElements => "List NEs",
            ApiOp::GetManagedElement => "Get NE",
            ApiOp::StartSupervision => "Start Supervision",
            ApiOp::StopSupervision => "Stop Supervision",
            ApiOp::GetNeStatus => "Get Status",
            ApiOp::EnableMaintenanceMode => "Maint On",
            ApiOp::DisableMaintenanceMode => "Maint Off",
            ApiOp::ExecuteNeAction => "Execute Action",
            ApiOp::SearchAddress => "Search Address",
            ApiOp::ListDevices => "List Devices",
            ApiOp::ReadDiagnostics => "Read Diagnostics",
            ApiOp::RunSpeedTest => "Speed Test",
            ApiOp::CertifyInstall => "Certify",
        }
    }

    /// One-line tech description shown on the console.
    pub fn blurb(self) -> &'static str {
        match self {
            ApiOp::Login => "Authenticate to the management plane",
            ApiOp::ShowOnt => "Read the ONT's current services",
            ApiOp::CreateService => "Provision the service record",
            ApiOp::UpdateService => "Merge changes into the record",
            ApiOp::RebootOnt => "Bounce the ONT",
            ApiOp::VerifyService => "Read back and confirm",
            ApiOp::Logout => "Close the session",
            ApiOp::CreateSubscriber => "POST /ems/subscriber",
            ApiOp::CreateOnt => "Pre-provision the ONT",
            ApiOp::DeleteSubscriber => "DELETE the subscriber",
            ApiOp::GetAllManagedElements => "List every supervised NE",
            ApiOp::GetManagedElement => "Pull one NE record",
            ApiOp::StartSupervision => "Begin polling the NE",
            ApiOp::StopSupervision => "Pause polling before maintenance",
            ApiOp::GetNeStatus => "Read the NE alarm state",
            ApiOp::EnableMaintenanceMode => "Silence alarms during work",
            ApiOp::DisableMaintenanceMode => "Resume normal alarming",
            ApiOp::ExecuteNeAction => "Run a test action on the NE",
            ApiOp::SearchAddress => "Find the customer by street address",
            ApiOp::ListDevices => "Show ONT/router on the account",
            ApiOp::ReadDiagnostics => "Pull light levels, errors, logs",
            ApiOp::RunSpeedTest => "Run a remote speed test",
            ApiOp::CertifyInstall => "Certify: all tests passed",
        }
    }
}

/// An API-driver puzzle: the player must issue the `expected` calls in
/// order, picking from `choices` (which include distractors). A wrong
/// pick raises an alarm (counter + console feedback) but never fails
/// the level outright — the puzzle is about learning the sequence, not
/// punishing exploration.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiSequenceDef {
    /// Which API is being driven: "nbi" (CMS/AMS SOAP/XML), "smx"
    /// (REST/JSON), or "bxe" (field portal workflow).
    pub api: String,
    /// The correct call order.
    pub expected: Vec<ApiOp>,
    /// Buttons shown on the console. Must contain every expected op
    /// (plus distractors); validated in tests.
    pub choices: Vec<ApiOp>,
}

/// Verify an API call sequence: exact ordered match against expected.
/// Vacuous truth on two empties; any wrong op, wrong order, or
/// length mismatch is false. Fails closed on empty expected with
/// non-empty placed (a level-authoring bug, not a pass).
pub fn verify_api_sequence(expected: &[ApiOp], placed: &[ApiOp]) -> bool {
    !expected.is_empty() && expected == placed
}

/// NOC alarm severity for Aino's triage levels. Mirrors
/// `osp_sim::AlarmSeverity` ranks: Critical outranks everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AlarmSeverityDef {
    Critical,
    Major,
    Minor,
    Warning,
}

/// One alarm on Aino's NOC board: an id, a severity, and a one-line
/// summary the triage console shows on its ack button.
#[derive(Debug, Clone, Deserialize)]
pub struct AlarmDef {
    /// Stable within the level; the triage console keys buttons by it.
    pub id: u8,
    pub severity: AlarmSeverityDef,
    pub summary: String,
}

/// Alarm-triage puzzle (Aino's NOC track): when present, the player must
/// acknowledge the listed alarms through the triage console in exactly
/// `expected_order` (highest priority first) before the level counts as
/// won. The board win check still applies underneath.
#[derive(Debug, Clone, Deserialize)]
pub struct AlarmTriageDef {
    /// Alarms raised when the level starts.
    pub alarms: Vec<AlarmDef>,
    /// Correct ack order: alarm ids, highest priority first. A wrong
    /// pick bumps the console's wrong-pick counter but never fails the
    /// level -- triage teaches priority, it does not punish exploration.
    pub expected_order: Vec<u8>,
}

/// Verify an alarm ack order: exact ordered match against expected.
/// Same fail-closed semantics as `verify_api_sequence`.
pub fn verify_triage_order(expected: &[u8], acked: &[u8]) -> bool {
    !expected.is_empty() && expected == acked
}

/// Which NEC/article domain a quiz question belongs to. Drives Léa's
/// per-domain scoring and the "study more" recommendations on the
/// results screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum QuizDomain {
    /// NEC Articles 90, 100, 110 — general requirements, definitions.
    GeneralRequirements,
    /// NEC Article 250 — grounding and bonding.
    GroundingBonding,
    /// NEC Articles 300-398 — wiring methods.
    WiringMethods,
    /// NEC Articles 500-516 — hazardous (classified) locations.
    HazardousLocations,
    /// NEC Articles 705-780 — special conditions.
    SpecialConditions,
    /// NEC Articles 800-830 — communications systems (Léa's home turf).
    CommsSystems,
    /// Theory and calculations (Ohm's law, power, dB math).
    Theory,
    /// Washington State law: RCW 19.28 + WAC 296-46B.
    WashingtonLaw,
}

/// One quiz question in Léa's track. Deserialized from the level JSON;
/// the `answer` string from the source quiz data is resolved to
/// `correct_idx` at import time (see `tools/import_quiz.py`).
#[derive(Debug, Clone, Deserialize)]
pub struct QuizQuestion {
    /// Stable id, e.g. "nec250-001".
    pub id: String,
    /// The question text.
    pub prompt: String,
    /// Exactly four choices, in display order.
    pub choices: [String; 4],
    /// Index into `choices` of the correct answer (pre-shuffle).
    pub correct_idx: usize,
    /// Léa's teaching moment, shown after the player answers.
    pub explanation: String,
    /// NEC article reference for lookup, e.g. "NEC 250.4(A)(1)".
    pub article_ref: Option<String>,
    /// Which domain this question belongs to.
    pub domain: QuizDomain,
}

/// A quiz level's question set (Léa's track). When a `LevelDef` carries
/// `Some(quiz)`, the board is unused and the quiz UI owns the level:
/// answer every question, score at or above `pass_pct`, and the level
/// counts as won.
#[derive(Debug, Clone, Deserialize)]
pub struct QuizDef {
    /// Questions in presentation order.
    pub questions: Vec<QuizQuestion>,
    /// Fraction of questions that must be answered correctly to pass.
    /// Defaults to 0.70 (the 70% exam standard).
    #[serde(default = "default_quiz_pass_pct")]
    pub pass_pct: f32,
    /// Optional per-question time limit in seconds. `None` = untimed.
    #[serde(default)]
    pub time_limit_s: Option<u64>,
}

fn default_quiz_pass_pct() -> f32 {
    0.70
}

/// Score a completed quiz: `(correct, total)`. `answers[i]` is the
/// player's chosen index for `questions[i]`; missing answers count as
/// wrong. Fail-closed: length mismatch never panics.
pub fn score_quiz(questions: &[QuizQuestion], answers: &[usize]) -> (usize, usize) {
    let total = questions.len();
    let correct = questions
        .iter()
        .zip(answers.iter())
        .filter(|(q, a)| **a == q.correct_idx)
        .count();
    (correct, total)
}

/// True when the player's answers meet the quiz's pass threshold.
/// Empty question sets never pass (a level-authoring bug, not a win).
pub fn quiz_passed(quiz: &QuizDef, answers: &[usize]) -> bool {
    if quiz.questions.is_empty() {
        return false;
    }
    let (correct, total) = score_quiz(&quiz.questions, answers);
    total > 0 && (correct as f32 / total as f32) >= quiz.pass_pct
}

/// Per-subscriber result of provisioning verification.
#[derive(Debug, Clone)]
pub struct ProvisioningResult {
    pub subscriber: String,
    pub profile: &'static str,
    pub rx_dbm: f64,
    pub in_window: bool,
}

/// Verify a provisioning plan: every subscriber's received power must land
/// inside its profile's window.
///
/// `rx = tx_dbm - splitter_loss_db - fiber_db_per_km * distance_km - outage_loss_db`
///
/// Contract: returns one result per subscriber, in order. Non-finite or
/// negative distances, unknown profiles, and non-finite losses fail closed
/// (`in_window: false`) rather than silently passing.
pub fn verify_provisioning(
    tx_dbm: f64,
    splitter_loss_db: f64,
    fiber_db_per_km: f64,
    outage_loss_db: f64,
    subscribers: &[SubscriberDef],
) -> Vec<ProvisioningResult> {
    let losses_finite = tx_dbm.is_finite()
        && splitter_loss_db.is_finite()
        && fiber_db_per_km.is_finite()
        && outage_loss_db.is_finite()
        && fiber_db_per_km >= 0.0
        && outage_loss_db >= 0.0;
    subscribers
        .iter()
        .map(|sub| {
            let profile = service_profile(&sub.profile);
            let distance_ok = sub.distance_km.is_finite() && sub.distance_km >= 0.0;
            let (rx_dbm, in_window) = match (profile, losses_finite && distance_ok) {
                (Some(p), true) => {
                    let rx = tx_dbm
                        - splitter_loss_db
                        - fiber_db_per_km * sub.distance_km
                        - outage_loss_db;
                    let ok =
                        rx.is_finite() && rx >= p.min_rx_dbm as f64 && rx <= p.max_rx_dbm as f64;
                    (rx, ok)
                }
                _ => (f64::NAN, false),
            };
            ProvisioningResult {
                subscriber: sub.name.clone(),
                profile: profile.map(|p| p.name).unwrap_or("unknown"),
                rx_dbm,
                in_window,
            }
        })
        .collect()
}

/// A degraded fixed plant piece (Astra §2c, the C3 anchor): while
/// this edge's defect is in place, the coax noise floor an evaluation
/// runs against rises by `floor_penalty_dbmv` — game-side, in
/// `LevelDef::coax_noise_floor_with_defect`, so no amp choice can
/// out-gain it (gain raises the carrier and the penalty together).
/// The fix is substitution: the player swaps in a known-good spare
/// through the jumper console; the evaluator never learns any of it.
#[derive(Debug, Clone, Deserialize)]
pub struct DefectiveEdgeDef {
    /// The fixed edge carrying the degraded jumper.
    pub from: u32,
    pub to: u32,
    /// Floor penalty in dBmV while the defect is in place. Authored
    /// large enough that no available gain clears it (proven by the
    /// exhaustive sweep test in the jumper module).
    pub floor_penalty_dbmv: f64,
    /// Work-order label of the known-good spare in the kit.
    pub spare_label: String,
}

/// One survey test point (Astra §2d): a named area whose live RSSI is
/// read by running `evaluate_wireless` from the source to `node` over
/// the shared placed graph. Historical readings are authored
/// constants displayed beside the live ones — data, never simulated.
#[derive(Debug, Clone, Deserialize)]
pub struct SurveyPointDef {
    /// Stable id, e.g. "stockroom" (used in Coverage state ids).
    pub id: String,
    /// Display label, e.g. "Stockroom".
    pub label: String,
    /// Board node the point's reading is taken at.
    pub node: u32,
    /// Extra authored loss (dB) applied to the point's RSSI only —
    /// models an obstruction the node graph does not. SNR is never
    /// adjusted: level and noise stay independent (§3 guardrail 2).
    #[serde(default)]
    pub extra_loss_db: f64,
    /// The point's acceptance requirement, in dBm.
    pub required_rssi_dbm: f64,
    /// Optional per-point SNR requirement; unset uses the level's
    /// effective minimum.
    #[serde(default)]
    pub required_snr_db: Option<f64>,
    /// The previous survey's RSSI at this point (authored constant).
    pub historical_rssi_dbm: f64,
    /// The previous survey's SNR at this point, where recorded.
    #[serde(default)]
    pub historical_snr_db: Option<f64>,
}

/// The diagnosis pick on a survey level (Astra §2d, the W2 anchor):
/// the player names what moved — RSSI, noise, or plant — and the pick
/// is scored against the authored truth. `trap_option` is the named
/// trap (e.g. "wrong channel"); picking it fires the DiagnosisWrong
/// reaction and denies the Diagnosis badge.
#[derive(Debug, Clone, Deserialize)]
pub struct DiagnosisDef {
    pub options: Vec<String>,
    /// Index into `options` of the correct cause.
    pub correct: usize,
    /// Index of the trap option, when one is authored.
    #[serde(default)]
    pub trap_option: Option<usize>,
}

/// Multi-point survey block (Astra §2d). When present, per-point
/// Coverage states join the verification vector, and (on the anchor
/// levels) the win gate requires every point to pass.
#[derive(Debug, Clone, Deserialize)]
pub struct SurveyDef {
    pub points: Vec<SurveyPointDef>,
    #[serde(default)]
    pub diagnosis: Option<DiagnosisDef>,
    /// Anchor levels (m1l9, m1l10) require every point to pass for
    /// the win; elsewhere the survey is display + badge evidence.
    #[serde(default)]
    pub required_for_win: bool,
}

/// Cable identification block (Astra §2b): the closet holds several
/// candidate homeruns; exactly one is the work order's service run.
/// Exactly one *other* candidate carries the misleading handwritten
/// label (the trap). Identification is a state machine over this
/// authored data, not a simulated mapper instrument.
#[derive(Debug, Clone, Deserialize)]
pub struct IdentificationDef {
    pub candidates: Vec<IdentificationCandidate>,
    /// The mapper ID the correct run must read.
    pub remote_id: String,
    /// The work order's recorded port for the service run.
    pub expected_port: String,
}

/// One candidate homerun in the closet.
#[derive(Debug, Clone, Deserialize)]
pub struct IdentificationCandidate {
    /// Stable id, e.g. "run-c".
    pub id: String,
    /// Printed closet label, e.g. "Port 3".
    pub closet_label: String,
    /// The handwritten label, when one is taped to this run.
    #[serde(default)]
    pub handwritten_label: Option<String>,
    /// True for exactly one candidate: the work order's service run.
    pub is_service_run: bool,
    /// What the cable mapper reads when this run is tested.
    pub mapper_id: String,
}

/// Well-formedness of an identification block (fail-closed): exactly
/// one service run whose mapper reads the expected remote id, and
/// the handwritten label — when present — sits on a different run.
/// Malformed authored data must never make Identity passable.
pub fn identification_is_well_formed(def: &IdentificationDef) -> bool {
    let service: Vec<&IdentificationCandidate> =
        def.candidates.iter().filter(|c| c.is_service_run).collect();
    if service.len() != 1 {
        return false;
    }
    let service = service[0];
    if service.mapper_id != def.remote_id {
        return false;
    }
    let labelled: Vec<&IdentificationCandidate> = def
        .candidates
        .iter()
        .filter(|c| c.handwritten_label.is_some())
        .collect();
    labelled.len() <= 1 && labelled.iter().all(|c| !c.is_service_run)
}

/// Connector workbench block (Astra §2e): an ordered sequence console
/// in the `api_console` idiom. The instruction card (`card_lines`)
/// carries every dimension/seating reference the steps test — card
/// data only, never a global constant (§3 guardrail 1).
#[derive(Debug, Clone, Deserialize)]
pub struct WorkbenchDef {
    /// Instruction-card title, e.g. "RG-6 compression termination".
    pub card_title: String,
    /// The card's printed lines (dimensions, references, torque).
    pub card_lines: Vec<String>,
    /// The ordered steps; the sequence runs once per cable end.
    pub steps: Vec<WorkbenchStepDef>,
    /// How many ends the player terminates (both ends = 2).
    #[serde(default = "default_workbench_ends")]
    pub ends: u8,
    /// Authored loss (dB) an uncaught defect adds to the terminated
    /// span, applied game-side in the verification states until the
    /// end is rebuilt.
    pub workmanship_loss_db: f64,
}

fn default_workbench_ends() -> u8 {
    2
}

/// One workbench step: pick the instruction-card action among
/// `options`; `correct` is the card's answer. A wrong pick at a
/// `critical` step leaves an uncaught defect in the finished end; a
/// wrong pick anywhere else is caught at inspection/test and costs
/// a wasted connector.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkbenchStepDef {
    pub prompt: String,
    pub options: Vec<String>,
    /// Index into `options` of the correct action.
    pub correct: usize,
    /// The named failure mode a wrong pick records, verbatim from
    /// the report's table (e.g. "Braid contacts the center conductor.").
    pub failure_mode: String,
    /// Critical steps are the ones whose mistake survives into the
    /// finished termination (shielding, seating, compression,
    /// inspection, continuity test).
    #[serde(default)]
    pub critical: bool,
}

/// Static address family for a config block (Astra §2f). A level may
/// carry one block per family; families never share a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ConfigFamily {
    V4,
    V6,
}

/// One inventory line: an address and who holds it.
#[derive(Debug, Clone, Deserialize)]
pub struct AllocationDef {
    pub address: String,
    pub holder: String,
}

/// Static IPv4/IPv6 configuration block (Astra §2f): configure the
/// device from the worksheet by choosing candidate values per field
/// (no free-text entry — the console idiom, and it keeps the
/// adversarial surface testable). `initial` is the fault-injected
/// starting configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StaticConfigDef {
    pub family: ConfigFamily,
    /// The device being configured, e.g. "Office printer".
    pub device_label: String,
    pub worksheet_address: String,
    pub worksheet_prefix_len: u32,
    pub worksheet_gateway: String,
    /// V6 only: the interface the link-local gateway is scoped to.
    #[serde(default)]
    pub worksheet_gateway_interface: Option<String>,
    pub worksheet_dns: String,
    /// Interfaces the gateway may be scoped to (V6).
    #[serde(default)]
    pub interfaces: Vec<String>,
    /// V4 only: the DHCP pool bounds — displayed, and *never* the
    /// availability rule (guardrail: availability is inventory only).
    #[serde(default)]
    pub dhcp_pool: Option<(String, String)>,
    /// The address inventory: who holds what.
    #[serde(default)]
    pub allocations: Vec<AllocationDef>,
    pub initial_address: String,
    pub initial_prefix_len: u32,
    pub initial_gateway: String,
    #[serde(default)]
    pub initial_gateway_interface: Option<String>,
    pub initial_dns: String,
    /// Candidate values offered per field; each list must contain
    /// the worksheet value plus the authored traps.
    pub address_candidates: Vec<String>,
    pub prefix_candidates: Vec<u32>,
    pub gateway_candidates: Vec<String>,
    pub dns_candidates: Vec<String>,
}

/// Address-availability rule (Astra §2f guardrail): a candidate is
/// rejected iff it appears in the inventory under another holder or
/// collides with the gateway — never for being in or out of the
/// DHCP pool, and never from a reachability probe.
pub fn address_availability(def: &StaticConfigDef, address: &str) -> Result<(), &'static str> {
    if address == def.worksheet_gateway {
        return Err("address collides with the gateway");
    }
    if def
        .allocations
        .iter()
        .any(|a| a.address == address && a.holder != def.device_label)
    {
        return Err("address is allocated to another device");
    }
    Ok(())
}

/// Verdict of one config Apply, per family: every field must match
/// the worksheet and the address must be available. Returns the
/// first failing field's feedback string (report §6 wording), or
/// `Ok(())` when the family passes in full.
#[allow(clippy::too_many_arguments)]
pub fn config_apply_verdict(
    def: &StaticConfigDef,
    address: &str,
    prefix_len: u32,
    gateway: &str,
    gateway_interface: Option<&str>,
    dns: &str,
) -> Result<(), String> {
    if let Err(why) = address_availability(def, address) {
        return Err(format!(
            "Address {address} is not available: {why} — availability comes from the inventory, not the DHCP pool."
        ));
    }
    if address != def.worksheet_address {
        return Err(format!(
            "{} address does not match the assigned worksheet ({}).",
            match def.family {
                ConfigFamily::V4 => "IPv4",
                ConfigFamily::V6 => "IPv6",
            },
            def.worksheet_address
        ));
    }
    if prefix_len != def.worksheet_prefix_len {
        return Err(format!(
            "Prefix /{prefix_len} does not match the worksheet (/{}) — a close mask is still a wrong mask.",
            def.worksheet_prefix_len
        ));
    }
    if gateway != def.worksheet_gateway {
        return Err(format!(
            "{} gateway does not match the assigned worksheet ({}).",
            match def.family {
                ConfigFamily::V4 => "IPv4",
                ConfigFamily::V6 => "IPv6",
            },
            def.worksheet_gateway
        ));
    }
    if def.worksheet_gateway_interface.as_deref() != gateway_interface {
        return Err("IPv6 default route uses the wrong outgoing interface.".to_string());
    }
    if dns != def.worksheet_dns {
        return Err(format!(
            "DNS server does not match the assigned worksheet ({}).",
            def.worksheet_dns
        ));
    }
    Ok(())
}

/// The intermittent-connection block (Astra §2b/§2e composition,
/// c1l6): the drop drops when the cabinet is disturbed. The player
/// reproduces the drop, isolates the loose fitting among the path's
/// connections, tightens it to the card reference, and re-verifies
/// with a post-repair disturb that must hold.
#[derive(Debug, Clone, Deserialize)]
pub struct IntermittentDef {
    /// The path's connections; exactly one is loose.
    pub connections: Vec<IntermittentConnectionDef>,
    /// The instruction-card tightening reference (card data only).
    pub repair_reference: String,
    /// The open edge (from, to) of the backup path — building it
    /// instead of repairing is a valid clear that forfeits the
    /// Diagnosis badge.
    pub backup_edge: (u32, u32),
}

/// One connection on the intermittent path.
#[derive(Debug, Clone, Deserialize)]
pub struct IntermittentConnectionDef {
    pub id: String,
    pub label: String,
    /// True for exactly one connection: the loose fitting.
    pub is_loose: bool,
}

/// Well-formedness of an intermittent block (fail-closed): exactly
/// one loose connection.
pub fn intermittent_is_well_formed(def: &IntermittentDef) -> bool {
    def.connections.iter().filter(|c| c.is_loose).count() == 1
}

/// Which optional-objective a mapped level pays Cores for (Astra
/// §2g: badges pay nothing; optional objectives pay). Evaluated from
/// attempt telemetry at results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ObjectiveKind {
    /// Identification completed with zero neighbor disruptions.
    ZeroNeighborDisruptions,
    /// Workbench completed with zero wasted connectors.
    ZeroWastedConnectors,
    /// c1l6 resolved by repair, not by reroute.
    RepairNotReroute,
    /// No wrong selections anywhere (identification/diagnosis/config).
    ZeroWrongSelections,
}

/// An optional objective on a mapped level: a briefing-level goal
/// that pays `cores` on a win when its condition holds.
#[derive(Debug, Clone, Deserialize)]
pub struct OptionalObjectiveDef {
    pub description: String,
    pub cores: u32,
    pub kind: ObjectiveKind,
}

#[derive(Debug, Clone, Deserialize, Resource)]
pub struct LevelDef {
    pub id: String,
    pub title: String,
    pub world: u32,
    /// Flavor/briefing text shown before the level starts.
    pub briefing: String,
    pub tx_dbm: f64,
    pub wavelength: WavelengthDef,
    pub window_min_dbm: f64,
    pub window_max_dbm: f64,
    /// Which transmission medium this level teaches. Required — no silent
    /// default, so a level that forgets it fails to parse loudly instead
    /// of simulating the wrong physics.
    pub medium: MediumDef,
    /// Ethernet only: the powered device at the target (e.g. a 25 W
    /// access point). dB-medium levels leave this unset.
    #[serde(default)]
    pub endpoint_poe_draw_w: Option<f64>,
    /// Ethernet only: minimum end-to-end bandwidth in Mbps.
    #[serde(default)]
    pub required_bandwidth_mbps: Option<u64>,
    /// Ethernet only: longest legal unbroken copper segment in meters
    /// (TIA-568 says 90 m + 10 m patch; levels use 100 m).
    #[serde(default)]
    pub max_segment_length_m: Option<f64>,
    /// Coax only: the plant's noise floor in dBmV before any ingress
    /// hazard raises it. Unset falls back to the quiet-plant default in
    /// `coax_params`.
    #[serde(default)]
    pub coax_noise_floor_dbmv: Option<f64>,
    /// Coax only: minimum carrier-to-noise ratio (dB) the tap must
    /// hold. Unset falls back to the default in `coax_params`.
    #[serde(default)]
    pub min_carrier_to_noise_db: Option<f64>,
    /// Wireless only: minimum carried SNR (dB) the link must hold end
    /// to end. Unset falls back to the default in `wireless_params`.
    #[serde(default)]
    pub min_snr_db: Option<f64>,
    pub nodes: Vec<LevelNode>,
    /// Edges already placed for the player (fixed plant they don't route).
    pub fixed_edges: Vec<LevelEdge>,
    /// Component choices the player may place between open node pairs.
    pub available_components: Vec<ComponentChoice>,
    pub source_node: u32,
    pub target_node: u32,
    pub scripted_outage: Option<ScriptedOutage>,
    /// Provisioning levels (Clara's track): the ONTs to turn up. Empty for
    /// classic single-path levels. When non-empty, the win check verifies
    /// every subscriber against its demanded profile instead of the
    /// source→target link budget.
    #[serde(default)]
    pub subscribers: Vec<SubscriberDef>,
    /// API-driver puzzle (Clara's NBI/SMx levels): when present, the
    /// player must also issue the expected call sequence through the
    /// API console before the level counts as won. The board win check
    /// still applies underneath.
    #[serde(default)]
    pub api_sequence: Option<ApiSequenceDef>,
    /// Alarm-triage puzzle (Aino's NOC track): when present, the player
    /// must also ack the alarms in the expected priority order through
    /// the triage console before the level counts as won.
    #[serde(default)]
    pub alarm_triage: Option<AlarmTriageDef>,
    /// Quiz level (Léa's study track): when present, the board is unused
    /// and the quiz UI owns the win condition. See `QuizDef`.
    #[serde(default)]
    pub quiz: Option<QuizDef>,
    /// Cable identification (Astra §2b): when present, the Identity
    /// state joins the win gate (except on the intermittent level,
    /// where the §2e composition supersedes it — see `intermittent`).
    #[serde(default)]
    pub identification: Option<IdentificationDef>,
    /// Defective fixed edge (Astra §2c): while the defect is in
    /// place, the Service (CNR) state is measured against a raised
    /// floor. See `DefectiveEdgeDef`.
    #[serde(default)]
    pub defective_edge: Option<DefectiveEdgeDef>,
    /// Multi-point survey + historical diagnosis (Astra §2d).
    #[serde(default)]
    pub survey: Option<SurveyDef>,
    /// Connector workbench (Astra §2e).
    #[serde(default)]
    pub workbench: Option<WorkbenchDef>,
    /// Static IPv4 configuration (Astra §2f). Per-family blocks:
    /// a level may carry either or both; families never share a
    /// verdict.
    #[serde(default)]
    pub static_config_v4: Option<StaticConfigDef>,
    /// Static IPv6 configuration (Astra §2f).
    #[serde(default)]
    pub static_config_v6: Option<StaticConfigDef>,
    /// Intermittent-connection composition (Astra §2b/§2e, c1l6).
    #[serde(default)]
    pub intermittent: Option<IntermittentDef>,
    /// Capstone Ethernet handoff gate (Astra §2a/§1.1 c1l10): after
    /// the coax states pass, the player verifies the Ethernet
    /// handoff before the level counts as won.
    #[serde(default)]
    pub handoff_required: bool,
    /// Optional objective that pays Cores on a win (Astra §2g).
    #[serde(default)]
    pub optional_objective: Option<OptionalObjectiveDef>,
    /// Optional dialogue hook keys fired on enter/win/fail — looked up in
    /// the Séraphine dialogue bank.
    pub on_enter_line: Option<String>,
    pub on_win_line: Option<String>,
    pub on_fail_line: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum WavelengthDef {
    Nm1310,
    Nm1490,
    Nm1550,
}

impl From<WavelengthDef> for Wavelength {
    fn from(w: WavelengthDef) -> Self {
        match w {
            WavelengthDef::Nm1310 => Wavelength::Nm1310,
            WavelengthDef::Nm1490 => Wavelength::Nm1490,
            WavelengthDef::Nm1550 => Wavelength::Nm1550,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LevelNode {
    pub id: u32,
    pub label: String,
    pub grid_x: f32,
    pub grid_y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LevelEdge {
    pub from: u32,
    pub to: u32,
    pub component: Component,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComponentChoice {
    pub from: u32,
    pub to: u32,
    pub component: Component,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScriptedOutage {
    pub fires_after_seconds: f64,
    pub kind: OutageKindDef,
    pub edge_from: u32,
    pub edge_to: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum OutageKindDef {
    FiberCut,
    AerialDamage,
    WaterIntrusion,
    ConnectorContamination,
    Macrobend,
    AmplifierFailure,
    IngressNoise,
    WirelessInterference,
}

impl From<OutageKindDef> for OutageKind {
    fn from(k: OutageKindDef) -> Self {
        match k {
            OutageKindDef::FiberCut => OutageKind::FiberCut,
            OutageKindDef::AerialDamage => OutageKind::AerialDamage,
            OutageKindDef::WaterIntrusion => OutageKind::WaterIntrusion,
            OutageKindDef::ConnectorContamination => OutageKind::ConnectorContamination,
            OutageKindDef::Macrobend => OutageKind::Macrobend,
            OutageKindDef::AmplifierFailure => OutageKind::AmplifierFailure,
            OutageKindDef::IngressNoise => OutageKind::IngressNoise,
            OutageKindDef::WirelessInterference => OutageKind::WirelessInterference,
        }
    }
}

impl LevelDef {
    pub fn receive_window(&self) -> ReceiveWindow {
        ReceiveWindow {
            min_dbm: self.window_min_dbm,
            max_dbm: self.window_max_dbm,
        }
    }

    pub fn medium(&self) -> Medium {
        Medium::from(self.medium)
    }

    /// Unit label for this level's receive window ("dBm", "dBmV", or ""
    /// for Ethernet, which has no signal level).
    pub fn units_label(&self) -> &'static str {
        self.medium().units_label()
    }

    /// Ethernet constraint parameters with the standards defaults: 100 m
    /// max segment (TIA-568), 0 W PoE draw, 0 Mbps required bandwidth.
    fn ethernet_params(&self) -> (f64, f64, u64) {
        (
            self.max_segment_length_m.unwrap_or(100.0),
            self.endpoint_poe_draw_w.unwrap_or(0.0),
            self.required_bandwidth_mbps.unwrap_or(0),
        )
    }

    /// Coax evaluation parameters: (quiet-plant noise floor in dBmV,
    /// minimum carrier-to-noise ratio in dB). Defaults: a −35 dBmV
    /// floor (a quiet plant's thermal floor referred to the tap) and
    /// 25 dB CNR (the classic carrier-quality bar — below it the
    /// picture degrades no matter how balanced the levels are).
    fn coax_params(&self) -> (f64, f64) {
        (
            self.coax_noise_floor_dbmv.unwrap_or(-35.0),
            self.min_carrier_to_noise_db.unwrap_or(25.0),
        )
    }

    /// Wireless evaluation parameter: the minimum carried SNR (dB)
    /// the link must hold. Default 10 dB — the floor under a robust
    /// digital link; interference raises the *requirement* from here
    /// (see `effective_wireless_min_snr_db`).
    fn wireless_params(&self) -> f64 {
        self.min_snr_db.unwrap_or(10.0)
    }

    /// The coax noise floor an evaluation actually runs against: the
    /// level's quiet-plant floor, raised by an unresolved ingress
    /// hazard's accumulated floor rise. Ingress does not eat the
    /// carrier — it buries it — so the outage moves the floor, never
    /// the received level.
    fn effective_coax_noise_floor_dbmv(&self, outage: Option<&Outage>) -> f64 {
        let (floor, _) = self.coax_params();
        match outage {
            Some(o) if !o.resolved && o.kind == OutageKind::IngressNoise => {
                floor + o.accumulated_extra_loss_db()
            }
            _ => floor,
        }
    }

    /// The wireless SNR requirement an evaluation actually runs
    /// against: the level's minimum, raised by an unresolved
    /// interference hazard's accumulated floor rise. The evaluator
    /// measures carried SNR against a fixed thermal floor, so a
    /// floor that rose by N dB is a requirement that rose by N dB.
    fn effective_wireless_min_snr_db(&self, outage: Option<&Outage>) -> f64 {
        let base = self.wireless_params();
        match outage {
            Some(o) if !o.resolved && o.kind == OutageKind::WirelessInterference => {
                base + o.accumulated_extra_loss_db()
            }
            _ => base,
        }
    }

    /// Plain (non-outage) win check, dispatched on the level's medium.
    pub fn is_win_state(&self, graph: &PathGraph, tx_dbm: f64, wavelength: Wavelength) -> bool {
        self.is_win_state_with_outage(graph, tx_dbm, wavelength, None)
    }

    /// Outage-aware win check. Fiber degrade hazards add their
    /// accumulated extra loss on top of the budget; coax ingress and
    /// wireless interference instead raise the noise floor the signal
    /// is judged against (see the `effective_*` helpers) — the carrier
    /// level itself is untouched. Full cuts rely on the caller
    /// excluding the severed edge (see `board::rebuild_live_graph`),
    /// so they surface as an ordinary disconnect. Ethernet ignores
    /// the outage — its constraints don't degrade over time.
    pub fn is_win_state_with_outage(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
        outage: Option<&Outage>,
    ) -> bool {
        // Quiz levels (Léa's track) never win via the board: the quiz
        // UI owns the win condition through `quiz_passed`. Without this
        // guard an empty board could spuriously satisfy a budget check.
        if self.quiz.is_some() {
            return false;
        }
        // Provisioning levels (Clara's track) replace the single-path
        // budget check: every subscriber must land inside its demanded
        // profile's window behind the placed splitter.
        if !self.subscribers.is_empty() {
            return self.is_provisioning_win(graph, tx_dbm, wavelength, outage);
        }
        match self.medium() {
            Medium::Ethernet => {
                let (max_segment_m, poe_draw_w, required_bw) = self.ethernet_params();
                graph
                    .evaluate_ethernet(
                        self.source_node,
                        self.target_node,
                        max_segment_m,
                        poe_draw_w,
                        required_bw,
                    )
                    .map(|eval| eval.passes())
                    .unwrap_or(false)
            }
            Medium::Coax => {
                let (_, min_cnr_db) = self.coax_params();
                graph
                    .evaluate_coax(
                        self.source_node,
                        self.target_node,
                        tx_dbm,
                        self.receive_window(),
                        self.effective_coax_noise_floor_dbmv(outage),
                        min_cnr_db,
                    )
                    .map(|eval| eval.passes())
                    .unwrap_or(false)
            }
            Medium::Wireless => graph
                .evaluate_wireless(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    self.receive_window(),
                    self.effective_wireless_min_snr_db(outage),
                )
                .map(|eval| eval.passes())
                .unwrap_or(false),
            _ => graph
                .compute_link_budget_with_outage(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    wavelength,
                    self.receive_window(),
                    outage,
                )
                .map(|result| result.in_window)
                .unwrap_or(false),
        }
    }

    /// Provisioning win check (Clara's track): the player must have placed
    /// a splitter, it must have enough ports for every subscriber, and every
    /// subscriber's received power must land inside its demanded profile's
    /// window. Outage loss (if any) counts against the budget — a fault on
    /// the feeder eats everyone's headroom.
    fn is_provisioning_win(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
        outage: Option<&Outage>,
    ) -> bool {
        // The split plan lives in the placed components: find the splitter.
        // No splitter placed yet means no plan to verify.
        let ratio = graph.edges.iter().find_map(|e| match &e.component {
            Component::Splitter { ratio } => Some(ratio),
            _ => None,
        });
        let Some(ratio) = ratio else {
            return false;
        };
        // Port scarcity is a hard constraint: a 1:8 splitter cannot serve
        // 9 subscribers, no matter how clean the budget is.
        if self.subscribers.len() as u32 > ratio.branch_count() {
            return false;
        }
        let outage_loss = outage.map(|o| o.accumulated_extra_loss_db()).unwrap_or(0.0);
        verify_provisioning(
            tx_dbm,
            ratio.insertion_loss_db(),
            wavelength.attenuation_db_per_km(),
            outage_loss,
            &self.subscribers,
        )
        .iter()
        .all(|r| r.in_window)
    }

    /// Ledger line for a route that is not complete yet: teaches the
    /// two placement gestures before anything is placed, then reports
    /// the running level at the frontier of the connected chain so the
    /// player can steer toward the receive window while building. A
    /// live degrade outage's extra loss is applied exactly as the
    /// complete-path budget applies it.
    fn partial_ledger(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
        outage: Option<&Outage>,
    ) -> String {
        let window = self.receive_window();
        let units = self.units_label();
        let (mut level, hops, _frontier) =
            graph.frontier_budget(self.source_node, tx_dbm, wavelength);
        if hops == 0 {
            return format!(
                "Nothing placed yet — tap a pill to place a component, or drag node to node to connect. Window: {:.1} to {:.1} {}",
                window.min_dbm, window.max_dbm, units
            );
        }
        if let Some(outage) = outage {
            if !outage.resolved && !outage.kind.is_full_cut() {
                level -= outage.accumulated_extra_loss_db();
            }
        }
        format!(
            "Placed: {} {}  |  Level so far: {:.2} {}  |  Window: {:.1} to {:.1} {} — route not complete",
            hops,
            if hops == 1 { "hop" } else { "hops" },
            level,
            units,
            window.min_dbm,
            window.max_dbm,
            units
        )
    }

    /// One-line ledger summary for the results screen and the live
    /// ledger, medium-aware: dB media get the loss/Rx/margin readout with
    /// the medium's unit label, Ethernet gets the constraint checklist.
    pub fn signal_ledger(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
        outage: Option<&Outage>,
    ) -> String {
        match self.medium() {
            Medium::Ethernet => {
                let (max_segment_m, poe_draw_w, required_bw) = self.ethernet_params();
                match graph.evaluate_ethernet(
                    self.source_node,
                    self.target_node,
                    max_segment_m,
                    poe_draw_w,
                    required_bw,
                ) {
                    Ok(eval) => {
                        let mut parts = vec![
                            format!(
                                "Longest segment: {:.0}m / {:.0}m max",
                                eval.longest_segment_m, max_segment_m
                            ),
                            format!(
                                "PoE: {:.0}W draw / {:.0}W budget",
                                eval.poe_draw_w, eval.poe_budget_w
                            ),
                            format!("Bandwidth: {} Mbps", eval.min_bandwidth_mbps),
                        ];
                        if eval.passes() {
                            parts.push("WITHIN SPEC".to_string());
                        } else {
                            let why: Vec<String> = eval
                                .violations
                                .iter()
                                .map(|v| match v {
                                    EthernetViolation::SegmentTooLong { longest_m, limit_m } => {
                                        format!("segment {longest_m:.0}m > {limit_m:.0}m max")
                                    }
                                    EthernetViolation::PoeOverBudget { draw_w, budget_w } => {
                                        format!("PoE {draw_w:.0}W > {budget_w:.0}W budget")
                                    }
                                    EthernetViolation::BandwidthTooLow {
                                        min_mbps,
                                        required_mbps,
                                    } => {
                                        format!("{min_mbps} Mbps < {required_mbps} Mbps required")
                                    }
                                    EthernetViolation::NonFiniteInput { field } => {
                                        format!("non-finite input for {field}")
                                    }
                                    EthernetViolation::UnsupportedComponent { detail, .. } => {
                                        detail.clone()
                                    }
                                })
                                .collect();
                            parts.push(format!("VIOLATION: {}", why.join("; ")));
                        }
                        parts.join("  |  ")
                    }
                    Err(_) => "Link disconnected — no route completed.".to_string(),
                }
            }
            Medium::Coax => {
                let (_, min_cnr_db) = self.coax_params();
                let floor = self.effective_coax_noise_floor_dbmv(outage);
                match graph.evaluate_coax(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    self.receive_window(),
                    floor,
                    min_cnr_db,
                ) {
                    Ok(eval) => {
                        let mut parts = vec![
                            format!("Rx: {:.2} dBmV", eval.received_dbmv),
                            format!(
                                "CNR: {:.1} dB (floor {:.0} dBmV) / {:.0} dB min",
                                eval.carrier_to_noise_db, floor, min_cnr_db
                            ),
                            window_verdict(eval.received_dbmv, self.receive_window()).to_string(),
                            cnr_verdict(eval.carrier_to_noise_db, min_cnr_db).to_string(),
                        ];
                        let extra: Vec<String> = eval
                            .violations
                            .iter()
                            .filter_map(|v| match v {
                                CoaxViolation::NonFiniteInput { field } => {
                                    Some(format!("non-finite input for {field}"))
                                }
                                CoaxViolation::UnsupportedComponent { detail, .. } => {
                                    Some(detail.clone())
                                }
                                _ => None,
                            })
                            .collect();
                        if !extra.is_empty() {
                            parts.push(format!("VIOLATION: {}", extra.join("; ")));
                        }
                        parts.join("  |  ")
                    }
                    Err(_) => self.partial_ledger(graph, tx_dbm, wavelength, outage),
                }
            }
            Medium::Wireless => {
                let required_snr_db = self.effective_wireless_min_snr_db(outage);
                match graph.evaluate_wireless(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    self.receive_window(),
                    required_snr_db,
                ) {
                    Ok(eval) => {
                        let mut parts = vec![
                            format!("Rx: {:.2} dBm", eval.received_dbm),
                            format!("SNR: {:.1} dB / {:.0} dB min", eval.snr_db, required_snr_db),
                            format!("Fade margin: {:.1} dB", eval.fade_margin_db),
                            window_verdict(eval.received_dbm, self.receive_window()).to_string(),
                            snr_verdict(eval.snr_db, required_snr_db).to_string(),
                        ];
                        let extra: Vec<String> = eval
                            .violations
                            .iter()
                            .filter_map(|v| match v {
                                WirelessViolation::NonFiniteInput { field } => {
                                    Some(format!("non-finite input for {field}"))
                                }
                                WirelessViolation::UnsupportedComponent { detail, .. } => {
                                    Some(detail.clone())
                                }
                                _ => None,
                            })
                            .collect();
                        if !extra.is_empty() {
                            parts.push(format!("VIOLATION: {}", extra.join("; ")));
                        }
                        parts.join("  |  ")
                    }
                    Err(_) => self.partial_ledger(graph, tx_dbm, wavelength, outage),
                }
            }
            _ => {
                let units = self.units_label();
                match graph.compute_link_budget_with_outage(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    wavelength,
                    self.receive_window(),
                    outage,
                ) {
                    Ok(result) => format!(
                        "Loss: {:.2} dB  |  Rx: {:.2} {}  |  Margin: {:.2} dB  |  {}",
                        result.total_loss_db,
                        result.received_dbm,
                        units,
                        result.margin_db,
                        window_verdict(result.received_dbm, self.receive_window()),
                    ),
                    Err(_) => self.partial_ledger(graph, tx_dbm, wavelength, outage),
                }
            }
        }
    }

    /// Independent verification states for this attempt (Astra §2a):
    /// the board's verdict split into per-state lines, each with its
    /// own status and one-line feedback. Pure derivation from the
    /// evaluators' outputs — no `osp_sim` change — so the live ledger
    /// and the results screen render the same vector and can never
    /// disagree about *why* a level stands where it stands.
    ///
    /// `mechanics` carries the states owned by the optional mechanic
    /// blocks (identity, workbench, config, …) as `(status, feedback)`
    /// pairs assembled by the caller from the live progress resources;
    /// a `None` entry means the mechanic is not in play on this level
    /// and its line is omitted. `service_defect_present` is the
    /// defective-jumper flag (§2c): while set, the Service state is
    /// measured against the defect-raised floor.
    pub fn verification_states(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
        outage: Option<&Outage>,
        mechanics: &MechanicStates,
    ) -> Vec<StateLine> {
        let mut lines: Vec<StateLine> = Vec::new();
        match self.medium() {
            Medium::Coax => {
                let (_, min_cnr_db) = self.coax_params();
                let floor = self.coax_noise_floor_with_defect(outage, mechanics);
                match graph.evaluate_coax(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    self.receive_window(),
                    floor,
                    min_cnr_db,
                ) {
                    Ok(eval) => {
                        lines.push(StateLine::new(
                            StateId::Continuity,
                            StateStatus::Pass,
                            "Continuity passes — the path resolves end to end.".to_string(),
                        ));
                        // §2e: a live workmanship defect attenuates
                        // the measured carrier (CNR is unaffected —
                        // carrier and ingress drop together).
                        let measured_dbmv = eval.received_dbmv - mechanics.workmanship_loss_db;
                        let in_window = self.receive_window().contains(measured_dbmv);
                        lines.push(StateLine::new(
                            StateId::CarrierLevel,
                            if in_window {
                                StateStatus::Pass
                            } else {
                                StateStatus::Fail
                            },
                            format!(
                                "Carrier level {:.1} dBmV vs window [{:.0}, {:.0}] dBmV.",
                                measured_dbmv, self.window_min_dbm, self.window_max_dbm
                            ),
                        ));
                        let cnr_ok = eval.carrier_to_noise_db >= min_cnr_db;
                        let feedback = if cnr_ok {
                            format!(
                                "CNR {:.1} dB clears the {:.0} dB service requirement.",
                                eval.carrier_to_noise_db, min_cnr_db
                            )
                        } else if in_window {
                            format!(
                                "Continuity passes, but service acceptance still fails: CNR {:.1} dB vs {:.0} dB required.",
                                eval.carrier_to_noise_db, min_cnr_db
                            )
                        } else {
                            format!(
                                "CNR {:.1} dB vs {:.0} dB required.",
                                eval.carrier_to_noise_db, min_cnr_db
                            )
                        };
                        lines.push(StateLine::new(
                            StateId::ServiceCnr,
                            if cnr_ok {
                                StateStatus::Pass
                            } else {
                                StateStatus::Fail
                            },
                            feedback,
                        ));
                    }
                    Err(_) => {
                        lines.push(StateLine::new(
                            StateId::Continuity,
                            StateStatus::Fail,
                            "Continuity fails — no complete path to the drop.".to_string(),
                        ));
                        lines.push(StateLine::new(
                            StateId::CarrierLevel,
                            StateStatus::Pending,
                            "No carrier to measure until the path is complete.".to_string(),
                        ));
                        lines.push(StateLine::new(
                            StateId::ServiceCnr,
                            StateStatus::Pending,
                            "Service cannot be accepted on an open path.".to_string(),
                        ));
                    }
                }
            }
            Medium::Wireless => {
                let required_snr_db = self.effective_wireless_min_snr_db(outage);
                match graph.evaluate_wireless(
                    self.source_node,
                    self.target_node,
                    tx_dbm,
                    self.receive_window(),
                    required_snr_db,
                ) {
                    Ok(eval) => {
                        lines.push(StateLine::new(
                            StateId::Continuity,
                            StateStatus::Pass,
                            "Link closes end to end.".to_string(),
                        ));
                        let in_window = self.receive_window().contains(eval.received_dbm);
                        lines.push(StateLine::new(
                            StateId::LinkLevel,
                            if in_window {
                                StateStatus::Pass
                            } else {
                                StateStatus::Fail
                            },
                            format!(
                                "RSSI {:.1} dBm vs window [{:.0}, {:.0}] dBm (fade margin {:.1} dB).",
                                eval.received_dbm,
                                self.window_min_dbm,
                                self.window_max_dbm,
                                eval.fade_margin_db
                            ),
                        ));
                        let snr_ok = eval.snr_db >= required_snr_db;
                        lines.push(StateLine::new(
                            StateId::LinkSnr,
                            if snr_ok {
                                StateStatus::Pass
                            } else {
                                StateStatus::Fail
                            },
                            if snr_ok {
                                format!(
                                    "SNR {:.1} dB clears the {:.0} dB requirement.",
                                    eval.snr_db, required_snr_db
                                )
                            } else if in_window {
                                format!(
                                    "Loud is not clear: level is in window but SNR {:.1} dB vs {:.0} dB required.",
                                    eval.snr_db, required_snr_db
                                )
                            } else {
                                format!(
                                    "SNR {:.1} dB vs {:.0} dB required.",
                                    eval.snr_db, required_snr_db
                                )
                            },
                        ));
                        let interference = outage.is_some_and(|o| {
                            !o.resolved && o.kind == OutageKind::WirelessInterference
                        });
                        lines.push(StateLine::new(
                            StateId::NoiseChannel,
                            if !interference {
                                StateStatus::NotApplicable
                            } else if snr_ok {
                                StateStatus::Pass
                            } else {
                                StateStatus::Fail
                            },
                            if interference {
                                format!(
                                    "Interference is active: the SNR requirement has risen to {:.0} dB.",
                                    required_snr_db
                                )
                            } else {
                                "No interference event on this link.".to_string()
                            },
                        ));
                        // Per-point survey states (§2d) ride the same
                        // vector: one Coverage line per authored point.
                        if let Some(survey) = &self.survey {
                            for point in &survey.points {
                                match self.survey_point_reading(graph, tx_dbm, outage, point) {
                                    Some((rssi_dbm, snr_db)) => {
                                        let snr_req =
                                            point.required_snr_db.unwrap_or(required_snr_db);
                                        let ok = rssi_dbm >= point.required_rssi_dbm
                                            && snr_db >= snr_req;
                                        lines.push(StateLine::new(
                                            StateId::Coverage(point.id.clone()),
                                            if ok {
                                                StateStatus::Pass
                                            } else {
                                                StateStatus::Fail
                                            },
                                            if ok {
                                                format!(
                                                    "Coverage ({}): {:.1} dBm clears the {:.0} dBm requirement.",
                                                    point.label, rssi_dbm, point.required_rssi_dbm
                                                )
                                            } else {
                                                format!(
                                                    "Coverage ({}): {:.1} dBm vs {:.0} dBm required — this point remains below this mission's requirement.",
                                                    point.label, rssi_dbm, point.required_rssi_dbm
                                                )
                                            },
                                        ));
                                    }
                                    None => {
                                        lines.push(StateLine::new(
                                            StateId::Coverage(point.id.clone()),
                                            StateStatus::Pending,
                                            format!(
                                                "Coverage ({}): no reading yet — the path to this point is open.",
                                                point.label
                                            ),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        lines.push(StateLine::new(
                            StateId::Continuity,
                            StateStatus::Fail,
                            "Link does not close — no complete path.".to_string(),
                        ));
                        lines.push(StateLine::new(
                            StateId::LinkLevel,
                            StateStatus::Pending,
                            "No RSSI to measure until the link closes.".to_string(),
                        ));
                        lines.push(StateLine::new(
                            StateId::LinkSnr,
                            StateStatus::Pending,
                            "SNR cannot be carried on an open link.".to_string(),
                        ));
                    }
                }
            }
            Medium::Ethernet => {
                let (max_segment_m, poe_draw_w, required_bw) = self.ethernet_params();
                match graph.evaluate_ethernet(
                    self.source_node,
                    self.target_node,
                    max_segment_m,
                    poe_draw_w,
                    required_bw,
                ) {
                    Ok(eval) => {
                        lines.push(StateLine::new(
                            StateId::Continuity,
                            StateStatus::Pass,
                            "Run is continuous end to end.".to_string(),
                        ));
                        lines.push(StateLine::new(
                            StateId::Application,
                            if eval.passes() {
                                StateStatus::Pass
                            } else {
                                StateStatus::Fail
                            },
                            if eval.passes() {
                                "All Ethernet constraints hold (segment, PoE, bandwidth)."
                                    .to_string()
                            } else {
                                format!(
                                    "{} constraint violation(s) on this run.",
                                    eval.violations.len()
                                )
                            },
                        ));
                    }
                    Err(_) => {
                        lines.push(StateLine::new(
                            StateId::Continuity,
                            StateStatus::Fail,
                            "Run is open — no complete path.".to_string(),
                        ));
                        lines.push(StateLine::new(
                            StateId::Application,
                            StateStatus::Pending,
                            "Constraints cannot be checked on an open run.".to_string(),
                        ));
                    }
                }
            }
            _ => {
                // Fiber (and the Study fallback): the optical budget's
                // verdicts rendered through the same vector.
                if self.quiz.is_some() {
                    // Quiz levels own no board states at all.
                } else {
                    match graph.compute_link_budget_with_outage(
                        self.source_node,
                        self.target_node,
                        tx_dbm,
                        wavelength,
                        self.receive_window(),
                        outage,
                    ) {
                        Ok(result) => {
                            lines.push(StateLine::new(
                                StateId::Continuity,
                                StateStatus::Pass,
                                "Path resolves end to end.".to_string(),
                            ));
                            lines.push(StateLine::new(
                                StateId::CarrierLevel,
                                if result.in_window {
                                    StateStatus::Pass
                                } else {
                                    StateStatus::Fail
                                },
                                format!(
                                    "Received {:.1} {} vs window [{:.0}, {:.0}] {}.",
                                    result.received_dbm,
                                    self.units_label(),
                                    self.window_min_dbm,
                                    self.window_max_dbm,
                                    self.units_label()
                                ),
                            ));
                        }
                        Err(_) => {
                            lines.push(StateLine::new(
                                StateId::Continuity,
                                StateStatus::Fail,
                                "Path is open — no complete route.".to_string(),
                            ));
                            lines.push(StateLine::new(
                                StateId::CarrierLevel,
                                StateStatus::Pending,
                                "No reading until the route is complete.".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        // Mechanic-owned states merge into the same vector (§2a: one
        // source, two surfaces). Order is fixed so the ledger and the
        // results screen always read identically.
        let mechanic_lines = [
            (StateId::Identity, &mechanics.identity),
            (StateId::Inspection, &mechanics.inspection),
            (StateId::Workmanship, &mechanics.workmanship),
            (StateId::Ipv4Config, &mechanics.ipv4),
            (StateId::Ipv6Config, &mechanics.ipv6),
            (StateId::Dns, &mechanics.dns),
            (StateId::Application, &mechanics.application),
            (StateId::Handoff, &mechanics.handoff),
            (StateId::Documentation, &mechanics.documentation),
        ];
        for (id, entry) in mechanic_lines {
            if let Some((status, feedback)) = entry {
                // The Ethernet arm already emitted an Application line;
                // a mechanic Application entry only joins media that
                // did not.
                if id == StateId::Application && lines.iter().any(|l| l.id == StateId::Application)
                {
                    continue;
                }
                lines.push(StateLine::new(id, *status, feedback.clone()));
            }
        }
        lines
    }

    /// The coax noise floor an evaluation actually runs against,
    /// including the defective-jumper penalty (§2c) while the defect
    /// is in place. The clean path (defect swapped out, or no defect
    /// authored) is exactly `effective_coax_noise_floor_dbmv` — the
    /// pinned c1l5 arithmetic (28.5 dB at full accrual) is measured
    /// on it and must never move.
    pub fn coax_noise_floor_with_defect(
        &self,
        outage: Option<&Outage>,
        mechanics: &MechanicStates,
    ) -> f64 {
        let floor = self.effective_coax_noise_floor_dbmv(outage);
        match &self.defective_edge {
            Some(defect) if mechanics.service_defect_present => floor + defect.floor_penalty_dbmv,
            _ => floor,
        }
    }

    /// True while this level's defective edge (§2c) is in the live
    /// plant: a defect is authored and the known-good spare has not
    /// been swapped in.
    pub fn defect_present(&self, spare_swapped: bool) -> bool {
        self.defective_edge.is_some() && !spare_swapped
    }

    /// One survey point's live reading (§2d): `evaluate_wireless`
    /// from the source to the point's node over the shared placed
    /// graph (the evaluator is pure; N points = N calls), with the
    /// point's authored extra loss (an obstruction the graph does not
    /// model) applied to RSSI only — SNR is returned as carried, so a
    /// point's level and noise states stay independent. `None` when
    /// the path to the point does not resolve.
    pub fn survey_point_reading(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        outage: Option<&Outage>,
        point: &SurveyPointDef,
    ) -> Option<(f64, f64)> {
        let eval = graph
            .evaluate_wireless(
                self.source_node,
                point.node,
                tx_dbm,
                self.receive_window(),
                self.effective_wireless_min_snr_db(outage),
            )
            .ok()?;
        Some((eval.received_dbm - point.extra_loss_db, eval.snr_db))
    }

    /// True when every authored survey point currently passes its
    /// RSSI (and SNR, where authored) requirement. Levels without a
    /// survey block pass vacuously. Fail-closed: an unresolvable
    /// point is a failing point.
    pub fn survey_points_pass(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        outage: Option<&Outage>,
    ) -> bool {
        let Some(survey) = &self.survey else {
            return true;
        };
        if survey.points.is_empty() {
            return false;
        }
        let required_snr_db = self.effective_wireless_min_snr_db(outage);
        survey.points.iter().all(|point| {
            match self.survey_point_reading(graph, tx_dbm, outage, point) {
                Some((rssi_dbm, snr_db)) => {
                    rssi_dbm >= point.required_rssi_dbm
                        && snr_db >= point.required_snr_db.unwrap_or(required_snr_db)
                }
                None => false,
            }
        })
    }
}

/// One independent verification state (Astra §2a). States are derived,
/// never stored as truth: the same vector renders on the live ledger
/// and the results screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateStatus {
    Pending,
    Pass,
    Fail,
    NotApplicable,
}

impl StateStatus {
    /// Ledger verdict word for this status.
    pub fn word(self) -> &'static str {
        match self {
            StateStatus::Pending => "PENDING",
            StateStatus::Pass => "PASS",
            StateStatus::Fail => "FAIL",
            StateStatus::NotApplicable => "N/A",
        }
    }
}

/// Identity of one verification state. `Coverage` carries the survey
/// point's authored id — points are data, not a fixed enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateId {
    Continuity,
    CarrierLevel,
    ServiceCnr,
    LinkLevel,
    LinkSnr,
    Coverage(String),
    NoiseChannel,
    Identity,
    Inspection,
    Workmanship,
    Handoff,
    Ipv4Config,
    Ipv6Config,
    Dns,
    Application,
    Documentation,
}

impl StateId {
    /// Display label for ledger/results rendering.
    pub fn label(&self) -> String {
        match self {
            StateId::Continuity => "Continuity".to_string(),
            StateId::CarrierLevel => "Carrier Level".to_string(),
            StateId::ServiceCnr => "Service (CNR)".to_string(),
            StateId::LinkLevel => "Link Level".to_string(),
            StateId::LinkSnr => "Link SNR".to_string(),
            StateId::Coverage(point_id) => format!("Coverage ({point_id})"),
            StateId::NoiseChannel => "Noise Channel".to_string(),
            StateId::Identity => "Identity".to_string(),
            StateId::Inspection => "Inspection".to_string(),
            StateId::Workmanship => "Workmanship".to_string(),
            StateId::Handoff => "Handoff".to_string(),
            StateId::Ipv4Config => "IPv4 Config".to_string(),
            StateId::Ipv6Config => "IPv6 Config".to_string(),
            StateId::Dns => "DNS".to_string(),
            StateId::Application => "Application".to_string(),
            StateId::Documentation => "Documentation".to_string(),
        }
    }
}

/// One rendered verification-state line: the state, its status, and
/// its own one-line feedback (measured value + requirement, never a
/// bare "failed").
#[derive(Debug, Clone, PartialEq)]
pub struct StateLine {
    pub id: StateId,
    pub status: StateStatus,
    pub feedback: String,
}

impl StateLine {
    pub fn new(id: StateId, status: StateStatus, feedback: String) -> Self {
        Self {
            id,
            status,
            feedback,
        }
    }
}

/// Mechanic-owned verification states, assembled by the caller from
/// the live progress resources (see `crate::astra`). Each entry is a
/// `(status, feedback)` pair; `None` omits the line entirely.
#[derive(Debug, Clone, Default)]
pub struct MechanicStates {
    pub identity: Option<(StateStatus, String)>,
    pub inspection: Option<(StateStatus, String)>,
    pub workmanship: Option<(StateStatus, String)>,
    pub handoff: Option<(StateStatus, String)>,
    pub ipv4: Option<(StateStatus, String)>,
    pub ipv6: Option<(StateStatus, String)>,
    pub dns: Option<(StateStatus, String)>,
    pub application: Option<(StateStatus, String)>,
    pub documentation: Option<(StateStatus, String)>,
    /// §2c: the defective jumper is still in the live plant.
    pub service_defect_present: bool,
    /// §2e: authored loss (dB) a live workmanship defect applies to
    /// the measured carrier level (level states only — the defect
    /// attenuates carrier and ingress together, so CNR is unchanged).
    pub workmanship_loss_db: f64,
}

/// One-line summary of a state vector for compact surfaces (the live
/// ledger): `Continuity PASS · Service (CNR) FAIL · …`. NotApplicable
/// lines are omitted — they carry no signal at ledger width.
pub fn states_summary(lines: &[StateLine]) -> String {
    lines
        .iter()
        .filter(|l| l.status != StateStatus::NotApplicable)
        .map(|l| format!("{} {}", l.id.label(), l.status.word()))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The feedback strings of every failing state, in vector order —
/// the results screen's per-state explanation block.
pub fn states_failures(lines: &[StateLine]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| l.status == StateStatus::Fail)
        .map(|l| l.feedback.clone())
        .collect()
}

/// Look up one state's line in a vector.
pub fn state_line<'a>(lines: &'a [StateLine], id: &StateId) -> Option<&'a StateLine> {
    lines.iter().find(|l| &l.id == id)
}

/// Verdict word for a completed route: inside the receive window, or
/// the side of it the received level missed on — a bare miss message
/// leaves the player guessing whether to add gain or add loss.
fn window_verdict(received_dbm: f64, window: ReceiveWindow) -> &'static str {
    if window.contains(received_dbm) {
        "IN WINDOW"
    } else if received_dbm < window.min_dbm {
        "TOO LOW"
    } else {
        "TOO HOT"
    }
}

/// Verdict word for a carrier-to-noise ratio against its requirement:
/// the coax failure mode a level-only readout hides — the levels can
/// be perfectly balanced while the carrier drowns in a risen floor.
fn cnr_verdict(cnr_db: f64, required_db: f64) -> &'static str {
    if cnr_db >= required_db {
        "CNR OK"
    } else {
        "CNR TOO LOW"
    }
}

/// Verdict word for a carried SNR against its requirement — the
/// wireless counterpart of `cnr_verdict`.
fn snr_verdict(snr_db: f64, required_db: f64) -> &'static str {
    if snr_db >= required_db {
        "SNR OK"
    } else {
        "SNR TOO LOW"
    }
}

/// Level JSON is embedded at compile time (rather than loaded through
/// Bevy's `AssetServer`) so the same code path works identically on
/// desktop and Android without needing APK asset-file access at runtime.
pub const LEVEL_SOURCES: &[&str] = &[
    include_str!("../assets/levels/world1_level1.json"),
    include_str!("../assets/levels/fiber3_clean_hands.json"),
    include_str!("../assets/levels/fiber4_fusion_or_bust.json"),
    include_str!("../assets/levels/fiber5_the_short_cut.json"),
    include_str!("../assets/levels/fiber6_split_decision.json"),
    include_str!("../assets/levels/fiber7_bend_dont_break.json"),
    include_str!("../assets/levels/fiber8_contamination_event.json"),
    include_str!("../assets/levels/world4_level1_outage.json"),
    include_str!("../assets/levels/fiber9_tight_budget.json"),
    include_str!("../assets/levels/fiber10_build_it_right.json"),
    include_str!("../assets/levels/coax1_unity_gain.json"),
    include_str!("../assets/levels/coax2_ingress.json"),
    include_str!("../assets/levels/coax3_longer_run.json"),
    include_str!("../assets/levels/coax4_tap_dance.json"),
    include_str!("../assets/levels/coax5_ingress_returns.json"),
    include_str!("../assets/levels/coax6_backup_plan.json"),
    include_str!("../assets/levels/coax7_hot_headend.json"),
    include_str!("../assets/levels/coax8_the_long_cascade.json"),
    include_str!("../assets/levels/coax9_precision_run.json"),
    include_str!("../assets/levels/coax10_ingress_storm.json"),
    include_str!("../assets/levels/wireless1_close_the_link.json"),
    include_str!("../assets/levels/wireless2_ride_the_storm.json"),
    include_str!("../assets/levels/wireless3_thread_the_needle.json"),
    include_str!("../assets/levels/wireless4_the_5ghz_tax.json"),
    include_str!("../assets/levels/wireless5_after_the_repeater.json"),
    include_str!("../assets/levels/wireless6_loud_is_not_clear.json"),
    include_str!("../assets/levels/wireless7_the_long_haul.json"),
    include_str!("../assets/levels/wireless8_aim_high.json"),
    include_str!("../assets/levels/wireless9_mixed_bands.json"),
    include_str!("../assets/levels/wireless10_linkas_gauntlet.json"),
    include_str!("../assets/levels/ethernet1_hundred_meter_wall.json"),
    include_str!("../assets/levels/ethernet2_power_budget.json"),
    include_str!("../assets/levels/ethernet3_category_matters.json"),
    include_str!("../assets/levels/ethernet4_the_long_corridor.json"),
    include_str!("../assets/levels/ethernet5_power_hungry.json"),
    include_str!("../assets/levels/ethernet6_every_constraint.json"),
    include_str!("../assets/levels/ethernet7_the_campus.json"),
    include_str!("../assets/levels/ethernet8_no_slack.json"),
    include_str!("../assets/levels/ethernet9_retrofit.json"),
    include_str!("../assets/levels/ethernet10_the_data_center.json"),
    include_str!("../assets/levels/clara1_provisioning.json"),
    include_str!("../assets/levels/clara2_outage.json"),
    include_str!("../assets/levels/clara3_dead_box_swap.json"),
    include_str!("../assets/levels/clara4_profile_audit.json"),
    include_str!("../assets/levels/clara5_drive_the_nbi.json"),
    include_str!("../assets/levels/clara6_bulk_turn_up.json"),
    include_str!("../assets/levels/clara7_smx_lifecycle.json"),
    include_str!("../assets/levels/clara8_building_turn_up.json"),
    include_str!("../assets/levels/clara9_tight_budget.json"),
    include_str!("../assets/levels/clara10_night_cutover.json"),
    include_str!("../assets/levels/aino1_first_shift.json"),
    include_str!("../assets/levels/aino2_triage_order.json"),
    include_str!("../assets/levels/aino3_the_cascade.json"),
    include_str!("../assets/levels/aino4_bulk_ack.json"),
    include_str!("../assets/levels/aino5_query_the_nbi.json"),
    include_str!("../assets/levels/aino6_supervision.json"),
    include_str!("../assets/levels/aino7_night_shift.json"),
    include_str!("../assets/levels/aino8_maintenance_window.json"),
    include_str!("../assets/levels/aino9_soap_fault.json"),
    include_str!("../assets/levels/aino10_all_clear.json"),
    include_str!("../assets/levels/hikari1_first_day.json"),
    include_str!("../assets/levels/hikari2_address_to_diagnostics.json"),
    include_str!("../assets/levels/hikari3_beat_the_callback.json"),
    include_str!("../assets/levels/hikari4_dirty_drop.json"),
    include_str!("../assets/levels/hikari5_certify_it.json"),
    include_str!("../assets/levels/hikari6_slow_portal.json"),
    include_str!("../assets/levels/hikari7_bend_in_the_wall.json"),
    include_str!("../assets/levels/hikari8_splitter_closet.json"),
    include_str!("../assets/levels/hikari9_night_trouble.json"),
    include_str!("../assets/levels/hikari10_master_tech.json"),
    include_str!("../assets/levels/lea1_tutorial.json"),
    include_str!("../assets/levels/lea2_grounding.json"),
    include_str!("../assets/levels/lea3_wiring_methods.json"),
    include_str!("../assets/levels/lea4_hazloc.json"),
    include_str!("../assets/levels/lea5_special_conditions.json"),
    include_str!("../assets/levels/lea6_comms_systems.json"),
    include_str!("../assets/levels/lea7_theory.json"),
    include_str!("../assets/levels/lea8_wa_law.json"),
    include_str!("../assets/levels/lea9_wa_admin.json"),
    include_str!("../assets/levels/lea10_mock_exam.json"),
];

/// Which bundled level is currently active. Advance this (e.g. from the
/// Results screen) to move to the next level; `setup_level` reads it on
/// every `OnEnter(Playing)`.
#[derive(Resource, Clone, Copy, Default)]
pub struct CurrentLevelIndex(pub usize);

pub fn load_level(index: usize) -> LevelDef {
    let idx = index.min(LEVEL_SOURCES.len() - 1);
    serde_json::from_str(LEVEL_SOURCES[idx]).expect("bundled level JSON must always parse")
}

#[cfg(test)]
mod tests {
    use super::*;

    // `load_level`'s `.expect("bundled level JSON must always parse")` is
    // only safe because every entry in `LEVEL_SOURCES` is verified here at
    // test time -- without this, a JSON typo in a new level file would
    // silently compile and only panic the first time a player reached
    // that level index at runtime.
    #[test]
    fn every_bundled_level_parses_and_has_a_usable_shape() {
        for (idx, _) in LEVEL_SOURCES.iter().enumerate() {
            let level = load_level(idx);
            assert!(!level.id.is_empty(), "level {idx} has an empty id");
            // Quiz levels never use the board -- the quiz UI owns their
            // win condition -- so board-shape assertions don't apply.
            if level.quiz.is_some() {
                continue;
            }
            assert!(!level.nodes.is_empty(), "level {idx} has no nodes");
            assert!(
                level.window_min_dbm < level.window_max_dbm,
                "level {idx} has an inverted receive window"
            );
            let node_ids: std::collections::HashSet<u32> =
                level.nodes.iter().map(|n| n.id).collect();
            assert!(
                node_ids.contains(&level.source_node),
                "level {idx}'s source_node isn't one of its declared nodes"
            );
            assert!(
                node_ids.contains(&level.target_node),
                "level {idx}'s target_node isn't one of its declared nodes"
            );
            for edge in level
                .fixed_edges
                .iter()
                .map(|e| (e.from, e.to))
                .chain(level.available_components.iter().map(|c| (c.from, c.to)))
            {
                assert!(
                    node_ids.contains(&edge.0) && node_ids.contains(&edge.1),
                    "level {idx} references an edge {edge:?} with an undeclared node"
                );
            }
        }
    }

    /// EXOS Registration IDs (docs/reference/tds/cms/guides/exos-provisioning.md):
    /// unique per ONT within a level, at most 10 chars, alphanumeric.
    /// The office pre-provisions them; a duplicate or malformed ID is a
    /// level-authoring bug that would collide in the field.
    #[test]
    fn subscriber_reg_ids_are_unique_and_well_formed() {
        for (idx, _) in LEVEL_SOURCES.iter().enumerate() {
            let level = load_level(idx);
            let mut seen = std::collections::HashSet::new();
            for sub in &level.subscribers {
                assert!(
                    !sub.reg_id.is_empty(),
                    "level {} subscriber '{}' has an empty reg_id",
                    idx,
                    sub.name
                );
                assert!(
                    sub.reg_id.len() <= 10,
                    "level {} reg_id '{}' exceeds 10 chars (EXOS limit)",
                    idx,
                    sub.reg_id
                );
                assert!(
                    sub.reg_id.chars().all(|c| c.is_ascii_alphanumeric()),
                    "level {} reg_id '{}' is not alphanumeric",
                    idx,
                    sub.reg_id
                );
                assert!(
                    seen.insert(sub.reg_id.clone()),
                    "level {} has duplicate reg_id '{}'",
                    idx,
                    sub.reg_id
                );
            }
        }
    }

    /// Every demanded profile must exist in the SERVICE_PROFILES catalog;
    /// a typo'd tier name fails the test, not the player at runtime.
    #[test]
    fn subscriber_profiles_all_resolve_in_catalog() {
        for (idx, _) in LEVEL_SOURCES.iter().enumerate() {
            let level = load_level(idx);
            for sub in &level.subscribers {
                assert!(
                    service_profile(&sub.profile).is_some(),
                    "level {} subscriber '{}' demands unknown profile '{}'",
                    idx,
                    sub.name,
                    sub.profile
                );
            }
        }
    }

    fn provisioning_sub(name: &str, profile: &str, distance_km: f64) -> SubscriberDef {
        SubscriberDef {
            name: name.into(),
            node: 0,
            profile: profile.into(),
            distance_km,
            reg_id: "TEST000001".into(),
        }
    }

    #[test]
    fn verify_provisioning_checks_each_subscriber_window() {
        // GPON OLT at +3 dBm, 1:8 splitter (10.6 dB), 1490 nm fiber.
        let subs = vec![
            provisioning_sub("near", "GPON-100", 2.0),
            provisioning_sub("far", "GPON-100", 12.0),
        ];
        let results = verify_provisioning(3.0, 10.6, 0.28, 0.0, &subs);
        assert_eq!(results.len(), 2);
        // near: 3 - 10.6 - 0.56 = -8.16 dBm — inside (-27, -8).
        assert!((results[0].rx_dbm - -8.16).abs() < 0.01);
        assert!(results[0].in_window);
        // far: 3 - 10.6 - 3.36 = -10.96 dBm — inside.
        assert!(results[1].in_window);
    }

    #[test]
    fn verify_provisioning_flags_overload_and_undershoot() {
        // Overload: hot OLT, tiny split, short drop.
        let hot = vec![provisioning_sub("hot", "GPON-100", 1.0)];
        let r = verify_provisioning(5.0, 7.3, 0.28, 0.0, &hot);
        // 5 - 7.3 - 0.28 = -2.58 dBm — above the -8 dBm ceiling.
        assert!(!r[0].in_window);

        // Undershoot: tired OLT, big split, long drop.
        let dim = vec![provisioning_sub("dim", "GPON-100", 20.0)];
        let r = verify_provisioning(-2.0, 17.7, 0.28, 0.0, &dim);
        // -2 - 17.7 - 5.6 = -25.3 dBm — inside (-27, -8), barely.
        assert!(r[0].in_window);
        let dimmer = vec![provisioning_sub("dimmer", "GPON-100", 25.0)];
        let r = verify_provisioning(-2.0, 17.7, 0.28, 0.0, &dimmer);
        // -2 - 17.7 - 7.0 = -26.7 dBm — inside.
        assert!(r[0].in_window);
        let dark = vec![provisioning_sub("dark", "GPON-100", 30.0)];
        let r = verify_provisioning(-2.0, 17.7, 0.28, 0.0, &dark);
        // -2 - 17.7 - 8.4 = -28.1 dBm — below the -27 dBm floor.
        assert!(!r[0].in_window);
    }

    #[test]
    fn verify_provisioning_fails_closed_on_bad_inputs() {
        let subs = vec![provisioning_sub("s", "GPON-100", 5.0)];
        // NaN loss fails closed.
        let r = verify_provisioning(3.0, f64::NAN, 0.28, 0.0, &subs);
        assert!(!r[0].in_window);
        // Negative distance fails closed.
        let bad = vec![provisioning_sub("s", "GPON-100", -1.0)];
        let r = verify_provisioning(3.0, 10.6, 0.28, 0.0, &bad);
        assert!(!r[0].in_window);
        // Unknown profile fails closed.
        let unknown = vec![provisioning_sub("s", "NOPE-999", 5.0)];
        let r = verify_provisioning(3.0, 10.6, 0.28, 0.0, &unknown);
        assert!(!r[0].in_window);
        assert_eq!(r[0].profile, "unknown");
        // Outage loss eats headroom.
        let r = verify_provisioning(3.0, 10.6, 0.28, 20.0, &subs);
        // 3 - 10.6 - 1.4 - 20 = -29 dBm — out.
        assert!(!r[0].in_window);
    }

    #[test]
    fn service_profile_catalog_has_expected_tiers() {
        let gpon = service_profile("GPON-100").expect("GPON-100 in catalog");
        assert_eq!(gpon.down_mbps, 100);
        assert_eq!(gpon.max_split_ratio, 32);
        let xgs = service_profile("XGS-1000").expect("XGS-1000 in catalog");
        assert!(xgs.max_rx_dbm < gpon.max_rx_dbm, "XGS window is tighter");
        assert!(service_profile("BOGUS").is_none());
    }

    #[test]
    fn load_level_clamps_an_out_of_range_index_to_the_last_level() {
        let clamped = load_level(usize::MAX);
        let last = load_level(LEVEL_SOURCES.len() - 1);
        assert_eq!(clamped.id, last.id);
    }

    /// Build the live graph for `level` with exactly one player-placed
    /// choice, mirroring `board::rebuild_live_graph`'s edge-exclusion
    /// rule: a full-cut outage's severed edge is absent while unresolved.
    fn graph_with_single_placement(level: &LevelDef, choice: &ComponentChoice) -> PathGraph {
        let severed = level.scripted_outage.as_ref().and_then(|s| {
            let kind = OutageKind::from(s.kind);
            kind.is_full_cut().then_some((s.edge_from, s.edge_to))
        });
        let mut graph = PathGraph::default();
        for node in &level.nodes {
            graph.add_node(node.id, node.label.clone());
        }
        for edge in &level.fixed_edges {
            if severed != Some((edge.from, edge.to)) {
                graph.connect(edge.from, edge.to, edge.component.clone());
            }
        }
        if severed != Some((choice.from, choice.to)) {
            graph.connect(choice.from, choice.to, choice.component.clone());
        }
        graph
    }

    fn fresh_outage(level: &LevelDef) -> Option<Outage> {
        level
            .scripted_outage
            .as_ref()
            .map(|s| Outage::new(OutageKind::from(s.kind), s.edge_from, s.edge_to))
    }

    // Every bundled level must be winnable, and — except the fiber
    // tutorial, where both splice types are valid — exactly one pill per
    // level may win. A level with zero winning pills soft-locks the
    // player; a level with two or more winning pills has no puzzle.
    // Exception: pure API-driver levels (clara5, clara7) offer no pills
    // at all — their puzzle is the console sequence, and the board link
    // ships pre-lit. They expect zero by design.
    #[test]
    fn every_level_has_exactly_its_intended_winning_pill_count() {
        // Clara's provisioning levels: clara1 has 3 splitter pills but only
        // the 1:16 has enough ports for 9 subscribers (1:4 and 1:8 fail the
        // port-count check); clara2 has 2 pills and only the 1:8 fits 6
        // subscribers (1:4 has 4 ports).
        let expected_wins = [
            2usize, 2, 1, 1, 1, 1, 2, 1, 1, 1, // Fiber 0-9
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // Coax 10-19
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // Mobile 20-29
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // Ethernet 30-39
            1, 1, 1, 1, 0, 1, 0, 1, 1, 1, // Clara 40-49
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Aino 50-59 (pure triage/API; pills span edges)
            2, 0, 2, 2, 1, 0, 1, 1, 2, 1, // Hikari 60-69
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Lea 70-79 (quiz levels never win via board)
        ];
        assert_eq!(
            LEVEL_SOURCES.len(),
            expected_wins.len(),
            "add the new level's expected win count here"
        );
        for (idx, expected) in expected_wins.iter().enumerate() {
            let level: LevelDef = serde_json::from_str(LEVEL_SOURCES[idx]).unwrap();
            let wavelength = Wavelength::from(level.wavelength);
            let outage = fresh_outage(&level);
            let wins = level
                .available_components
                .iter()
                .filter(|choice| {
                    let graph = graph_with_single_placement(&level, choice);
                    level.is_win_state_with_outage(
                        &graph,
                        level.tx_dbm,
                        wavelength,
                        outage.as_ref(),
                    )
                })
                .count();
            assert_eq!(
                wins, *expected,
                "level {} ({}) has {wins} winning pills, expected {expected}",
                idx, level.id
            );
        }
    }

    // The two storm levels under the specialty evaluators: ingress and
    // interference raise the noise floor (the CNR / SNR requirement),
    // never the carrier level — so the winning play is riding the
    // storm out on the pill that was balanced at dusk, NOT out-shouting
    // the floor. (The previous optical model subtracted the accrual
    // from the received level, which made the loudest pill the repair;
    // that is exactly the field mistake these levels now teach against.)
    #[test]
    fn storm_repair_pills_win_mid_storm() {
        let coax = load_level(11); // c1l2 in the 80-level layout
        let wavelength = Wavelength::from(coax.wavelength);
        let balanced_amp = coax
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 3.0))
            .expect("c1l2 must offer a 3 dB amp");
        let graph = graph_with_single_placement(&coax, balanced_amp);

        // 90 s in: floor has risen 9 dB (−35 → −26 dBmV). The 3 dB amp
        // still lands Rx 8 dBmV, and CNR 8 − (−26) = 34 ≥ 25.
        let mut ingress = fresh_outage(&coax).unwrap();
        ingress.tick(90.0);
        assert!(
            coax.is_win_state_with_outage(&graph, coax.tx_dbm, wavelength, Some(&ingress)),
            "3 dB amp must ride out c1l2 90 s into the ingress storm (CNR 34 ≥ 25)"
        );

        // Full accrual (12 dB cap): floor −23 dBmV, CNR 31 ≥ 25 — the
        // balanced pill survives the whole storm, not just its middle.
        let mut full_ingress = fresh_outage(&coax).unwrap();
        full_ingress.tick(600.0);
        assert_eq!(full_ingress.accumulated_extra_loss_db(), 12.0);
        assert!(
            coax.is_win_state_with_outage(&graph, coax.tx_dbm, wavelength, Some(&full_ingress)),
            "3 dB amp must survive c1l2 at full 12 dB ingress accrual (CNR 31 ≥ 25)"
        );

        // The old model's repair — the 12 dB amp — is now the trap:
        // Rx 17 dBmV overshoots the 15 dBmV window top. Out-shouting
        // a risen floor cooks the drop.
        let loud_amp = coax
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 12.0))
            .expect("c1l2 must offer a 12 dB amp");
        let loud_graph = graph_with_single_placement(&coax, loud_amp);
        assert!(
            !coax.is_win_state_with_outage(&loud_graph, coax.tx_dbm, wavelength, Some(&ingress)),
            "12 dB amp must NOT win c1l2 mid-storm (Rx 17 dBmV > 15 dBmV window)"
        );

        let wireless = load_level(21); // m1l2 in the 80-level layout
        let wavelength = Wavelength::from(wireless.wavelength);
        let balanced_rpt = wireless
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 20.0))
            .expect("m1l2 must offer a 20 dBm repeater");
        let graph = graph_with_single_placement(&wireless, balanced_rpt);

        // 80 s in: the SNR requirement has climbed 8 dB (10 → 18).
        // The 20 dBm repeater lands Rx −68.01 dBm with carried SNR
        // 26.99 — still clear of the raised bar.
        let mut interference = fresh_outage(&wireless).unwrap();
        interference.tick(80.0);
        assert!(
            wireless.is_win_state_with_outage(
                &graph,
                wireless.tx_dbm,
                wavelength,
                Some(&interference),
            ),
            "20 dBm repeater must ride out m1l2 80 s into the interference storm (SNR 26.99 ≥ 18)"
        );

        // And the loud swap is the trap here too: the 30 dBm repeater
        // lands Rx −58.02 dBm, above the −65 dBm window top — more
        // power never fixes a risen floor.
        let loud_rpt = wireless
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 30.0))
            .expect("m1l2 must offer a 30 dBm repeater");
        let loud_graph = graph_with_single_placement(&wireless, loud_rpt);
        assert!(
            !wireless.is_win_state_with_outage(
                &loud_graph,
                wireless.tx_dbm,
                wavelength,
                Some(&interference),
            ),
            "30 dBm repeater must NOT win m1l2 mid-storm (Rx −58.02 dBm > −65 dBm window)"
        );
    }

    /// All three ingress levels (c1l2, c1l5, c1l10) must stay winnable
    /// through the FULL 12 dB floor-rise accrual, with exactly the
    /// same single winning pill they have at rest — the coax2 caution
    /// from the wiring brief, proven per level: the binding case is
    /// c1l5, whose winner lands Rx 5.5 dBmV for a full-accrual CNR of
    /// 5.5 − (−35 + 12) = 28.5 ≥ 25.
    #[test]
    fn coax_ingress_levels_stay_winnable_through_full_accrual() {
        for idx in [11, 14, 19] {
            let level = load_level(idx);
            let wavelength = Wavelength::from(level.wavelength);
            let mut storm = fresh_outage(&level).unwrap();
            storm.tick(600.0); // far past the cap
            assert_eq!(
                storm.accumulated_extra_loss_db(),
                12.0,
                "level {} ({}) outage must cap at 12 dB",
                idx,
                level.id
            );
            let winners = level
                .available_components
                .iter()
                .filter(|choice| {
                    let graph = graph_with_single_placement(&level, choice);
                    level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, Some(&storm))
                })
                .count();
            assert_eq!(
                winners, 1,
                "level {} ({}) must keep exactly one winning pill at full ingress accrual",
                idx, level.id
            );
        }

        // The binding numbers, asserted directly against the evaluator:
        // c1l5's winner (5 dB amp) at full accrual.
        let level = load_level(14);
        let winner = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 5.0))
            .expect("c1l5 must offer a 5 dB amp");
        let graph = graph_with_single_placement(&level, winner);
        let mut storm = fresh_outage(&level).unwrap();
        storm.tick(600.0);
        let eval = graph
            .evaluate_coax(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                level.receive_window(),
                level.effective_coax_noise_floor_dbmv(Some(&storm)),
                level.coax_params().1,
            )
            .expect("c1l5 path must resolve");
        assert!(
            (eval.received_dbmv - 5.5).abs() < 0.01,
            "c1l5 winner Rx must be 5.5 dBmV, got {}",
            eval.received_dbmv
        );
        assert!(
            (eval.carrier_to_noise_db - 28.5).abs() < 0.01,
            "c1l5 winner CNR at full accrual must be 28.5 dB, got {}",
            eval.carrier_to_noise_db
        );
        assert!(eval.passes());
    }

    /// The remaining two binding cases for the specialty defaults,
    /// asserted numerically against the evaluators at rest: c1l8's
    /// winning cascade holds CNR 36 against the −35 dBmV quiet-plant
    /// floor (the 25 dB bar clears with headroom), and m1l10's
    /// winning repeater carries SNR 16.72 — comfortably above the
    /// 10 dB wireless default, so the level binds the default from
    /// the safe side: any default above 16.72 would soft-lock it.
    /// (Win/lose coverage for both levels also comes from
    /// `every_level_has_exactly_its_intended_winning_pill_count`.)
    #[test]
    fn specialty_binding_cases_hold_at_rest() {
        // c1l8 (index 17): exactly one winning pill, at CNR 36.
        let level = load_level(17);
        let wavelength = Wavelength::from(level.wavelength);
        let winners: Vec<&ComponentChoice> = level
            .available_components
            .iter()
            .filter(|choice| {
                let graph = graph_with_single_placement(&level, choice);
                level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None)
            })
            .collect();
        assert_eq!(
            winners.len(),
            1,
            "c1l8 must have exactly one winning pill, got {}",
            winners.len()
        );
        let graph = graph_with_single_placement(&level, winners[0]);
        let eval = graph
            .evaluate_coax(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                level.receive_window(),
                level.effective_coax_noise_floor_dbmv(None),
                level.coax_params().1,
            )
            .expect("c1l8 path must resolve");
        assert!(
            (eval.carrier_to_noise_db - 36.0).abs() < 0.1,
            "c1l8 winner CNR must be 36 dB, got {}",
            eval.carrier_to_noise_db
        );
        assert!(eval.passes());

        // m1l10 (index 29): exactly one winning pill, at SNR 16.72.
        let level = load_level(29);
        let wavelength = Wavelength::from(level.wavelength);
        let winners: Vec<&ComponentChoice> = level
            .available_components
            .iter()
            .filter(|choice| {
                let graph = graph_with_single_placement(&level, choice);
                level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None)
            })
            .collect();
        assert_eq!(
            winners.len(),
            1,
            "m1l10 must have exactly one winning pill, got {}",
            winners.len()
        );
        let graph = graph_with_single_placement(&level, winners[0]);
        let eval = graph
            .evaluate_wireless(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                level.receive_window(),
                level.wireless_params(),
            )
            .expect("m1l10 path must resolve");
        assert!(
            (eval.snr_db - 16.72).abs() < 0.05,
            "m1l10 winner SNR must be 16.72 dB, got {}",
            eval.snr_db
        );
        assert!(
            eval.snr_db >= level.wireless_params(),
            "m1l10 winner SNR must clear the 10 dB wireless default it binds"
        );
        assert!(eval.passes());
    }

    /// Adversarial: the specialty evaluators and the optical budget
    /// genuinely disagree, in both directions — which is why coax and
    /// wireless levels must not be scored by the optical engine.
    #[test]
    fn specialty_evaluators_disagree_with_the_optical_budget() {
        // Direction 1 — optical passes, coax evaluator fails on CNR
        // alone. c1l2 with a hostile plant floor (−10 dBmV, via the
        // level's own override field): the 3 dB amp lands Rx 8 dBmV,
        // dead center of the [0, 15] window, but CNR is 18 < 25.
        let mut level = load_level(11);
        level.coax_noise_floor_dbmv = Some(-10.0);
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 3.0))
            .expect("c1l2 must offer a 3 dB amp")
            .clone();
        let graph = graph_with_single_placement(&level, &amp);
        let wavelength = Wavelength::from(level.wavelength);
        let optical = graph
            .compute_link_budget_with_outage(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                wavelength,
                level.receive_window(),
                None,
            )
            .expect("c1l2 path must resolve");
        assert!(
            optical.in_window,
            "optical budget must pass the balanced c1l2 plant (Rx {:.2})",
            optical.received_dbm
        );
        assert!(
            !level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None),
            "coax evaluator must fail the same plant on CNR alone"
        );
        let eval = graph
            .evaluate_coax(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                level.receive_window(),
                level.effective_coax_noise_floor_dbmv(None),
                level.coax_params().1,
            )
            .expect("c1l2 path must resolve");
        assert_eq!(eval.violations.len(), 1, "CNR must be the ONLY violation");
        assert!(
            matches!(
                eval.violations[0],
                CoaxViolation::CarrierToNoiseTooLow { .. }
            ),
            "the sole violation must be CarrierToNoiseTooLow, got {:?}",
            eval.violations
        );

        // Direction 2 — optical passes, wireless evaluator fails on
        // SNR alone. A synthetic amp chain on m1l1's shape: a 1 km
        // first hop wrecks the carried SNR (reception at −80.05 dBm →
        // SNR 14.95), the 90 dB amp shouts the wreckage back into the
        // window over a short second hop — level fine, signal ruined.
        // (Under the optical budget an amplifier is free gain; the
        // wireless evaluator charges it 5 dB of noise figure.)
        let level = load_level(20); // m1l1: window [−75, −40]
        let mut graph = PathGraph::default();
        for node in &level.nodes {
            graph.add_node(node.id, node.label.clone());
        }
        graph.connect(
            0,
            1,
            Component::WirelessHop {
                distance_m: 1000.0,
                frequency_mhz: 2400.0,
            },
        );
        graph.connect(1, 2, Component::Amplifier { gain_db: 90.0 });
        graph.connect(
            2,
            3,
            Component::WirelessHop {
                distance_m: 100.0,
                frequency_mhz: 2400.0,
            },
        );
        let optical = graph
            .compute_link_budget_with_outage(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                Wavelength::from(level.wavelength),
                level.receive_window(),
                None,
            )
            .expect("synthetic path must resolve");
        assert!(
            optical.in_window,
            "optical budget must pass the amp chain (Rx {:.2})",
            optical.received_dbm
        );
        assert!(
            !level.is_win_state_with_outage(
                &graph,
                level.tx_dbm,
                Wavelength::from(level.wavelength),
                None,
            ),
            "wireless evaluator must fail the amp chain on carried SNR"
        );
        let eval = graph
            .evaluate_wireless(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                level.receive_window(),
                level.wireless_params(),
            )
            .expect("synthetic path must resolve");
        assert!(
            level.receive_window().contains(eval.received_dbm),
            "amp chain level must be in window (got {:.2}) — SNR is the failure",
            eval.received_dbm
        );
        assert!(
            eval.violations
                .iter()
                .any(|v| matches!(v, WirelessViolation::SignalToNoiseTooLow { .. })),
            "violations must include SignalToNoiseTooLow, got {:?}",
            eval.violations
        );

        // Direction 3 — the reverse: the wireless evaluator passes a
        // state the optical budget fails. m1l2's balanced repeater at
        // full interference accrual: the optical model subtracts the
        // 12 dB accrual from the received level (−68.01 − 12 = −80.01,
        // below the −75 floor), while the floor-rise model keeps the
        // level and raises the SNR bar to 22 — and 26.99 clears it.
        let level = load_level(21);
        let rpt = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 20.0))
            .expect("m1l2 must offer a 20 dBm repeater");
        let graph = graph_with_single_placement(&level, rpt);
        let mut storm = fresh_outage(&level).unwrap();
        storm.tick(600.0);
        let optical = graph
            .compute_link_budget_with_outage(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                Wavelength::from(level.wavelength),
                level.receive_window(),
                Some(&storm),
            )
            .expect("m1l2 path must resolve");
        assert!(
            !optical.in_window,
            "optical budget must fail m1l2 at full accrual (Rx {:.2})",
            optical.received_dbm
        );
        assert!(
            level.is_win_state_with_outage(
                &graph,
                level.tx_dbm,
                Wavelength::from(level.wavelength),
                Some(&storm),
            ),
            "wireless evaluator must pass m1l2 at full accrual (SNR 26.99 ≥ 22)"
        );
    }

    /// Adversarial: garbage specialty parameters fail closed — a NaN
    /// launch, floor, or requirement must never let a level pass, and
    /// the ledger must render the failure instead of panicking.
    #[test]
    fn specialty_params_fail_closed_on_nan() {
        // Coax: NaN launch power.
        let mut level = load_level(11);
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 3.0))
            .expect("c1l2 must offer a 3 dB amp")
            .clone();
        let graph = graph_with_single_placement(&level, &amp);
        let wavelength = Wavelength::from(level.wavelength);
        assert!(
            !level.is_win_state_with_outage(&graph, f64::NAN, wavelength, None),
            "NaN coax launch must fail closed"
        );
        // Coax: NaN floor / NaN CNR requirement via the level fields.
        level.coax_noise_floor_dbmv = Some(f64::NAN);
        assert!(
            !level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None),
            "NaN coax noise floor must fail closed"
        );
        level.coax_noise_floor_dbmv = None;
        level.min_carrier_to_noise_db = Some(f64::NAN);
        assert!(
            !level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None),
            "NaN CNR requirement must fail closed"
        );
        let ledger = level.signal_ledger(&graph, level.tx_dbm, wavelength, None);
        assert!(
            ledger.contains("non-finite input"),
            "ledger must name the non-finite input, got: {ledger}"
        );

        // Wireless: NaN launch / NaN SNR requirement.
        let mut level = load_level(20);
        let rpt = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 20.0))
            .expect("m1l1 must offer a 20 dBm repeater")
            .clone();
        let graph = graph_with_single_placement(&level, &rpt);
        let wavelength = Wavelength::from(level.wavelength);
        assert!(
            level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None),
            "sanity: m1l1's 20 dBm repeater wins with honest parameters"
        );
        assert!(
            !level.is_win_state_with_outage(&graph, f64::NAN, wavelength, None),
            "NaN wireless launch must fail closed"
        );
        level.min_snr_db = Some(f64::NAN);
        assert!(
            !level.is_win_state_with_outage(&graph, level.tx_dbm, wavelength, None),
            "NaN SNR requirement must fail closed"
        );
    }

    /// The ledgers speak each discipline's language: a coax ledger
    /// carries CNR, a wireless ledger carries SNR and fade margin —
    /// and a drowned carrier reads CNR TOO LOW even at a good level.
    #[test]
    fn specialty_ledgers_report_cnr_and_snr() {
        let level = load_level(10); // c1l1
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 5.0))
            .expect("c1l1 must offer a 5 dB amp");
        let graph = graph_with_single_placement(&level, amp);
        let ledger = level.signal_ledger(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
        );
        assert!(
            ledger.contains("CNR:") && ledger.contains("CNR OK") && ledger.contains("IN WINDOW"),
            "coax ledger must report a passing CNR, got: {ledger}"
        );

        let level = load_level(20); // m1l1
        let rpt = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 20.0))
            .expect("m1l1 must offer a 20 dBm repeater");
        let graph = graph_with_single_placement(&level, rpt);
        let ledger = level.signal_ledger(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
        );
        assert!(
            ledger.contains("SNR:") && ledger.contains("SNR OK") && ledger.contains("IN WINDOW"),
            "wireless ledger must report a passing SNR, got: {ledger}"
        );

        // The CNR failure wording shows up in the ledger, not just the
        // evaluator: hostile floor on c1l2 (same setup as the
        // disagreement test) reads CNR TOO LOW at an in-window level.
        let mut level = load_level(11);
        level.coax_noise_floor_dbmv = Some(-10.0);
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 3.0))
            .expect("c1l2 must offer a 3 dB amp")
            .clone();
        let graph = graph_with_single_placement(&level, &amp);
        let ledger = level.signal_ledger(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
        );
        assert!(
            ledger.contains("CNR TOO LOW") && ledger.contains("IN WINDOW"),
            "ledger must read CNR TOO LOW at an in-window level, got: {ledger}"
        );
    }

    /// BxE portal ops (Hikari's field track): every variant needs a
    /// non-empty label and blurb (the console renders both), and the
    /// address-to-diagnostics / slow-portal / master-tech sequences
    /// must verify through the shared sequence checker.
    #[test]
    fn bxe_ops_have_labels_and_verify_sequences() {
        for op in [
            ApiOp::SearchAddress,
            ApiOp::ListDevices,
            ApiOp::ReadDiagnostics,
            ApiOp::RunSpeedTest,
            ApiOp::CertifyInstall,
        ] {
            assert!(!op.label().is_empty(), "{op:?} has an empty label");
            assert!(!op.blurb().is_empty(), "{op:?} has an empty blurb");
        }
        // hikari2: full portal path.
        let full = [
            ApiOp::Login,
            ApiOp::SearchAddress,
            ApiOp::ListDevices,
            ApiOp::ReadDiagnostics,
        ];
        assert!(verify_api_sequence(&full, &full));
        assert!(!verify_api_sequence(&full, &full[..3]));
        // hikari6: the slow-portal shortcut skips search entirely.
        let fast = [ApiOp::Login, ApiOp::ReadDiagnostics];
        assert!(verify_api_sequence(&fast, &fast));
        assert!(!verify_api_sequence(&fast, &full));
        // hikari10: the complete certification workflow.
        let certify = [
            ApiOp::Login,
            ApiOp::SearchAddress,
            ApiOp::ListDevices,
            ApiOp::ReadDiagnostics,
            ApiOp::RunSpeedTest,
            ApiOp::CertifyInstall,
        ];
        assert!(verify_api_sequence(&certify, &certify));
    }

    // ---- Astra §2a: independent verification states ----

    fn status_of(lines: &[StateLine], id: &StateId) -> StateStatus {
        state_line(lines, id)
            .unwrap_or_else(|| panic!("state {id:?} must be present"))
            .status
    }

    #[test]
    fn verification_states_split_coax_verdicts() {
        // c1l1 winner: every coax state passes independently.
        let level = load_level(10);
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 5.0))
            .expect("c1l1 must offer a 5 dB amp");
        let graph = graph_with_single_placement(&level, amp);
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &MechanicStates::default(),
        );
        assert_eq!(status_of(&lines, &StateId::Continuity), StateStatus::Pass);
        assert_eq!(status_of(&lines, &StateId::CarrierLevel), StateStatus::Pass);
        assert_eq!(status_of(&lines, &StateId::ServiceCnr), StateStatus::Pass);
        let summary = states_summary(&lines);
        assert!(summary.contains("Service (CNR) PASS"), "got: {summary}");
        assert!(states_failures(&lines).is_empty());
    }

    #[test]
    fn verification_states_show_the_c3_precondition() {
        // The C3 lesson's precondition, as a state vector: carrier in
        // window while service fails (hostile floor on c1l2, the same
        // setup as the evaluator-disagreement test).
        let mut level = load_level(11);
        level.coax_noise_floor_dbmv = Some(-10.0);
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 3.0))
            .expect("c1l2 must offer a 3 dB amp")
            .clone();
        let graph = graph_with_single_placement(&level, &amp);
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &MechanicStates::default(),
        );
        assert_eq!(status_of(&lines, &StateId::Continuity), StateStatus::Pass);
        assert_eq!(status_of(&lines, &StateId::CarrierLevel), StateStatus::Pass);
        assert_eq!(status_of(&lines, &StateId::ServiceCnr), StateStatus::Fail);
        let failures = states_failures(&lines);
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].contains("Continuity passes, but service acceptance still fails"),
            "the three-state readout must name the split, got: {}",
            failures[0]
        );
    }

    #[test]
    fn verification_states_split_wireless_level_from_snr() {
        // m1l1 winner: level and SNR pass independently.
        let level = load_level(20);
        let rpt = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 20.0))
            .expect("m1l1 must offer a 20 dBm repeater");
        let graph = graph_with_single_placement(&level, rpt);
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &MechanicStates::default(),
        );
        assert_eq!(status_of(&lines, &StateId::LinkLevel), StateStatus::Pass);
        assert_eq!(status_of(&lines, &StateId::LinkSnr), StateStatus::Pass);

        // The amp-chain trap (same synthetic graph as the
        // disagreement test): level in window, SNR wrecked — "loud
        // is not clear" as a state vector.
        let mut graph = PathGraph::default();
        for node in &level.nodes {
            graph.add_node(node.id, node.label.clone());
        }
        graph.connect(
            0,
            1,
            Component::WirelessHop {
                distance_m: 1000.0,
                frequency_mhz: 2400.0,
            },
        );
        graph.connect(1, 2, Component::Amplifier { gain_db: 90.0 });
        graph.connect(
            2,
            3,
            Component::WirelessHop {
                distance_m: 100.0,
                frequency_mhz: 2400.0,
            },
        );
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &MechanicStates::default(),
        );
        assert_eq!(status_of(&lines, &StateId::LinkLevel), StateStatus::Pass);
        assert_eq!(status_of(&lines, &StateId::LinkSnr), StateStatus::Fail);
    }

    #[test]
    fn verification_states_fail_closed_on_an_open_path() {
        // Nothing placed: continuity fails and the measured states
        // stay Pending — never a spurious Pass on an open route.
        let level = load_level(10);
        let mut graph = PathGraph::default();
        for node in &level.nodes {
            graph.add_node(node.id, node.label.clone());
        }
        for edge in &level.fixed_edges {
            graph.connect(edge.from, edge.to, edge.component.clone());
        }
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &MechanicStates::default(),
        );
        assert_eq!(status_of(&lines, &StateId::Continuity), StateStatus::Fail);
        assert_eq!(
            status_of(&lines, &StateId::CarrierLevel),
            StateStatus::Pending
        );
        assert_eq!(
            status_of(&lines, &StateId::ServiceCnr),
            StateStatus::Pending
        );
    }

    #[test]
    fn astra_registry_card_data_matches_shipped_json() {
        // §3.1 registry: across all 20 mapped coax/wireless levels,
        // the acceptance fields are explicit in the JSON (never
        // silently defaulted), the briefing states the acceptance
        // numbers, and no briefing keeps CATV plant vocabulary
        // (G.fast ruling: distribution point, not headend).
        let raws: Vec<serde_json::Value> = LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        let defs: Vec<LevelDef> = LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        let mut coax = 0;
        let mut wireless = 0;
        for (raw, def) in raws.iter().zip(&defs) {
            let is_coax = def.id.starts_with("c1l");
            let is_wireless = def.id.starts_with("m1l");
            if !is_coax && !is_wireless {
                continue;
            }
            assert!(
                !def.briefing.contains("headend") && !def.briefing.contains("Headend"),
                "{} briefing keeps CATV vocabulary",
                def.id
            );
            if is_coax {
                coax += 1;
                assert_eq!(
                    raw["coax_noise_floor_dbmv"].as_f64(),
                    Some(-35.0),
                    "{}",
                    def.id
                );
                assert_eq!(
                    raw["min_carrier_to_noise_db"].as_f64(),
                    Some(25.0),
                    "{}",
                    def.id
                );
                assert!(
                    def.briefing.contains("25 dB"),
                    "{} briefing must state CNR",
                    def.id
                );
                assert!(
                    def.briefing.contains("35 dBmV"),
                    "{} briefing must state the floor",
                    def.id
                );
            } else {
                wireless += 1;
                assert_eq!(raw["min_snr_db"].as_f64(), Some(10.0), "{}", def.id);
                assert!(
                    def.briefing.contains("10 dB"),
                    "{} briefing must state SNR",
                    def.id
                );
            }
            // Card == JSON: every authored block is well-formed
            // against the shipped data itself.
            if let Some(ident) = &def.identification {
                assert!(identification_is_well_formed(ident), "{}", def.id);
            }
            if let Some(inter) = &def.intermittent {
                assert!(intermittent_is_well_formed(inter), "{}", def.id);
            }
            if let Some(wb) = &def.workbench {
                assert!(!wb.card_title.is_empty(), "{}", def.id);
                assert!(!wb.card_lines.is_empty(), "{}", def.id);
                assert_eq!(wb.steps.len(), 10, "{}", def.id);
                for step in &wb.steps {
                    assert!(step.correct < step.options.len(), "{}", def.id);
                }
            }
            for cfg in [&def.static_config_v4, &def.static_config_v6]
                .into_iter()
                .flatten()
            {
                assert!(
                    cfg.address_candidates.contains(&cfg.worksheet_address),
                    "{}",
                    def.id
                );
                assert!(
                    cfg.gateway_candidates.contains(&cfg.worksheet_gateway),
                    "{}",
                    def.id
                );
                assert!(
                    cfg.dns_candidates.contains(&cfg.worksheet_dns),
                    "{}",
                    def.id
                );
                assert!(
                    address_availability(cfg, &cfg.worksheet_address).is_ok(),
                    "{} worksheet address must be available to its own device",
                    def.id
                );
            }
            if let Some(survey) = &def.survey {
                assert!(!survey.points.is_empty(), "{}", def.id);
                if let Some(d) = &survey.diagnosis {
                    assert!(d.correct < d.options.len(), "{}", def.id);
                }
            }
        }
        assert_eq!(coax, 10);
        assert_eq!(wireless, 10);
    }

    #[test]
    fn m1l10_capstone_survey_gates_on_the_measured_winner() {
        // Measured on the shipped data: the 25 dBm repeater reads
        // -78.28 dBm / SNR 16.72 at Site B and passes every point;
        // the amplifier trap fails the survey outright.
        let level: LevelDef = LEVEL_SOURCES
            .iter()
            .map(|s| serde_json::from_str(s).unwrap())
            .find(|l: &LevelDef| l.id == "m1l10")
            .unwrap();
        assert!(level.handoff_required == false);
        let winner = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 25.0))
            .unwrap();
        let graph = graph_with_single_placement(&level, winner);
        assert!(crate::astra::survey_gate_pass(
            &level,
            &graph,
            level.tx_dbm,
            None
        ));
        let trap = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { .. }))
            .unwrap();
        let graph = graph_with_single_placement(&level, trap);
        assert!(!crate::astra::survey_gate_pass(
            &level,
            &graph,
            level.tx_dbm,
            None
        ));
    }

    #[test]
    fn verification_states_merge_mechanic_lines_in_fixed_order() {
        let level = load_level(10);
        let amp = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 5.0))
            .expect("c1l1 must offer a 5 dB amp");
        let graph = graph_with_single_placement(&level, amp);
        let mechanics = MechanicStates {
            identity: Some((StateStatus::Fail, "Mapper reads remote ID-5.".to_string())),
            documentation: Some((StateStatus::Pending, "Closeout open.".to_string())),
            ..MechanicStates::default()
        };
        let lines = level.verification_states(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
            &mechanics,
        );
        assert_eq!(status_of(&lines, &StateId::Identity), StateStatus::Fail);
        assert_eq!(
            status_of(&lines, &StateId::Documentation),
            StateStatus::Pending
        );
        // Mechanic lines come after the board states.
        let identity_pos = lines
            .iter()
            .position(|l| l.id == StateId::Identity)
            .unwrap();
        let service_pos = lines
            .iter()
            .position(|l| l.id == StateId::ServiceCnr)
            .unwrap();
        assert!(identity_pos > service_pos);
    }
}
