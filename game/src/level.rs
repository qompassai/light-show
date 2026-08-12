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
