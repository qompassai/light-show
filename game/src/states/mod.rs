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

/// Why the game most recently entered `GameState::Results` — set in the
/// same system call as the `NextState` write by whichever system triggers
/// the transition (`playing::check_win_condition` on a clean in-window
/// finish, or `outage::check_outage_resolution` on either a successful
/// repair or a timed-out one), and read by `results::show_results` to
/// pick the win/fail banner, ledger framing, and companion dialogue key.
/// Inserted with an arbitrary initial value at startup — always
/// overwritten before any real transition into `Results`, so the initial
/// value is never actually shown to a player.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LevelOutcome {
    pub won: bool,
}

impl Default for LevelOutcome {
    fn default() -> Self {
        Self { won: true }
    }
}
