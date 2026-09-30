//! Level data model — deserialized from `assets/levels/*.json`. Each level
//! describes a starting `PathGraph` layout (some edges pre-placed, some left
//! for the player to complete), a target receive window, and optionally a
//! scripted outage that fires after N seconds of play.

use bevy::prelude::*;
use osp_sim::{Component, OutageKind, ReceiveWindow, Wavelength};
use serde::Deserialize;

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
}

impl From<OutageKindDef> for OutageKind {
    fn from(k: OutageKindDef) -> Self {
        match k {
            OutageKindDef::FiberCut => OutageKind::FiberCut,
            OutageKindDef::AerialDamage => OutageKind::AerialDamage,
            OutageKindDef::WaterIntrusion => OutageKind::WaterIntrusion,
            OutageKindDef::ConnectorContamination => OutageKind::ConnectorContamination,
            OutageKindDef::Macrobend => OutageKind::Macrobend,
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
}

/// Level JSON is embedded at compile time (rather than loaded through
/// Bevy's `AssetServer`) so the same code path works identically on
/// desktop and Android without needing APK asset-file access at runtime.
pub const LEVEL_SOURCES: &[&str] = &[
    include_str!("../assets/levels/world1_level1.json"),
    include_str!("../assets/levels/world4_level1_outage.json"),
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
}
