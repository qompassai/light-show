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
    /// A coax line amplifier fails outright: the cascade loses its gain
    /// and the downstream level collapses — a full cut for the purposes
    /// of the win check.
    AmplifierFailure,
    /// Ingress noise on the coax plant (bad shielding, loose connectors):
    /// the noise floor climbs steadily, so a level that started in-window
    /// drifts out unless the player rebalances gain.
    IngressNoise,
    /// RF interference on a wireless link (a new transmitter, weather):
    /// raises the effective noise floor the same way ingress does on
    /// coax, but on Linka's medium.
    WirelessInterference,
}

impl OutageKind {
    /// Whether this hazard fully severs the affected edge (no signal at
    /// all gets through until rerouted/repaired) versus merely degrading
    /// it (edge stays connected but loses extra dB the longer it's left
    /// unresolved). Exhaustive match on purpose: a future new variant
    /// must make an explicit choice here rather than silently falling
    /// into a wildcard default.
    pub fn is_full_cut(&self) -> bool {
        match self {
            OutageKind::FiberCut | OutageKind::AerialDamage | OutageKind::AmplifierFailure => true,
            OutageKind::WaterIntrusion
            | OutageKind::ConnectorContamination
            | OutageKind::Macrobend
            | OutageKind::IngressNoise
            | OutageKind::WirelessInterference => false,
        }
    }

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
            OutageKind::AmplifierFailure => {
                "A line amplifier just died — the cascade lost its gain."
            }
            OutageKind::IngressNoise => {
                "Ingress noise is climbing — the floor is rising under your signal."
            }
            OutageKind::WirelessInterference => {
                "Interference is stomping on the link — the floor is rising."
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
            OutageKind::AmplifierFailure => 90.0,
            OutageKind::IngressNoise => 110.0,
            OutageKind::WirelessInterference => 100.0,
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

    /// For degrading hazards, loss grows the longer the outage is
    /// unresolved — this is the value to add on top of the link budget.
    /// Water intrusion climbs fastest (15 dB cap); ingress noise and
    /// wireless interference climb a little slower (12 dB cap), which is
    /// what gives the coax/wireless repair levels their rebalancing
    /// window.
    pub fn accumulated_extra_loss_db(&self) -> f64 {
        match self.kind {
            OutageKind::WaterIntrusion => (self.elapsed_seconds / 10.0).min(15.0),
            OutageKind::IngressNoise | OutageKind::WirelessInterference => {
                (self.elapsed_seconds / 10.0).min(12.0)
            }
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_cut_classification_is_exhaustive_and_correct() {
        assert!(OutageKind::FiberCut.is_full_cut());
        assert!(OutageKind::AerialDamage.is_full_cut());
        assert!(OutageKind::AmplifierFailure.is_full_cut());
        assert!(!OutageKind::WaterIntrusion.is_full_cut());
        assert!(!OutageKind::ConnectorContamination.is_full_cut());
        assert!(!OutageKind::Macrobend.is_full_cut());
        assert!(!OutageKind::IngressNoise.is_full_cut());
        assert!(!OutageKind::WirelessInterference.is_full_cut());
    }

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

    #[test]
    fn degrade_hazards_climb_and_cap() {
        let mut ingress = Outage::new(OutageKind::IngressNoise, 1, 2);
        ingress.tick(50.0);
        assert_relative_eq_local(ingress.accumulated_extra_loss_db(), 5.0);

        let mut interference = Outage::new(OutageKind::WirelessInterference, 1, 2);
        interference.tick(200.0);
        assert_relative_eq_local(interference.accumulated_extra_loss_db(), 12.0);

        let cut = Outage::new(OutageKind::AmplifierFailure, 1, 2);
        assert_relative_eq_local(cut.accumulated_extra_loss_db(), 0.0);
    }

    #[test]
    fn every_kind_has_flavor_text_and_a_timer() {
        for kind in [
            OutageKind::FiberCut,
            OutageKind::AerialDamage,
            OutageKind::WaterIntrusion,
            OutageKind::ConnectorContamination,
            OutageKind::Macrobend,
            OutageKind::AmplifierFailure,
            OutageKind::IngressNoise,
            OutageKind::WirelessInterference,
        ] {
            assert!(!kind.flavor_text().is_empty());
            assert!(kind.base_timer_seconds() > 0.0);
        }
    }

    fn assert_relative_eq_local(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
}
