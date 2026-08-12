//! Top-level game state machine. Each variant owns its own plugin (see
//! sibling modules) so systems are only scheduled while that state is active.

pub mod menu;
pub mod outage;
pub mod playing;
pub mod results;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    /// Normal puzzle solving: player is routing fiber, no active outage.
    Playing,
    /// An outage event has fired; timer is running and the ledger shows the
    /// fault location until resolved.
    OutageActive,
    /// Level finished (win or fail) — shows Séraphine's reaction + summary.
    Results,
}
