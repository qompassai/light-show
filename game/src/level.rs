//! Level data model — deserialized from `assets/levels/*.json`. Each level
//! describes a starting `PathGraph` layout (some edges pre-placed, some left
//! for the player to complete), a target receive window, and optionally a
//! scripted outage that fires after N seconds of play.

use bevy::prelude::*;
use osp_sim::{
    CoaxViolation, Component, EthernetViolation, Medium, Outage, OutageKind, PathGraph,
    ReceiveWindow, Wavelength, WirelessViolation,
};
use serde::{Deserialize, Serialize};

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

/// One fiber in the TIA-598 color chart: a fiber's identity is the
/// (tube, strand) color pair, and its number is
/// `(tube_number - 1) * 12 + strand_number`. Deserialized from the
/// splice work orders (see [`SpliceWorkOrdersDef`]); the chart test
/// re-derives every entry from that rule instead of trusting the data.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpliceFiberDef {
    /// 1-based fiber number, 1..=144.
    pub fiber: u32,
    /// 1-based buffer-tube number, 1..=12.
    pub tube_number: u32,
    pub tube_color: String,
    /// 1-based strand number inside the tube, 1..=12.
    pub strand_number: u32,
    pub strand_color: String,
}

/// One line of a splice beat's work order: splice this fiber to the
/// secondary splitter's pigtail feeding `drop`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpliceWorkOrderEntryDef {
    pub fiber: u32,
    pub tube_number: u32,
    pub tube_color: String,
    pub strand_number: u32,
    pub strand_color: String,
    /// The drop (premises) this fiber must light.
    pub drop: String,
}

/// A damaged strand inside a beat: the player must exclude it and
/// splice the designated `spare` instead of forcing the match.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpliceDamageBeatDef {
    pub fiber: u32,
    pub tube_number: u32,
    pub tube_color: String,
    pub strand_number: u32,
    pub strand_color: String,
    /// What is wrong with the strand, as written on the work order.
    pub condition: String,
    /// The instruction the work order gives for it.
    pub action: String,
    /// The designated spare fiber spliced in its place.
    pub spare: SpliceFiberDef,
}

/// One teaching beat of the splice work orders: a presented tray, the
/// ordered splices to make, and the rule that scores them.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpliceBeatDef {
    /// 1-based beat number, in play order.
    pub beat: u32,
    pub name: String,
    /// The craft lesson this beat teaches.
    pub teaching: String,
    /// What the player is shown at the tray.
    pub presented: String,
    pub work_order: Vec<SpliceWorkOrderEntryDef>,
    /// Damaged strands to exclude (empty on beats without damage).
    #[serde(default)]
    pub damage_beats: Vec<SpliceDamageBeatDef>,
    pub win_rule: String,
}

/// Splice work orders (sp1, the splice "Field School" scenario): the
/// TIA-598 chart and the beat-by-beat strand→pigtail work orders the
/// repair is scored against. This is an optional [`LevelDef`] block so
/// the authored data (staged in `splice_work_orders.json`) lands typed
/// and tested instead of as dead text. The strand→pigtail interaction
/// UI that consumes it is future work (CONTENT-NOTES gap 4): today the
/// block parses, round-trips, and is validated against the color rule,
/// while sp1's shipped win check remains the light-path budget.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SpliceWorkOrdersDef {
    /// The fiber-number rule, as written on the work order.
    pub rule: String,
    /// The 12 TIA-598 colors in sequence; tubes and strands both wear it.
    pub color_sequence: Vec<String>,
    /// The full 144-fiber chart, in fiber-number order.
    pub chart_144: Vec<SpliceFiberDef>,
    /// The beats, in play order.
    pub beats: Vec<SpliceBeatDef>,
    /// Why a mis-splice fails the way it does (lights the wrong drop).
    #[serde(default)]
    pub scoring_honesty: Option<String>,
    /// The briefing card text that hands the player the chart.
    #[serde(default)]
    pub briefing_chart_text: Option<String>,
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
    /// Splice work orders (sp1 only): the TIA-598 chart + beats for the
    /// strand→pigtail repair. `None` on every other level. See
    /// [`SpliceWorkOrdersDef`] for why the data lands before its UI.
    #[serde(default)]
    pub splice_work_orders: Option<SpliceWorkOrdersDef>,
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

