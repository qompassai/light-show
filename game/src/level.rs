//! Level data model — deserialized from `assets/levels/*.json`. Each level
//! describes a starting `PathGraph` layout (some edges pre-placed, some left
//! for the player to complete), a target receive window, and optionally a
//! scripted outage that fires after N seconds of play.

use bevy::prelude::*;
use osp_sim::{
    Component, EthernetViolation, Medium, Outage, OutageKind, PathGraph, ReceiveWindow, Wavelength,
};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum MediumDef {
    Fiber,
    Coax,
    Wireless,
    Ethernet,
}

impl From<MediumDef> for Medium {
    fn from(m: MediumDef) -> Self {
        match m {
            MediumDef::Fiber => Medium::Fiber,
            MediumDef::Coax => Medium::Coax,
            MediumDef::Wireless => Medium::Wireless,
            MediumDef::Ethernet => Medium::Ethernet,
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

/// An NBI/SMx API operation the player can issue. Clara's API-puzzle
/// levels (see `docs/reference/tds/cms/guides/cms-nbi-api.md` and
/// `docs/reference/tds/cms/guides/smx-api.md`). Deserialized from level
/// JSON by variant name; unknown names fail the parse loudly.
/// NBI (CMS) speaks SOAP/XML; SMx speaks REST/JSON — the enum covers
/// both, and each level's `api` tag says which door you're knocking on.
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
    /// Which API is being driven: "nbi" (CMS SOAP/XML) or "smx" (REST/JSON).
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

    /// Plain (non-outage) win check, dispatched on the level's medium.
    pub fn is_win_state(&self, graph: &PathGraph, tx_dbm: f64, wavelength: Wavelength) -> bool {
        self.is_win_state_with_outage(graph, tx_dbm, wavelength, None)
    }

    /// Outage-aware win check. Degrade hazards add their accumulated
    /// extra loss on top (fiber/coax/wireless); full cuts rely on the
    /// caller excluding the severed edge (see `board::rebuild_live_graph`),
    /// so they surface as an ordinary disconnect. Ethernet ignores the
    /// outage — its constraints don't degrade over time.
    pub fn is_win_state_with_outage(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
        outage: Option<&Outage>,
    ) -> bool {
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
                        if result.in_window {
                            "IN WINDOW"
                        } else {
                            "OUT OF WINDOW"
                        },
                    ),
                    Err(_) => "Link disconnected — no route completed.".to_string(),
                }
            }
        }
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

    // The two storm levels are only winnable if the mid-storm repair pill
    // actually lands back in window once the degrade has climbed: the 12 dB
    // amp 90 s into IngressNoise, and the 30 dBm repeater 80 s into
    // WirelessInterference.
    #[test]
    fn storm_repair_pills_win_mid_storm() {
        let coax = load_level(11); // c1l2 in the 50-level layout
        let mut ingress = fresh_outage(&coax).unwrap();
        ingress.tick(90.0); // +9 dB of floor rise
        let repair_amp = coax
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Amplifier { gain_db } if gain_db == 12.0))
            .expect("c1l2 must offer a 12 dB amp");
        let graph = graph_with_single_placement(&coax, repair_amp);
        assert!(
            coax.is_win_state_with_outage(
                &graph,
                coax.tx_dbm,
                Wavelength::from(coax.wavelength),
                Some(&ingress),
            ),
            "12 dB amp must rebalance c1l2 90 s into the ingress storm (17 − 9 = 8 dBmV)"
        );

        let wireless = load_level(21); // wireless2 in the 50-level layout
        let mut interference = fresh_outage(&wireless).unwrap();
        interference.tick(80.0); // +8 dB of floor rise
        let repair_rpt = wireless
            .available_components
            .iter()
            .find(|c| matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 30.0))
            .expect("m1l2 must offer a 30 dBm repeater");
        let graph = graph_with_single_placement(&wireless, repair_rpt);
        assert!(
            wireless.is_win_state_with_outage(
                &graph,
                wireless.tx_dbm,
                Wavelength::from(wireless.wavelength),
                Some(&interference),
            ),
            "30 dBm repeater must ride out m1l2 80 s into the interference storm"
        );
    }
}
