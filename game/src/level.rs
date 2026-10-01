//! Level data model — deserialized from `assets/levels/*.json`. Each level
//! describes a starting `PathGraph` layout (some edges pre-placed, some left
//! for the player to complete), a target receive window, and optionally a
//! scripted outage that fires after N seconds of play.

use bevy::prelude::*;
use osp_sim::{Component, EthernetViolation, Medium, Outage, OutageKind, PathGraph, ReceiveWindow, Wavelength};
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
    pub fn is_win_state(
        &self,
        graph: &PathGraph,
        tx_dbm: f64,
        wavelength: Wavelength,
    ) -> bool {
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
                                        format!(
                                            "segment {longest_m:.0}m > {limit_m:.0}m max"
                                        )
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
    include_str!("../assets/levels/world4_level1_outage.json"),
    include_str!("../assets/levels/coax1_unity_gain.json"),
    include_str!("../assets/levels/coax2_ingress.json"),
    include_str!("../assets/levels/wireless1_close_the_link.json"),
    include_str!("../assets/levels/wireless2_ride_the_storm.json"),
    include_str!("../assets/levels/ethernet1_hundred_meter_wall.json"),
    include_str!("../assets/levels/ethernet2_power_budget.json"),
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
    #[test]
    fn every_level_has_exactly_its_intended_winning_pill_count() {
        let expected_wins = [2usize, 1, 1, 1, 1, 1, 1, 1];
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
        let coax = load_level(3);
        let mut ingress = fresh_outage(&coax).unwrap();
        ingress.tick(90.0); // +9 dB of floor rise
        let repair_amp = coax
            .available_components
            .iter()
            .find(|c| {
                matches!(c.component, Component::Amplifier { gain_db } if gain_db == 12.0)
            })
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

        let wireless = load_level(5);
        let mut interference = fresh_outage(&wireless).unwrap();
        interference.tick(80.0); // +8 dB of floor rise
        let repair_rpt = wireless
            .available_components
            .iter()
            .find(|c| {
                matches!(c.component, Component::Repeater { tx_dbm } if tx_dbm == 30.0)
            })
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
