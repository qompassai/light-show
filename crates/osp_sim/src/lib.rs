//! osp_sim — Outside Plant link-budget simulation core.
//!
//! This crate contains no rendering or engine code. It is the pure "physics"
//! of Light Show: model an OSP path as a graph of components, compute the
//! received signal at the far end, and evaluate outage hazards. Fiber,
//! coax, and wireless share the dB-budget machinery (see
//! [`Medium`](crate::medium::Medium)); Ethernet is evaluated against
//! structured-cabling constraints instead. Kept engine-agnostic so it can
//! be unit tested on its own and reused by any future frontend (Bevy today,
//! something else tomorrow).

pub mod alarm;
pub mod component;
pub mod graph;
pub mod medium;
pub mod outage;
pub mod wavelength;

pub use alarm::{Alarm, AlarmAck, AlarmSeverity};
pub use component::{
    free_space_path_loss_db, CableCategory, Component, ConnectorType, SpliceType,
    COAX_LOSS_DB_PER_M,
};
pub use graph::{
    CoaxEval, CoaxViolation, EthernetEval, EthernetViolation, LinkBudgetResult, PathGraph,
    PathNode, WirelessEval, WirelessViolation,
};
pub use medium::Medium;
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

    // ---- validation: window / margin math ----

    #[test]
    fn margin_inside_is_distance_to_nearest_edge() {
        // GPON ONT window is −27 … −8 dBm. At −15: 12 dB above the floor,
        // 7 dB below the ceiling → margin 7.
        let w = ReceiveWindow::GPON_ONT;
        assert_eq!(w.margin(-15.0), 7.0);
        // −25: 2 dB above the floor is the binding edge.
        assert_eq!(w.margin(-25.0), 2.0);
    }

    #[test]
    fn window_edges_are_inclusive_with_zero_margin() {
        let w = ReceiveWindow::GPON_ONT;
        assert!(w.contains(w.min_dbm));
        assert!(w.contains(w.max_dbm));
        assert_eq!(w.margin(w.min_dbm), 0.0);
        assert_eq!(w.margin(w.max_dbm), 0.0);
    }

    #[test]
    fn margin_outside_is_signed_overshoot() {
        let w = ReceiveWindow::GPON_ONT;
        assert_eq!(w.margin(-30.0), -3.0); // 3 dB too weak
        assert_eq!(w.margin(-5.0), -3.0); // 3 dB too hot
    }

    // ---- adversarial: non-finite levels must fail closed ----

    #[test]
    fn nan_level_is_never_in_window_or_comfortable() {
        let w = ReceiveWindow::GPON_ONT;
        assert!(!w.contains(f64::NAN));
        let margin = w.margin(f64::NAN);
        assert!(
            margin.is_nan() || margin <= 0.0,
            "NaN reported margin {margin}"
        );
    }

    #[test]
    fn infinite_levels_are_out_of_window_with_infinite_deficit() {
        let w = ReceiveWindow::GPON_ONT;
        for level in [f64::INFINITY, f64::NEG_INFINITY] {
            assert!(!w.contains(level));
            assert_eq!(w.margin(level), f64::NEG_INFINITY);
        }
    }

    #[test]
    fn inverted_window_contains_nothing() {
        // Adversarial: a level file with min/max swapped must not accept
        // any received level.
        let w = ReceiveWindow {
            min_dbm: -8.0,
            max_dbm: -27.0,
        };
        for level in [-30.0, -27.0, -15.0, -8.0, 0.0] {
            assert!(!w.contains(level), "{level} accepted by inverted window");
        }
    }
}
