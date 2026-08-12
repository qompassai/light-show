//! Outage-active state: a fault has fired on the current level's graph.
//! Shows the alarm UI, ticks the repair timer, and returns to `Playing`
//! once the player patches the affected edge (or to `Results` on failure).

use super::GameState;
use bevy::prelude::*;
use osp_sim::Outage;

pub struct OutagePlugin;

impl Plugin for OutagePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActiveOutage::default())
            .add_systems(OnEnter(GameState::OutageActive), announce_outage)
            .add_systems(
                Update,
                (tick_outage, check_outage_resolution).run_if(in_state(GameState::OutageActive)),
            );
    }
}

#[derive(Resource, Default)]
pub struct ActiveOutage {
    pub outage: Option<Outage>,
}

fn announce_outage(active: Res<ActiveOutage>) {
    if let Some(outage) = &active.outage {
        info!(
            "OUTAGE: {} (timer: {:.0}s)",
            outage.kind.flavor_text(),
            outage.time_remaining()
        );
    }
}

fn tick_outage(time: Res<Time>, mut active: ResMut<ActiveOutage>) {
    if let Some(outage) = &mut active.outage {
        outage.tick(time.delta_seconds_f64());
    }
}

fn check_outage_resolution(
    active: Res<ActiveOutage>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Some(outage) = &active.outage else { return };
    if outage.resolved {
        next_state.set(GameState::Playing);
    } else if outage.is_expired() {
        next_state.set(GameState::Results);
    }
}
