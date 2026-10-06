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
    /// Seconds since the outage was raised. Invariant: always finite and
    /// non-negative. [`Outage::tick`] only accepts finite positive steps,
    /// and deserialization clamps anything else to 0.0 (see
    /// `deserialize_elapsed_non_negative`), so forged or corrupt input can
    /// neither un-expire the outage nor turn accumulated extra loss
    /// negative (which the link budget would then apply as gain).
    #[serde(deserialize_with = "deserialize_elapsed_non_negative")]
    pub elapsed_seconds: f64,
    pub resolved: bool,
}

/// `elapsed_seconds` deserializer: the timer invariant lives here, in one
/// documented place. Only finite, non-negative values are meaningful
/// elapsed time — a negative value would un-expire the outage and drive
/// accumulated extra loss negative, and NaN would poison the timer. Both
/// clamp to 0.0, mirroring [`Outage::tick`]'s guard that only finite
/// positive steps count.
fn deserialize_elapsed_non_negative<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Ok(0.0)
    }
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

    /// Advance the complaint timer. Only finite, positive `dt_seconds`
    /// counts: a negative step would hand back time already lost (even
    /// un-expiring the outage), and NaN would poison `elapsed_seconds`
    /// into instant expiry at full accumulated loss.
    pub fn tick(&mut self, dt_seconds: f64) {
        if !self.resolved && dt_seconds.is_finite() && dt_seconds > 0.0 {
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
        let raw_db = match self.kind {
            OutageKind::WaterIntrusion => (self.elapsed_seconds / 10.0).min(15.0),
            OutageKind::IngressNoise | OutageKind::WirelessInterference => {
                (self.elapsed_seconds / 10.0).min(12.0)
            }
            _ => 0.0,
        };
        // Defense in depth: extra loss is a loss, never gain — clamp the
        // floor at 0 dB. NaN propagates instead of flooring to 0 so corrupt
        // state fails closed downstream rather than routing for free.
        if raw_db.is_nan() {
            f64::NAN
        } else {
            raw_db.max(0.0)
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

    // ---- validation: timer boundaries and loss accumulation ----

    /// ConnectorContamination's base complaint timer.
    const CONTAMINATION_TIMER_S: f64 = 60.0;
    /// WaterIntrusion's accumulated-loss ceiling.
    const WATER_INTRUSION_CAP_DB: f64 = 15.0;

    #[test]
    fn expires_exactly_at_the_timer_boundary() {
        let mut o = Outage::new(OutageKind::ConnectorContamination, 1, 2);
        o.tick(CONTAMINATION_TIMER_S);
        assert_relative_eq_local(o.time_remaining(), 0.0);
        assert!(o.is_expired(), "remaining == 0 must count as expired");
    }

    #[test]
    fn not_expired_just_before_the_boundary() {
        let mut o = Outage::new(OutageKind::ConnectorContamination, 1, 2);
        o.tick(CONTAMINATION_TIMER_S - 0.001);
        assert!(!o.is_expired());
        assert!(o.time_remaining() > 0.0);
    }

    #[test]
    fn time_remaining_clamps_at_zero_long_after_expiry() {
        let mut o = Outage::new(OutageKind::FiberCut, 1, 2);
        o.tick(10_000.0);
        assert_relative_eq_local(o.time_remaining(), 0.0);
    }

    #[test]
    fn water_intrusion_saturates_at_its_cap() {
        let mut o = Outage::new(OutageKind::WaterIntrusion, 1, 2);
        o.tick(1_000.0);
        assert_relative_eq_local(o.accumulated_extra_loss_db(), WATER_INTRUSION_CAP_DB);
    }

    #[test]
    fn static_and_cut_hazards_accumulate_no_extra_loss() {
        for kind in [
            OutageKind::ConnectorContamination,
            OutageKind::Macrobend,
            OutageKind::FiberCut,
            OutageKind::AerialDamage,
        ] {
            let mut o = Outage::new(kind, 1, 2);
            o.tick(100.0);
            assert_relative_eq_local(o.accumulated_extra_loss_db(), 0.0);
        }
    }

    // ---- adversarial: resolution and clock faults ----

    #[test]
    fn resolved_outage_freezes_its_timer_and_never_expires() {
        let mut o = Outage::new(OutageKind::WaterIntrusion, 1, 2);
        o.tick(30.0);
        o.resolved = true;
        o.tick(500.0);
        assert_relative_eq_local(o.elapsed_seconds, 30.0);
        assert!(!o.is_expired());
    }

    #[test]
    fn negative_dt_cannot_rewind_an_expired_outage() {
        // A clock glitch must not hand the player back time they lost.
        let mut o = Outage::new(OutageKind::ConnectorContamination, 1, 2);
        o.tick(CONTAMINATION_TIMER_S);
        o.tick(-30.0);
        assert!(o.is_expired());
        assert_relative_eq_local(o.elapsed_seconds, CONTAMINATION_TIMER_S);
    }

    #[test]
    fn non_finite_dt_is_ignored() {
        // NaN would otherwise poison elapsed_seconds: time_remaining's
        // max(0.0) turns NaN into 0 (instant expiry) and the loss cap's
        // min() turns it into a full 15 dB hit.
        for dt in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut o = Outage::new(OutageKind::WaterIntrusion, 1, 2);
            o.tick(5.0);
            o.tick(dt);
            assert_relative_eq_local(o.elapsed_seconds, 5.0);
            assert!(!o.is_expired(), "dt={dt} expired the outage");
            assert_relative_eq_local(o.accumulated_extra_loss_db(), 0.5);
        }
    }

    #[test]
    fn outage_roundtrips_through_serde() {
        let mut o = Outage::new(OutageKind::IngressNoise, 3, 4);
        o.tick(12.5);
        let json = serde_json::to_string(&o).expect("serialize");
        let back: Outage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.kind, OutageKind::IngressNoise);
        assert_eq!((back.edge_from, back.edge_to), (3, 4));
        assert_relative_eq_local(back.elapsed_seconds, 12.5);
    }

    fn assert_relative_eq_local(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
}