/// Out-of-track scenario levels ("Field School", hosted by Séraphine):
/// the two field-job scenarios and the splice scenario. These are
/// deliberately NOT part of [`LEVEL_SOURCES`] — the 80 shipped track
/// levels and their indices are frozen (see `Companion::track_indices`),
/// so scenarios live in their own compile-time registry, addressed by
/// level id (the `footage.rs` scan is the precedent), and enter play
/// through [`CurrentScenarioId`] instead of a track position.
pub const SCENARIO_SOURCES: &[&str] = &[
    include_str!("../assets/levels/fieldjob1_replace_the_1x4.json"),
    include_str!("../assets/levels/fieldjob2_prove_the_sb.json"),
    include_str!("../assets/levels/splice1_secondary_split.json"),
];

/// Ids of every bundled scenario level, in Field School menu order.
/// Kept in lockstep with [`SCENARIO_SOURCES`] by the registry tests.
pub const SCENARIO_IDS: &[&str] = &["fj1", "fj2", "sp1"];

/// Load a scenario level by id (e.g. `"fj1"`). `None` for any id that
/// is not a bundled scenario — including shipped track ids, which are
/// addressed by index through [`load_level`], never by this loader.
/// Never panics: unknown ids are a caller bug to surface, not a crash.
pub fn load_scenario(id: &str) -> Option<LevelDef> {
    if !SCENARIO_IDS.contains(&id) {
        return None;
    }
    SCENARIO_SOURCES.iter().find_map(|source| {
        let def: LevelDef = serde_json::from_str(source).ok()?;
        (def.id == id).then_some(def)
    })
}

/// True when `id` names a bundled scenario level. Results routing keys
/// off this: scenarios never advance along a track.
pub fn is_scenario_id(id: &str) -> bool {
    SCENARIO_IDS.contains(&id)
}

/// The scenario level the player picked on the companion-select
/// Field School row, if any. `Some(id)` takes precedence over
/// [`CurrentLevelIndex`] everywhere gameplay loads its level (see
/// [`load_current_level`]); starting a track level clears it back to
/// `None`, and results routing clears it on the way back to the picker.
#[derive(Resource, Clone, Default)]
pub struct CurrentScenarioId(pub Option<String>);

