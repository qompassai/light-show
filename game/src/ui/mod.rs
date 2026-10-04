//! Ledger UI: the running dB budget readout styled like an OTDR trace.
//! Shows each placed component's contribution and the live received-power
//! number so players learn to read a loss budget the way a real OSP tech
//! reads an OTDR printout. Ethernet levels get the constraint checklist
//! instead (see `LevelDef::signal_ledger`).

use crate::level::LevelDef;
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::GameState;
use crate::waifu::FavorPoints;
use bevy::prelude::*;

pub mod neon;

pub struct LedgerUiPlugin;

impl Plugin for LedgerUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_ledger_text
                .run_if(in_state(GameState::Playing).or_else(in_state(GameState::OutageActive))),
        );
    }
}

#[derive(Component)]
pub struct LedgerText;

fn update_ledger_text(
    live: Res<LiveGraph>,
    level: Res<LevelDef>,
    active_outage: Res<ActiveOutage>,
    favor: Res<FavorPoints>,
    mut query: Query<&mut Text, With<LedgerText>>,
) {
    let outage_suffix = active_outage
        .outage
        .as_ref()
        .filter(|o| !o.resolved)
        .map(|o| format!("  |  OUTAGE: {:.0}s left", o.time_remaining()))
        .unwrap_or_default();

    let signal = level.signal_ledger(
        &live.graph,
        live.tx_dbm,
        live.wavelength.0,
        active_outage.outage.as_ref(),
    );
    for mut text in &mut query {
        text.0 = format!("{signal}{outage_suffix}  |  Favor: {}", favor.0);
    }
}
