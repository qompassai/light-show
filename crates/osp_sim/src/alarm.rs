//! NOC alarm model: AMS-style alarm lifecycle on top of [`Outage`].
//!
//! An [`Alarm`] wraps an [`Outage`] (which remains the simulation truth for
//! elapsed time, loss accumulation, and resolution) with the operational
//! metadata a Network Operations Center needs: severity, acknowledgement
//! state, and dispatch timestamps. Modeled as data so the same types feed
//! both the simulation core and any frontend alarm list.

use serde::{Deserialize, Serialize};

use crate::outage::{Outage, OutageKind};

/// AMS-style alarm severity. Derived from the outage kind and how far the
/// customer-complaint timer has burned, via [`AlarmSeverity::from_outage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmSeverity {
    /// Full cut: no signal at all reaches the far end. Customer-facing now.
    Critical,
    /// Degrading fast: will breach the receive window or the complaint
    /// timer soon if left alone.
    Major,
    /// Degraded but stable: out of spec, repair at leisure.
    Minor,
    /// Early warning: fault present, no customer impact yet.
    Warning,
}

impl AlarmSeverity {
    /// Derive severity from an outage kind and elapsed seconds.
    ///
    /// Contract: `elapsed_seconds` may be any finite or infinite f64;
    /// negative values clamp to zero urgency (Warning for degrading kinds),
    /// NaN falls through to Warning (the safe default for unknown urgency).
    /// Full cuts ([`OutageKind::is_full_cut`]) are always Critical,
    /// regardless of elapsed time. The match is exhaustive on purpose: a
    /// future new [`OutageKind`] variant must make an explicit severity
    /// choice here rather than silently falling into a default.
    pub fn from_outage(kind: &OutageKind, elapsed_seconds: f64) -> Self {
        // A severed edge carries no signal at all: maximum severity from
        // the first second, no escalation needed.
        if kind.is_full_cut() {
            return AlarmSeverity::Critical;
        }
        // Degrading hazards escalate as the complaint timer burns. Urgency
        // is the fraction of the base timer elapsed; >= 1.0 means expired.
        let urgency = (elapsed_seconds / kind.base_timer_seconds()).max(0.0);
        match kind {
            // Handled by the is_full_cut() guard above; listed explicitly
            // so the match stays exhaustive and a future variant cannot
            // slip through silently.
            OutageKind::FiberCut | OutageKind::AerialDamage | OutageKind::AmplifierFailure => {
                AlarmSeverity::Critical
            }
            // Progressive hazards: accumulated loss climbs the longer they
            // are left (see Outage::accumulated_extra_loss_db), so they
            // escalate faster.
            OutageKind::WaterIntrusion
            | OutageKind::IngressNoise
            | OutageKind::WirelessInterference => {
                if urgency >= 0.65 {
                    AlarmSeverity::Major
                } else if urgency >= 0.30 {
                    AlarmSeverity::Minor
                } else {
                    AlarmSeverity::Warning
                }
            }
            // Static hazards: fixed extra loss that does not grow, but the
            // complaint timer still burns, so they escalate more slowly.
            OutageKind::ConnectorContamination | OutageKind::Macrobend => {
                if urgency >= 0.80 {
                    AlarmSeverity::Major
                } else if urgency >= 0.40 {
                    AlarmSeverity::Minor
                } else {
                    AlarmSeverity::Warning
                }
            }
        }
    }
}

/// AMS alarm acknowledgement lifecycle.
///
/// `New` -> `Acknowledged` -> `Resolved` -> `Cleared`. Transitions only move
/// forward; [`Alarm::acknowledge`] and [`Alarm::sync_from_outage`] enforce
/// this. `Cleared` is terminal: cleared alarms never re-fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlarmAck {
    /// Just raised, flashing in the NOC list. No one owns it yet.
    New,
    /// A companion has been dispatched; the repair timer still runs.
    Acknowledged {
        /// Index into the companion roster (0-based).
        companion_idx: u8,
    },
    /// The underlying outage resolved (player repaired it).
    Resolved,
    /// Archived to history by the player. Terminal.
    Cleared,
}

