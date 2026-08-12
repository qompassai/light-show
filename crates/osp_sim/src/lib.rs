//! osp_sim — Outside Plant fiber-optic link-budget simulation core.
//!
//! This crate contains no rendering or engine code. It is the pure "physics"
//! of Light Show: model an OSP path as a graph of components, compute the
//! received optical power at the far end, and evaluate outage hazards.
//! Kept engine-agnostic so it can be unit tested on its own and reused by
//! any future frontend (Bevy today, something else tomorrow).

pub mod component;
pub mod graph;
pub mod outage;
pub mod wavelength;

pub use component::{Component, ConnectorType, SpliceType};
pub use graph::{LinkBudgetResult, PathGraph, PathNode};
pub use outage::{Outage, OutageKind};
pub use wavelength::Wavelength;

/// Standard GPON downstream transmit power range (dBm), per ITU-T G.984.2
/// class B+ optics. Used as the default Point-A launch power unless a level
/// overrides it.
pub const DEFAULT_TX_DBM: f64 = 3.0;

/// A target receive window, e.g. GPON ONT sensitivity range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReceiveWindow {
    pub min_dbm: f64,
    pub max_dbm: f64,
}

impl ReceiveWindow {
    pub const GPON_ONT: ReceiveWindow = ReceiveWindow {
        min_dbm: -27.0,
        max_dbm: -8.0,
    };

    pub fn contains(&self, dbm: f64) -> bool {
        dbm >= self.min_dbm && dbm <= self.max_dbm
    }

    pub fn margin(&self, dbm: f64) -> f64 {
        // Positive = comfortably inside window (distance to nearest edge).
        // Negative = outside window (magnitude = how far outside).
        if self.contains(dbm) {
            (dbm - self.min_dbm).min(self.max_dbm - dbm)
        } else if dbm < self.min_dbm {
            dbm - self.min_dbm
        } else {
            self.max_dbm - dbm
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_contains_and_margin() {
        let w = ReceiveWindow::GPON_ONT;
        assert!(w.contains(-15.0));
        assert!(!w.contains(-30.0));
        assert!(!w.contains(-5.0));
        assert!(w.margin(-15.0) > 0.0);
        assert!(w.margin(-30.0) < 0.0);
    }
}
