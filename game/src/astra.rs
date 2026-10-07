//! Astra shared layer: the win-gate conjunction for the optional
//! mechanic blocks (§2b–§2f), the assembly of mechanic-owned
//! verification states (§2a), and — from slice 8 — badge evaluation,
//! closeout assembly, and attempt telemetry (§2g).
//!
//! Every Astra mechanic follows the established extension pattern: an
//! optional `LevelDef` block plus a console under `states/`. This
//! module is the one place the blocks' gates and state lines are
//! combined, so `playing::check_win_condition` and
//! `outage::check_outage_resolution` stay a single conjunction each
//! and the ledger/results surfaces render one identical vector.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::level::{LevelDef, MechanicStates};
use crate::states::identification::{identity_state, IdentificationProgress};

/// All Astra progress resources as one system parameter. The win
/// checks and the results screen are at Bevy's system-param limit;
/// bundling the mechanic resources here means each slice adds a
/// field, never a parameter. Fields are `Option` so headless test
/// worlds that init resources selectively degrade gracefully: a
/// missing resource beside a present block fails that gate closed.
#[derive(SystemParam)]
pub struct AstraProgress<'w> {
    pub ident: Option<Res<'w, IdentificationProgress>>,
}

impl AstraProgress<'_> {
    /// The identification progress, when the resource exists.
    pub fn ident(&self) -> Option<&IdentificationProgress> {
        self.ident.as_deref()
    }
}

/// True when every Astra mechanic gate on this level passes. Levels
/// without a block pass that mechanic vacuously — the conjunction is
/// over *present* blocks only, exactly like the api/triage gates.
/// Identification is superseded on the intermittent level (c1l6),
/// where the §2e composition owns the identity-shaped gate.
pub fn astra_gates_pass(level: &LevelDef, progress: &AstraProgress) -> bool {
    if level.intermittent.is_none() {
        if let Some(def) = &level.identification {
            match progress.ident() {
                Some(ident) if ident.identity_passed(def) => {}
                _ => return false,
            }
        }
    }
    true
}

/// Assemble the mechanic-owned verification states (§2a) from the
/// live progress resources. Board states are derived separately by
/// `LevelDef::verification_states`; this is only the mechanic half,
/// so both surfaces (ledger, results) merge identical lines.
pub fn mechanic_states(level: &LevelDef, ident: Option<&IdentificationProgress>) -> MechanicStates {
    let mut mechanics = MechanicStates::default();
    if level.intermittent.is_none() {
        if let (Some(def), Some(progress)) = (&level.identification, ident) {
            mechanics.identity = Some(identity_state(def, progress));
        }
    }
    mechanics
}