/// A single NOC alarm: an [`Outage`] plus operational metadata.
///
/// The `outage` field is the simulation truth — elapsed time, loss, and the
/// `resolved` flag all live there. This struct adds the NOC layer: a stable
/// `id` for UI selection, a derived `severity`, the ack lifecycle state,
/// and dispatch timestamps for response-time scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    /// Stable per-session identifier, monotonic from the raising
    /// [`AlarmList`](crate::AlarmList). Unique within a session.
    pub id: u64,
    /// The underlying fault. `outage.resolved` drives the ack lifecycle.
    pub outage: Outage,
    /// Current severity; refreshed by [`Alarm::refresh_severity`] and
    /// [`Alarm::sync_from_outage`] as the outage ages.
    pub severity: AlarmSeverity,
    /// Where this alarm sits in the ack lifecycle.
    pub ack: AlarmAck,
    /// Session seconds when the alarm was raised (for the NOC timeline).
    pub raised_at_secs: f64,
    /// Session seconds when first acknowledged, if ever (for
    /// response-time scoring: `acked_at_secs - raised_at_secs`).
    pub acked_at_secs: Option<f64>,
}

/// Size pin (2026-10-08 struct-size audit): the NOC alarm list
/// holds one `Alarm` per raised outage and re-derives severities
/// from the whole list as outages age.
const _: () = assert!(std::mem::size_of::<Alarm>() <= 64);

impl Alarm {
    /// Raise a new alarm for a fired outage. Starts at [`AlarmAck::New`]
    /// with severity derived from the outage kind at zero elapsed time.
    pub fn new(id: u64, outage: Outage, now_secs: f64) -> Self {
        let severity = AlarmSeverity::from_outage(&outage.kind, outage.elapsed_seconds);
        Self {
            id,
            outage,
            severity,
            ack: AlarmAck::New,
            raised_at_secs: now_secs,
            acked_at_secs: None,
        }
    }

    /// Re-derive severity from the current outage state. Call after
    /// ticking the outage, or use [`Alarm::sync_from_outage`] to also
    /// advance the ack lifecycle.
    pub fn refresh_severity(&mut self) {
        self.severity = AlarmSeverity::from_outage(&self.outage.kind, self.outage.elapsed_seconds);
    }

    /// Acknowledge the alarm and dispatch a companion.
    ///
    /// Idempotent: acknowledging an already-acknowledged alarm is a no-op
    /// (keeps the original `companion_idx` and `acked_at_secs`); acknowledging
    /// a `Resolved` or `Cleared` alarm is also a no-op — a finished fault
    /// cannot be re-dispatched.
    pub fn acknowledge(&mut self, companion_idx: u8, now_secs: f64) {
        if self.ack == AlarmAck::New {
            self.ack = AlarmAck::Acknowledged { companion_idx };
            self.acked_at_secs = Some(now_secs);
        }
    }

    /// Advance the ack lifecycle from the simulation state. Refreshes
    /// severity, then moves `New`/`Acknowledged` to `Resolved` when the
    /// underlying outage resolved. `Resolved` and `Cleared` are left
    /// untouched — a cleared alarm never re-fires through sync.
    pub fn sync_from_outage(&mut self) {
        self.refresh_severity();
        if self.outage.resolved && matches!(self.ack, AlarmAck::New | AlarmAck::Acknowledged { .. })
        {
            self.ack = AlarmAck::Resolved;
        }
    }

    /// Mark a resolved alarm as cleared (archived to history). Only
    /// `Resolved` alarms can be cleared; anything else is a no-op.
    pub fn clear(&mut self) {
        if self.ack == AlarmAck::Resolved {
            self.ack = AlarmAck::Cleared;
        }
    }

