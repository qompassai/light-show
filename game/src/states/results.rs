//! Results screen: shows win/fail, final link-budget ledger, and triggers
//! the matching Séraphine reaction line.

use super::GameState;
use bevy::prelude::*;

pub struct ResultsPlugin;

impl Plugin for ResultsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Results), show_results);
    }
}

fn show_results() {
    info!("Level complete — showing results ledger + Seraphine reaction.");
}