/// The one level-loading entry point gameplay uses: the scenario named
/// by `scenario_id` when one is active, otherwise the track level at
/// `index`. An unknown scenario id falls back to the track level
/// instead of panicking — the picker only ever writes ids from
/// [`SCENARIO_IDS`], so the fallback is unreachable in normal play and
/// exists so a stale id can never soft-lock the game on a black screen.
pub fn load_current_level(index: usize, scenario_id: Option<&str>) -> LevelDef {
    match scenario_id {
        Some(id) => load_scenario(id).unwrap_or_else(|| load_level(index)),
        None => load_level(index),
    }
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
        graph_with_placements(level, &[choice])
    }

    /// Like `graph_with_single_placement`, but places several choices —
    /// the scenario levels (fj1/fj2) gate completion on two edges at
    /// once (the splice bleed AND the splitter swap), so their
    /// playthrough tests must replay whole choice combinations.
    fn graph_with_placements(level: &LevelDef, choices: &[&ComponentChoice]) -> PathGraph {
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
        for choice in choices {
            if severed != Some((choice.from, choice.to)) {
                graph.connect(choice.from, choice.to, choice.component.clone());
            }
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

    // -- Scenario (Field School) levels -----------------------------------
    //
    // fj1/fj2/sp1 live OUTSIDE `LEVEL_SOURCES` (see `SCENARIO_SOURCES`):
    // the 80 track levels and their indices are frozen. The tests below
    // pin both halves of that bargain — the freeze, and the scenarios'
    // own winnability/readings replayed through the shipped loss engine.

    /// Adversarial (the freeze): the track registry must stay exactly
    /// the 80 shipped levels, and the recon-named index anchors must
    /// still resolve to the same level ids — an accidental mid-track
    /// insertion of a scenario would shift every one of these.
    #[test]
    fn track_registry_stays_frozen_at_80_levels() {
        assert_eq!(LEVEL_SOURCES.len(), 80, "the 80-level freeze holds");
        for (index, id) in [
            (0, "w1l1"),
            (11, "c1l2"),
            (14, "c1l5"),
            (17, "c1l8"),
            (20, "m1l1"),
            (21, "m1l2"),
            (29, "m1l10"),
        ] {
            assert_eq!(
                load_level(index).id,
                id,
                "track index {index} must still be {id}"
            );
        }
    }

    #[test]
    fn scenario_registry_holds_exactly_fj1_fj2_sp1() {
        assert_eq!(SCENARIO_SOURCES.len(), 3);
        assert_eq!(SCENARIO_IDS, ["fj1", "fj2", "sp1"]);
        // Sources and ids agree, in menu order.
        for (source, id) in SCENARIO_SOURCES.iter().zip(SCENARIO_IDS.iter()) {
            let level: LevelDef = serde_json::from_str(source).unwrap();
            assert_eq!(&level.id, id);
            assert!(is_scenario_id(id));
        }
        // A track id is never a scenario id.
        assert!(!is_scenario_id("w1l1"));
        assert!(!is_scenario_id("c1l2"));
    }

    #[test]
    fn every_scenario_parses_and_has_a_usable_shape() {
        for id in SCENARIO_IDS {
            let level = load_scenario(id).unwrap_or_else(|| panic!("{id} must load"));
            assert!(!level.nodes.is_empty(), "{id} has no nodes");
            assert!(
                level.window_min_dbm < level.window_max_dbm,
                "{id} has an inverted receive window"
            );
            let node_ids: std::collections::HashSet<u32> =
                level.nodes.iter().map(|n| n.id).collect();
            assert!(node_ids.contains(&level.source_node), "{id} source node");
            assert!(node_ids.contains(&level.target_node), "{id} target node");
            for edge in level
                .fixed_edges
                .iter()
                .map(|e| (e.from, e.to))
                .chain(level.available_components.iter().map(|c| (c.from, c.to)))
            {
                assert!(
                    node_ids.contains(&edge.0) && node_ids.contains(&edge.1),
                    "{id} references an edge {edge:?} with an undeclared node"
                );
            }
            // The shipped win-count discipline applies to scenarios too:
            // every decision must offer at least two choices, or there
            // is no decision to teach.
            for group in scenario_choice_groups(&level) {
                assert!(
                    group.len() >= 2,
                    "{id} offers a decision with fewer than two choices"
                );
            }
        }
    }

    /// Adversarial: the scenario loader fails closed — unknown ids,
    /// near-miss ids, and shipped track ids all return `None`, and
    /// `load_current_level` falls back to the track level rather than
    /// panicking on a stale scenario id.
    #[test]
    fn scenario_loader_rejects_unknown_ids_without_panicking() {
        for bogus in ["", "fj3", "FJ1", "fj1 ", "sp2", "w1l1", "c1l2"] {
            assert!(
                load_scenario(bogus).is_none(),
                "{bogus:?} must not resolve as a scenario"
            );
        }
        let fallback = load_current_level(11, Some("fj3"));
        assert_eq!(fallback.id, "c1l2", "stale scenario id falls back");
        let track = load_current_level(11, None);
        assert_eq!(track.id, "c1l2");
        let scenario = load_current_level(11, Some("sp1"));
        assert_eq!(scenario.id, "sp1", "an active scenario wins over index");
    }

    /// Player choices grouped into decisions by the node they leave
    /// from, in encounter order — the unit a scenario playthrough picks
    /// one of per decision. Grouping by source node (not by exact edge)
    /// is what makes fj1's splitter swap one decision: the new 1x4
    /// (7→8) and the old 1x4 (7→9) are alternative destinations for the
    /// same placement, never both halves of one repair.
    fn scenario_choice_groups(level: &LevelDef) -> Vec<Vec<&ComponentChoice>> {
        let mut groups: Vec<Vec<&ComponentChoice>> = Vec::new();
        for choice in &level.available_components {
            match groups.iter_mut().find(|g| g[0].from == choice.from) {
                Some(group) => group.push(choice),
                None => groups.push(vec![choice]),
            }
        }
        groups
    }

    /// Replay one choice combination through the shipped loss engine:
    /// `(received dBm, in-window)` for the completed source→target path.
    fn scenario_received_dbm(level: &LevelDef, choices: &[&ComponentChoice]) -> (f64, bool) {
        let graph = graph_with_placements(level, choices);
        let result = graph
            .compute_link_budget_with_outage(
                level.source_node,
                level.target_node,
                level.tx_dbm,
                Wavelength::from(level.wavelength),
                level.receive_window(),
                None,
            )
            .expect("scenario path must resolve");
        (result.received_dbm, result.in_window)
    }

    /// Assert one replayed combination lands at the documented reading
    /// (CONTENT-NOTES, ±0.02 dB) with the documented verdict class.
    fn assert_scenario_reading(
        level: &LevelDef,
        choices: &[&ComponentChoice],
        expected_dbm: f64,
        verdict: &str,
    ) {
        let (rx_dbm, in_window) = scenario_received_dbm(level, choices);
        assert!(
            (rx_dbm - expected_dbm).abs() < 0.02,
            "{} replay must land at {expected_dbm:.2} dBm, got {rx_dbm:.4}",
            level.id
        );
        let graph = graph_with_placements(level, choices);
        let ledger = level.signal_ledger(
            &graph,
            level.tx_dbm,
            Wavelength::from(level.wavelength),
            None,
        );
        assert!(
            ledger.contains(verdict),
            "{} ledger must read {verdict}, got: {ledger}",
            level.id
        );
        assert_eq!(
            in_window,
            verdict == "IN WINDOW",
            "{} verdict class for Rx {rx_dbm:.2}",
            level.id
        );
        if verdict == "TOO LOW" {
            assert!(
                rx_dbm < level.window_min_dbm,
                "{} TOO LOW reading must sit below the window floor",
                level.id
            );
        }
    }

    /// The scenario counterpart of
    /// `every_level_has_exactly_its_intended_winning_pill_count`:
    /// counted over whole choice combinations (fj1/fj2 gate on two
    /// edges at once). fj1/fj2 each have exactly one winning
    /// combination — the full repair; sp1 has two winning pills
    /// (fusion, and the poorer-but-legit mechanical emergency repair).
    #[test]
    fn scenario_levels_have_exactly_their_intended_winning_combination_count() {
        for (id, expected) in [("fj1", 1usize), ("fj2", 1), ("sp1", 2)] {
            let level = load_scenario(id).expect("scenario loads");
            let groups = scenario_choice_groups(&level);
            let total: usize = groups.iter().map(|g| g.len()).product();
            let mut wins = 0;
            for mut n in 0..total {
                let picks: Vec<&ComponentChoice> = groups
                    .iter()
                    .map(|g| {
                        let choice = g[n % g.len()];
                        n /= g.len();
                        choice
                    })
                    .collect();
                if level.is_win_state(
                    &graph_with_placements(&level, &picks),
                    level.tx_dbm,
                    Wavelength::from(level.wavelength),
                ) {
                    wins += 1;
                }
            }
            assert_eq!(
                wins, expected,
                "{id} has {wins} winning combinations, expected {expected}"
            );
        }
    }

    fn splice_on_edge<'a>(
        level: &'a LevelDef,
        from: u32,
        to: u32,
        fusion: bool,
    ) -> &'a ComponentChoice {
        level
            .available_components
            .iter()
            .find(|c| {
                c.from == from
                    && c.to == to
                    && if fusion {
                        matches!(
                            c.component,
                            Component::Splice {
                                kind: osp_sim::SpliceType::Fusion,
                                ..
                            }
                        )
                    } else {
                        matches!(
                            c.component,
                            Component::Splice {
                                kind: osp_sim::SpliceType::Mechanical,
                                ..
                            }
                        )
                    }
            })
            .expect("scenario must offer the splice choice")
    }

    fn choice_on_edge(level: &LevelDef, from: u32, to: u32) -> &ComponentChoice {
        level
            .available_components
            .iter()
            .find(|c| c.from == from && c.to == to)
            .expect("scenario must offer the edge choice")
    }

    /// fj1 playthrough: both repairs (re-splice the bleed AND replace
    /// the 1x4) land the documented -17.65 dBm in window; either repair
    /// alone, or neither, leaves the customer dark (CONTENT-NOTES).
    #[test]
    fn fj1_choice_combinations_replay_the_job_readings() {
        let level = load_scenario("fj1").expect("fj1 loads");
        let bleed_fixed = splice_on_edge(&level, 6, 7, true);
        let bleed_as_found = splice_on_edge(&level, 6, 7, false);
        let new_splitter = choice_on_edge(&level, 7, 8);
        let old_splitter = choice_on_edge(&level, 7, 9);
        assert_scenario_reading(&level, &[bleed_fixed, new_splitter], -17.65, "IN WINDOW");
        assert_scenario_reading(&level, &[bleed_fixed, old_splitter], -20.47, "TOO LOW");
        assert_scenario_reading(&level, &[bleed_as_found, new_splitter], -22.17, "TOO LOW");
        assert_scenario_reading(&level, &[bleed_as_found, old_splitter], -25.00, "TOO LOW");
    }

    /// fj2 playthrough: the SB leg replays the same discipline — full
    /// repair -18.03 dBm in window, both partials fail, and the
    /// as-found plant reads the documented -25.45 dBm.
    #[test]
    fn fj2_choice_combinations_replay_the_job_readings() {
        let level = load_scenario("fj2").expect("fj2 loads");
        let bleed_fixed = splice_on_edge(&level, 7, 8, true);
        let bleed_as_found = splice_on_edge(&level, 7, 8, false);
        let new_splitter = choice_on_edge(&level, 8, 9);
        let old_splitter = choice_on_edge(&level, 8, 10);
        assert_scenario_reading(&level, &[bleed_fixed, new_splitter], -18.03, "IN WINDOW");
        assert_scenario_reading(&level, &[bleed_fixed, old_splitter], -20.86, "TOO LOW");
        assert_scenario_reading(&level, &[bleed_as_found, new_splitter], -22.63, "TOO LOW");
        assert_scenario_reading(&level, &[bleed_as_found, old_splitter], -25.45, "TOO LOW");
    }

    /// sp1 playthrough: a fusion splice lands -18.125 dBm, a mechanical
    /// splice is a legitimate (poorer) emergency repair at -18.45 dBm,
    /// and forcing the damaged strand kinks 3.5 dB out of the budget.
    #[test]
    fn sp1_choice_combinations_replay_the_job_readings() {
        let level = load_scenario("sp1").expect("sp1 loads");
        let fusion = splice_on_edge(&level, 3, 4, true);
        let mechanical = splice_on_edge(&level, 3, 4, false);
        let macrobend = level
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Macrobend { .. }))
            .expect("sp1 must offer the forced-macrobend trap");
        assert_scenario_reading(&level, &[fusion], -18.125, "IN WINDOW");
        assert_scenario_reading(&level, &[mechanical], -18.45, "IN WINDOW");
        assert_scenario_reading(&level, &[macrobend], -21.55, "TOO LOW");
    }

    /// The splice work orders land typed on sp1, survive a
    /// serialize→parse round-trip byte-for-value, and the whole chart
    /// re-derives from the TIA-598 rule the work order states.
    #[test]
    fn sp1_work_orders_parse_round_trip_and_follow_the_color_rule() {
        let level = load_scenario("sp1").expect("sp1 loads");
        let orders = level
            .splice_work_orders
            .as_ref()
            .expect("sp1 must carry its splice work orders");
        assert!(orders.rule.contains("(tube - 1) * 12 + strand"));
        assert_eq!(orders.color_sequence.len(), 12);
        assert_eq!(orders.color_sequence[0], "blue");
        assert_eq!(orders.color_sequence[11], "aqua");
        assert_eq!(orders.chart_144.len(), 144);
        assert_eq!(orders.beats.len(), 3);

        // The rule, checked against every chart entry — not spot checks:
        // fiber number, tube color, and strand color must all agree with
        // the sequence positions.
        for entry in &orders.chart_144 {
            assert_eq!(
                entry.fiber,
                (entry.tube_number - 1) * 12 + entry.strand_number,
                "chart fiber {} breaks the numbering rule",
                entry.fiber
            );
            assert_eq!(
                entry.tube_color,
                orders.color_sequence[(entry.tube_number - 1) as usize],
                "chart fiber {} tube color",
                entry.fiber
            );
            assert_eq!(
                entry.strand_color,
                orders.color_sequence[(entry.strand_number - 1) as usize],
                "chart fiber {} strand color",
                entry.fiber
            );
        }
        // The transposition trap the level teaches: fiber 2 is
        // blue/orange, fiber 13 is orange/blue.
        let fiber2 = &orders.chart_144[1];
        assert_eq!(
            (fiber2.tube_color.as_str(), fiber2.strand_color.as_str()),
            ("blue", "orange")
        );
        let fiber13 = &orders.chart_144[12];
        assert_eq!(
            (fiber13.tube_color.as_str(), fiber13.strand_color.as_str()),
            ("orange", "blue")
        );

        // Beat 3's damage beats: fibers 55 and 118 are excluded for
        // their designated spares 60 and 120.
        let damage = &orders.beats[2].damage_beats;
        assert_eq!(damage.len(), 2);
        assert_eq!((damage[0].fiber, damage[0].spare.fiber), (55, 60));
        assert_eq!((damage[1].fiber, damage[1].spare.fiber), (118, 120));

        // Round-trip: the block serializes and re-parses to itself.
        let json = serde_json::to_string(orders).expect("work orders serialize");
        let parsed: SpliceWorkOrdersDef =
            serde_json::from_str(&json).expect("work orders re-parse");
        assert_eq!(&parsed, orders);
    }

    /// Adversarial: the optional block is absent everywhere it does not
    /// belong — shipped track levels and the two field-job scenarios
    /// parse fine with `splice_work_orders: None`.
    #[test]
    fn work_orders_block_is_absent_where_it_does_not_belong() {
        for idx in [0usize, 11, 79] {
            assert!(
                load_level(idx).splice_work_orders.is_none(),
                "track level {idx} must not carry splice work orders"
            );
        }
        for id in ["fj1", "fj2"] {
            assert!(
                load_scenario(id)
                    .expect("scenario loads")
                    .splice_work_orders
                    .is_none(),
                "{id} must not carry splice work orders"
            );
        }
        // And a level JSON with no block at all still parses (serde
        // default): strip the block from sp1's source and re-parse.
        let mut value: serde_json::Value =
            serde_json::from_str(SCENARIO_SOURCES[2]).expect("sp1 source parses");
        value
            .as_object_mut()
            .expect("level JSON is an object")
            .remove("splice_work_orders");
        let stripped: LevelDef =
            serde_json::from_value(value).expect("sp1 without the block parses");
        assert!(stripped.splice_work_orders.is_none());
        assert_eq!(stripped.id, "sp1");
    }
}