    /// Seconds from raise to first ack, if the alarm was acknowledged.
    pub fn response_time_secs(&self) -> Option<f64> {
        self.acked_at_secs
            .map(|acked| (acked - self.raised_at_secs).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outage(kind: OutageKind) -> Outage {
        Outage::new(kind, 1, 2)
    }

    // ---- validation: severity derivation ----

    #[test]
    fn full_cuts_are_critical_at_any_age() {
        for kind in [
            OutageKind::FiberCut,
            OutageKind::AerialDamage,
            OutageKind::AmplifierFailure,
        ] {
            assert_eq!(
                AlarmSeverity::from_outage(&kind, 0.0),
                AlarmSeverity::Critical
            );
            // Even one second before the complaint timer expires.
            assert_eq!(
                AlarmSeverity::from_outage(&kind, kind.base_timer_seconds() - 1.0),
                AlarmSeverity::Critical
            );
            // Even long after expiry.
            assert_eq!(
                AlarmSeverity::from_outage(&kind, kind.base_timer_seconds() * 3.0),
                AlarmSeverity::Critical
            );
        }
    }

    #[test]
    fn progressive_hazard_escalates_warning_minor_major() {
        let kind = OutageKind::WaterIntrusion; // 120s timer
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 0.0),
            AlarmSeverity::Warning
        );
        // 0.30 * 120 = 36s -> Minor
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 36.0),
            AlarmSeverity::Minor
        );
        // 0.65 * 120 = 78s -> Major
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 78.0),
            AlarmSeverity::Major
        );
        // Past expiry -> still Major (never Critical for degrading kinds)
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 240.0),
            AlarmSeverity::Major
        );
    }

    #[test]
    fn static_hazard_escalates_more_slowly() {
        let kind = OutageKind::ConnectorContamination; // 60s timer
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 0.0),
            AlarmSeverity::Warning
        );
        // 0.40 * 60 = 24s -> Minor (later than the progressive 0.30)
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 23.9),
            AlarmSeverity::Warning
        );
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 24.0),
            AlarmSeverity::Minor
        );
        // 0.80 * 60 = 48s -> Major
        assert_eq!(
            AlarmSeverity::from_outage(&kind, 48.0),
            AlarmSeverity::Major
        );
    }

    #[test]
    fn new_alarm_starts_unacked_with_derived_severity() {
        let alarm = Alarm::new(7, outage(OutageKind::FiberCut), 100.0);
        assert_eq!(alarm.id, 7);
        assert_eq!(alarm.severity, AlarmSeverity::Critical);
        assert_eq!(alarm.ack, AlarmAck::New);
        assert_eq!(alarm.raised_at_secs, 100.0);
        assert_eq!(alarm.acked_at_secs, None);
        assert_eq!(alarm.response_time_secs(), None);
    }

    #[test]
    fn acknowledge_dispatches_companion_and_records_time() {
        let mut alarm = Alarm::new(1, outage(OutageKind::WaterIntrusion), 50.0);
        alarm.acknowledge(2, 63.5);
        assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 2 });
        assert_eq!(alarm.acked_at_secs, Some(63.5));
        assert_eq!(alarm.response_time_secs(), Some(13.5));
    }

    #[test]
    fn sync_resolves_when_outage_resolves() {
        let mut alarm = Alarm::new(1, outage(OutageKind::Macrobend), 0.0);
        alarm.acknowledge(0, 5.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        assert_eq!(alarm.ack, AlarmAck::Resolved);
    }

    #[test]
    fn sync_resolves_unacked_alarm_too() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        assert_eq!(alarm.ack, AlarmAck::Resolved);
    }

    #[test]
    fn clear_archives_resolved_alarm() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        alarm.clear();
        assert_eq!(alarm.ack, AlarmAck::Cleared);
    }

    #[test]
    fn severity_refreshes_as_outage_ages() {
        let mut alarm = Alarm::new(1, outage(OutageKind::WaterIntrusion), 0.0);
        assert_eq!(alarm.severity, AlarmSeverity::Warning);
        alarm.outage.tick(80.0); // past the 0.65 Major threshold
        alarm.refresh_severity();
        assert_eq!(alarm.severity, AlarmSeverity::Major);
    }

    // ---- adversarial: lifecycle guards ----

    #[test]
    fn acknowledge_is_idempotent() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 10.0);
        alarm.acknowledge(1, 20.0);
        // Second ack with a different companion and time: no-op.
        alarm.acknowledge(3, 99.0);
        assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 1 });
        assert_eq!(alarm.acked_at_secs, Some(20.0));
        assert_eq!(alarm.response_time_secs(), Some(10.0));
    }

    #[test]
    fn acknowledge_resolved_alarm_is_noop() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        assert_eq!(alarm.ack, AlarmAck::Resolved);
        alarm.acknowledge(2, 50.0);
        assert_eq!(alarm.ack, AlarmAck::Resolved);
        assert_eq!(alarm.acked_at_secs, None);
    }

    #[test]
    fn acknowledge_cleared_alarm_is_noop() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        alarm.clear();
        alarm.acknowledge(0, 50.0);
        assert_eq!(alarm.ack, AlarmAck::Cleared);
    }

    #[test]
    fn sync_never_resurrects_cleared_alarm() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        alarm.clear();
        // Even if the sim somehow un-resolves, the cleared alarm stays cleared.
        alarm.outage.resolved = false;
        alarm.sync_from_outage();
        assert_eq!(alarm.ack, AlarmAck::Cleared);
    }

    #[test]
    fn clear_only_works_on_resolved() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.clear(); // New -> no-op
        assert_eq!(alarm.ack, AlarmAck::New);
        alarm.acknowledge(0, 5.0);
        alarm.clear(); // Acknowledged -> no-op
        assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 0 });
    }

    #[test]
    fn negative_elapsed_clamps_to_warning() {
        // Defensive: a clock glitch must not panic or produce Critical.
        assert_eq!(
            AlarmSeverity::from_outage(&OutageKind::WaterIntrusion, -100.0),
            AlarmSeverity::Warning
        );
        // Full cuts are unaffected by the clamp.
        assert_eq!(
            AlarmSeverity::from_outage(&OutageKind::FiberCut, -100.0),
            AlarmSeverity::Critical
        );
    }

    #[test]
    fn response_time_never_negative() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 100.0);
        // Ack timestamp before raise (clock skew): clamped to 0.
        alarm.acknowledge(0, 50.0);
        assert_eq!(alarm.response_time_secs(), Some(0.0));
    }

    #[test]
    fn alarm_roundtrips_through_serde() {
        let mut alarm = Alarm::new(42, outage(OutageKind::IngressNoise), 7.5);
        alarm.acknowledge(3, 9.0);
        let json = serde_json::to_string(&alarm).expect("serialize");
        let back: Alarm = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, 42);
        assert_eq!(back.ack, AlarmAck::Acknowledged { companion_idx: 3 });
        assert_eq!(back.raised_at_secs, 7.5);
    }

    // ---- validation: threshold edges and sync without resolution ----

    /// WirelessInterference's 100 s timer makes urgency == elapsed / 100.
    const INTERFERENCE_TIMER_S: f64 = 100.0;
    /// Just under a threshold, in seconds of elapsed time.
    const JUST_BEFORE_S: f64 = 0.01;

    #[test]
    fn progressive_thresholds_are_inclusive_lower_bounds() {
        let kind = OutageKind::WirelessInterference;
        assert_eq!(kind.base_timer_seconds(), INTERFERENCE_TIMER_S);
        let at = |secs: f64| AlarmSeverity::from_outage(&kind, secs);
        assert_eq!(at(30.0 - JUST_BEFORE_S), AlarmSeverity::Warning);
        assert_eq!(at(30.0), AlarmSeverity::Minor);
        assert_eq!(at(65.0 - JUST_BEFORE_S), AlarmSeverity::Minor);
        assert_eq!(at(65.0), AlarmSeverity::Major);
    }

    #[test]
    fn macrobend_uses_the_static_thresholds() {
        let kind = OutageKind::Macrobend; // 60 s timer: Minor at 24, Major at 48
        let at = |secs: f64| AlarmSeverity::from_outage(&kind, secs);
        assert_eq!(at(47.9), AlarmSeverity::Minor);
        assert_eq!(at(48.0), AlarmSeverity::Major);
        // A progressive hazard at the same urgency (0.30) would already
        // be Minor; static hazards are still Warning there.
        assert_eq!(at(18.0), AlarmSeverity::Warning);
    }

    #[test]
    fn new_alarm_from_a_pre_aged_outage_reflects_its_elapsed_time() {
        let mut aged = outage(OutageKind::WaterIntrusion);
        aged.tick(100.0); // urgency 0.83 → Major
        let alarm = Alarm::new(1, aged, 100.0);
        assert_eq!(alarm.severity, AlarmSeverity::Major);
    }

    #[test]
    fn sync_escalates_severity_without_resolving() {
        let mut alarm = Alarm::new(1, outage(OutageKind::IngressNoise), 0.0);
        alarm.acknowledge(1, 2.0);
        alarm.outage.tick(100.0);
        alarm.sync_from_outage();
        assert_eq!(alarm.severity, AlarmSeverity::Major);
        assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 1 });
    }

    #[test]
    fn expired_but_unresolved_alarm_can_still_be_dispatched() {
        let mut alarm = Alarm::new(1, outage(OutageKind::Macrobend), 0.0);
        alarm.outage.tick(500.0);
        alarm.sync_from_outage();
        assert!(alarm.outage.is_expired());
        alarm.acknowledge(2, 500.0);
        assert_eq!(alarm.ack, AlarmAck::Acknowledged { companion_idx: 2 });
    }

    // ---- adversarial: non-finite clocks and lifecycle races ----

    #[test]
    fn nan_elapsed_falls_back_to_warning() {
        assert_eq!(
            AlarmSeverity::from_outage(&OutageKind::WaterIntrusion, f64::NAN),
            AlarmSeverity::Warning
        );
        assert_eq!(
            AlarmSeverity::from_outage(&OutageKind::Macrobend, f64::NAN),
            AlarmSeverity::Warning
        );
    }

    #[test]
    fn infinite_elapsed_caps_degrading_kinds_at_major() {
        for kind in [OutageKind::IngressNoise, OutageKind::ConnectorContamination] {
            assert_eq!(
                AlarmSeverity::from_outage(&kind, f64::INFINITY),
                AlarmSeverity::Major
            );
        }
    }

    #[test]
    fn resolved_alarm_stays_resolved_if_outage_flaps_before_clear() {
        let mut alarm = Alarm::new(1, outage(OutageKind::WaterIntrusion), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        // Sim flaps back to unresolved before the player archives it.
        alarm.outage.resolved = false;
        alarm.sync_from_outage();
        assert_eq!(alarm.ack, AlarmAck::Resolved);
        alarm.clear();
        assert_eq!(alarm.ack, AlarmAck::Cleared);
    }

    #[test]
    fn resolved_alarm_severity_does_not_keep_escalating() {
        let mut alarm = Alarm::new(1, outage(OutageKind::WaterIntrusion), 0.0);
        alarm.outage.tick(10.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        let frozen = alarm.severity;
        // The outage's timer is frozen once resolved, so later frames
        // cannot push a finished alarm up the list.
        alarm.outage.tick(1_000.0);
        alarm.sync_from_outage();
        assert_eq!(alarm.severity, frozen);
        assert_eq!(frozen, AlarmSeverity::Warning);
    }

    #[test]
    fn clear_is_idempotent_and_resync_is_inert() {
        let mut alarm = Alarm::new(1, outage(OutageKind::FiberCut), 0.0);
        alarm.outage.resolved = true;
        alarm.sync_from_outage();
        alarm.clear();
        alarm.clear();
        alarm.sync_from_outage();
        assert_eq!(alarm.ack, AlarmAck::Cleared);
        assert_eq!(alarm.acked_at_secs, None);
    }

    #[test]
    fn unknown_ack_state_is_rejected_by_serde() {
        let parsed: Result<AlarmAck, _> = serde_json::from_str("\"Snoozed\"");
        assert!(parsed.is_err());
    }
}
