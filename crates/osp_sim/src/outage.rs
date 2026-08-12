//! Outage/hazard events: mid-level faults the player must diagnose and
//! repair before a timer expires. Modeled as data so levels can script
//! specific outages, and so the same event feeds both simulation and UI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutageKind {
    /// Backhoe / dig-up severs a buried span outright — total loss on that
    /// edge, must be rerouted or emergency-spliced.
    FiberCut,
    /// Storm/branch strike on aerial plant — usually reroutable via a
    /// protection path if one was pre-built.
    AerialDamage,
    /// Splice closure floods; loss climbs steadily until repaired.
    WaterIntrusion,
    /// Field tech (or the player) leaves a connector dirty/scratched.
    ConnectorContamination,
    /// Someone staples/kinks a drop cable below minimum bend radius.
    Macrobend,
}

impl OutageKind {
    pub fn flavor_text(&self) -> &'static str {
        match self {
            OutageKind::FiberCut => "Backhoe strike! Buried span severed near marker 14+00.",
            OutageKind::AerialDamage => {
                "Storm knocked a limb across the aerial span — strand is intact, fiber isn't."
            }
            OutageKind::WaterIntrusion => {
                "Splice closure gasket failed. Water's creeping in and loss is climbing."
            }
            OutageKind::ConnectorContamination => {
                "Someone mated a connector without inspecting it first. Classic."
            }
            OutageKind::Macrobend => {
                "Drop cable stapled way under minimum bend radius. It's basically whispering now."
            }
        }
    }

    /// Baseline seconds before the "customer complaint" timer fires, before
    /// level-specific modifiers.
    pub fn base_timer_seconds(&self) -> f64 {
        match self {
            OutageKind::FiberCut => 90.0,
            OutageKind::AerialDamage => 75.0,
            OutageKind::WaterIntrusion => 120.0,
            OutageKind::ConnectorContamination => 60.0,
            OutageKind::Macrobend => 60.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outage {
    pub kind: OutageKind,
    pub edge_from: u32,
    pub edge_to: u32,
    pub elapsed_seconds: f64,
    pub resolved: bool,
}

impl Outage {
    pub fn new(kind: OutageKind, edge_from: u32, edge_to: u32) -> Self {
        Self {
            kind,
            edge_from,
            edge_to,
            elapsed_seconds: 0.0,
            resolved: false,
        }
    }

    pub fn tick(&mut self, dt_seconds: f64) {
        if !self.resolved {
            self.elapsed_seconds += dt_seconds;
        }
    }

    pub fn time_remaining(&self) -> f64 {
        (self.kind.base_timer_seconds() - self.elapsed_seconds).max(0.0)
    }

    pub fn is_expired(&self) -> bool {
        !self.resolved && self.time_remaining() <= 0.0
    }

    /// For water intrusion specifically, loss grows the longer it's
    /// unresolved — this is the value to add to the affected splice's
    /// `degradation_db`.
    pub fn accumulated_extra_loss_db(&self) -> f64 {
        match self.kind {
            OutageKind::WaterIntrusion => (self.elapsed_seconds / 10.0).min(15.0),
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_intrusion_worsens_over_time() {
        let mut o = Outage::new(OutageKind::WaterIntrusion, 1, 2);
        o.tick(30.0);
        assert_relative_eq_local(o.accumulated_extra_loss_db(), 3.0);
        assert!(!o.is_expired());
    }

    #[test]
    fn expires_after_base_timer() {
        let mut o = Outage::new(OutageKind::ConnectorContamination, 1, 2);
        o.tick(61.0);
        assert!(o.is_expired());
    }

    fn assert_relative_eq_local(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
}
