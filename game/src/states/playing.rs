//! Core puzzle-solving state: renders the OSP node graph, lets the player
//! tap/drag to place components on open edges, keeps a live `osp_sim`
//! `PathGraph` in sync, and watches the scripted outage clock.

use super::GameState;
use crate::level::LevelDef;
use bevy::prelude::*;
use osp_sim::{PathGraph, Wavelength};

pub struct PlayingPlugin;

impl Plugin for PlayingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LiveGraph::default())
            .insert_resource(LevelClock::default())
            .add_systems(OnEnter(GameState::Playing), setup_level)
            .add_systems(
                Update,
                (tick_clock, check_scripted_outage, check_win_condition)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// The player's current in-progress graph, mirrored from `LevelDef` plus
/// whatever edges they've placed so far. Wrapped as a resource so UI and
/// simulation systems can both read/write it without a full ECS redesign.
#[derive(Resource, Default)]
pub struct LiveGraph {
    pub graph: PathGraph,
    pub wavelength: WavelengthWrapper,
    pub tx_dbm: f64,
}

// Bevy resources need a concrete default; osp_sim::Wavelength has no
// Default impl (physically all three are valid "defaults"), so we wrap it.
pub struct WavelengthWrapper(pub Wavelength);
impl Default for WavelengthWrapper {
    fn default() -> Self {
        WavelengthWrapper(Wavelength::Nm1490)
    }
}

#[derive(Resource, Default)]
pub struct LevelClock {
    pub elapsed_seconds: f64,
}

fn setup_level(mut live: ResMut<LiveGraph>, level: Option<Res<LevelDef>>) {
    let Some(level) = level else { return };
    let mut graph = PathGraph::default();
    for node in &level.nodes {
        graph.add_node(node.id, node.label.clone());
    }
    for edge in &level.fixed_edges {
        graph.connect(edge.from, edge.to, edge.component.clone());
    }
    live.graph = graph;
    live.wavelength = WavelengthWrapper(level.wavelength.into());
    live.tx_dbm = level.tx_dbm;
}

fn tick_clock(time: Res<Time>, mut clock: ResMut<LevelClock>) {
    clock.elapsed_seconds += time.delta_seconds_f64();
}

fn check_scripted_outage(
    clock: Res<LevelClock>,
    level: Option<Res<LevelDef>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(level) = level else { return };
    let Some(outage) = &level.scripted_outage else {
        return;
    };
    if clock.elapsed_seconds >= outage.fires_after_seconds {
        next_state.set(GameState::OutageActive);
    }
}

fn check_win_condition(
    live: Res<LiveGraph>,
    level: Option<Res<LevelDef>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(level) = level else { return };
    let Ok(result) = live.graph.compute_link_budget(
        level.source_node,
        level.target_node,
        live.tx_dbm,
        live.wavelength.0,
        level.receive_window(),
    ) else {
        return;
    };
    if result.in_window {
        next_state.set(GameState::Results);
    }
}
