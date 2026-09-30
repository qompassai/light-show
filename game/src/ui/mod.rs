//! Ledger UI: the running dB budget readout styled like an OTDR trace.
//! Shows each placed component's contribution and the live received-power
//! number so players learn to read a loss budget the way a real OSP tech
//! reads an OTDR printout.

use crate::level::LevelDef;
use crate::states::outage::ActiveOutage;
use crate::states::playing::LiveGraph;
use crate::states::GameState;
use crate::waifu::FavorPoints;
use bevy::prelude::*;

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
    let Ok(result) = live.graph.compute_link_budget_with_outage(
        level.source_node,
        level.target_node,
        live.tx_dbm,
        live.wavelength.0,
        level.receive_window(),
        active_outage.outage.as_ref(),
    ) else {
        return;
    };

    let outage_suffix = active_outage
        .outage
        .as_ref()
        .filter(|o| !o.resolved)
        .map(|o| format!("  |  OUTAGE: {:.0}s left", o.time_remaining()))
        .unwrap_or_default();

    for mut text in &mut query {
        text.sections[0].value = format!(
            "Loss: {:.2} dB  |  Rx: {:.2} dBm  |  Window: [{:.0}, {:.0}] dBm  |  {}{}  |  Favor: {}",
            result.total_loss_db,
            result.received_dbm,
            level.window_min_dbm,
            level.window_max_dbm,
            if result.in_window {
                "IN WINDOW"
            } else {
                "OUT OF WINDOW"
            },
            outage_suffix,
            favor.0
        );
    }
}
