//! Puzzle-piece components. Each variant carries the real-world loss range
//! it's drawn from; specific instances pick a concrete value (sometimes
//! randomized within range to create hazard variance, e.g. dirty
//! connectors).

use crate::wavelength::Wavelength;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpliceType {
    /// Fusion splice: melts two fiber ends together. Lowest loss, but slow
    /// (requires a fusion splicer prop and "prep time" resource in-level).
    Fusion,
    /// Mechanical splice: aligns and clamps fiber ends with gel/index-
    /// matching fluid. Faster to place (useful under outage time pressure)
    /// but higher, less consistent loss.
    Mechanical,
}

impl SpliceType {
    /// Typical insertion loss in dB (midpoint of real-world range; level
    /// data may add jitter).
    pub fn typical_loss_db(&self) -> f64 {
        match self {
            SpliceType::Fusion => 0.075,
            SpliceType::Mechanical => 0.4,
        }
    }

    pub fn place_time_seconds(&self) -> f64 {
        match self {
            SpliceType::Fusion => 45.0,
            SpliceType::Mechanical => 10.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorType {
    /// Ultra Physical Contact — flat polish, ~-50 dB return loss.
    Upc,
    /// Angled Physical Contact — 8° angled polish, ~-60 dB return loss,
    /// required wherever the level flags "reflectance sensitive" (e.g.
    /// RF video overlay, high-bandwidth XGS-PON legs).
    Apc,
}

impl ConnectorType {
    pub fn typical_loss_db(&self) -> f64 {
        match self {
            ConnectorType::Upc => 0.35,
            ConnectorType::Apc => 0.30,
        }
    }

    pub fn typical_return_loss_db(&self) -> f64 {
        match self {
            ConnectorType::Upc => -50.0,
            ConnectorType::Apc => -60.0,
        }
    }
}

/// A splitter: divides one input into N outputs for PON fan-out levels.
/// Values are real-world PLC splitter insertion-loss figures (includes
/// intrinsic splitting loss + excess loss).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitterRatio {
    OneByTwo,
    OneByFour,
    OneByEight,
    OneBySixteen,
    OneByThirtyTwo,
}

impl SplitterRatio {
    pub fn insertion_loss_db(&self) -> f64 {
        match self {
            SplitterRatio::OneByTwo => 3.6,
            SplitterRatio::OneByFour => 7.3,
            SplitterRatio::OneByEight => 10.6,
            SplitterRatio::OneBySixteen => 13.7,
            SplitterRatio::OneByThirtyTwo => 17.7,
        }
    }

    pub fn branch_count(&self) -> u32 {
        match self {
            SplitterRatio::OneByTwo => 2,
            SplitterRatio::OneByFour => 4,
            SplitterRatio::OneByEight => 8,
            SplitterRatio::OneBySixteen => 16,
            SplitterRatio::OneByThirtyTwo => 32,
        }
    }
}

/// A single edge/segment in the OSP path graph. Every level is built from a
/// sequence (and, for PON levels, a branching tree) of these.
///
/// The set spans all four transmission media (see
/// [`Medium`](crate::medium::Medium)): fiber keeps the original
/// splice/connector/splitter set, coax adds amplifiers/taps/coax spans,
/// wireless adds free-space hops and regenerative repeaters, and Ethernet
/// adds runs and switches for constraint-based evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Component {
    /// A run of fiber cable of the given length, in an aerial, buried, or
    /// conduit plant type (plant type currently cosmetic/hazard-flavor, but
    /// reserved for future weather/dig-hazard weighting).
    Span {
        length_km: f64,
        plant: PlantType,
    },
    Splice {
        kind: SpliceType,
        /// Extra loss added by a mid-level hazard (e.g. incipient water
        /// intrusion raising loss over time). 0.0 under normal conditions.
        degradation_db: f64,
    },
    Connector {
        kind: ConnectorType,
        /// Contamination adds loss on top of the nominal figure; a "clean
        /// the connector" interaction can zero this back out.
        contamination_db: f64,
    },
    Splitter {
        ratio: SplitterRatio,
    },
    /// A macrobend/kink hazard the player introduced (or the level pre-
    /// seeded as a fault): visualized as a tight kink in the routed line.
    Macrobend {
        excess_loss_db: f64,
    },
    /// A coax line amplifier: adds `gain_db` (typically 15–30 dB) to the
    /// running level. Unlike fiber plant, coax cascades routinely include
    /// gain — the design puzzle is choosing gain that lands inside the
    /// receive window without overdriving into distortion.
    Amplifier {
        gain_db: f64,
    },
    /// A coax directional tap: the drop toward the target leaves via the
    /// tap port with `tap_loss_db` insertion loss. (The thru port that
    /// continues the cascade is not modeled — single-path levels always
    /// take the tap port toward the target.)
    Tap {
        tap_loss_db: f64,
    },
    /// A run of 75-ohm coax of the given length in meters.
    CoaxSpan {
        length_m: f64,
    },
    /// A wireless hop of `distance_m` meters at `frequency_mhz` MHz.
    /// Loss is free-space path loss (see [`free_space_path_loss_db`]).
    WirelessHop {
        distance_m: f64,
        frequency_mhz: f64,
    },
    /// A regenerative (decode-and-forward) repeater: it retransmits at its
    /// own `tx_dbm`, so the running level resets here — the original
    /// signal's history ends at this edge. This is what distinguishes it
    /// from an [`Component::Amplifier`], which only boosts whatever
    /// arrives (noise and all).
    Repeater {
        tx_dbm: f64,
    },
    /// A copper Ethernet run of `length_m` meters. Evaluated by
    /// [`PathGraph::evaluate_ethernet`](crate::graph::PathGraph::evaluate_ethernet),
    /// not the dB budget path.
    EthernetRun {
        length_m: f64,
        category: CableCategory,
    },
    /// An Ethernet switch placed mid-path: terminates the current copper
    /// segment (every segment must respect the length limit) and
    /// contributes `poe_budget_w` watts of PoE budget for powered devices
    /// downstream.
    Switch {
        poe_budget_w: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantType {
    Aerial,
    Buried,
    Conduit,
}

/// Attenuation of 75-ohm coax (RG-6 class) at ~750 MHz, in dB per meter.
/// The real figure is 5–6 dB per 100 m; we take the midpoint.
pub const COAX_LOSS_DB_PER_M: f64 = 0.055;

/// Copper cable categories for Ethernet runs, with rated bandwidths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CableCategory {
    Cat5e,
    Cat6,
}

impl CableCategory {
    /// Rated bandwidth in Mbps (1000BASE-T / 10GBASE-T short reach).
    pub fn bandwidth_mbps(&self) -> u64 {
        match self {
            CableCategory::Cat5e => 1_000,
            CableCategory::Cat6 => 10_000,
        }
    }
}

/// Free-space path loss in dB for `distance_m` meters at `frequency_mhz`
/// MHz: 20·log₁₀(d) + 20·log₁₀(f) − 27.55. The inverse-square law in dB
/// clothing — doubling the distance always costs ~6 dB, at any frequency.
pub fn free_space_path_loss_db(distance_m: f64, frequency_mhz: f64) -> f64 {
    20.0 * distance_m.log10() + 20.0 * frequency_mhz.log10() - 27.55
}

/// NaN-aware `f64::max(value, 0.0)`: a negative length is an authoring
/// error that floors to zero loss, but NaN is corrupt data — `f64::max`
/// silently returns the 0.0 floor for NaN, so NaN propagates instead and
/// the budget fails closed downstream.
fn non_negative_or_nan(value: f64) -> f64 {
    if value.is_nan() {
        f64::NAN
    } else {
        value.max(0.0)
    }
}

/// Clamp a passive component's total loss at 0 dB: passive plant may lose
/// power but never add it, so a finite negative total from hostile level
/// data is an authoring error, not gain. Non-finite totals (NaN, ±inf)
/// propagate instead of flooring to 0 — they are corrupt data, and the
/// budget fails closed on them downstream (−inf floored to 0 dB would let
/// an infinitely-gained link pass).
fn passive_loss_db(total_db: f64) -> f64 {
    if total_db.is_finite() {
        total_db.max(0.0)
    } else {
        total_db
    }
}

impl Component {
    /// Loss this component contributes at the given wavelength, in dB.
    /// Gain components report negative loss so per-component ledger rows
    /// stay arithmetically honest. Passive components are clamped at 0 dB:
    /// passive plant may lose power but never add it — a negative total
    /// from hostile level data is an authoring error, not gain. The sole
    /// exception is [`Component::Amplifier`], whose negative loss is
    /// intentional, designed-in gain (coax cascades are engineered around
    /// it); every other component that computes to negative loss is
    /// floored at zero. Only meaningful for the dB-budget media (fiber,
    /// coax, wireless): Ethernet components report 0.0 here and are
    /// evaluated by `evaluate_ethernet` instead, and `Repeater` reports 0.0
    /// because its effect (regeneration) lives in
    /// [`Component::apply_level`].
    ///
    /// NaN inputs propagate as NaN loss: `f64::max` silently drops a NaN
    /// operand, so every floor and clamp below checks NaN first. A NaN
    /// loss fails closed downstream — [`ReceiveWindow::contains`] is false
    /// for NaN and `margin` is NaN — instead of routing a corrupt link for
    /// free.
    pub fn loss_db(&self, wavelength: Wavelength) -> f64 {
        match self {
            Component::Span { length_km, .. } => {
                non_negative_or_nan(*length_km) * wavelength.attenuation_db_per_km()
            }
            Component::Splice {
                kind,
                degradation_db,
            } => passive_loss_db(kind.typical_loss_db() + degradation_db),
            Component::Connector {
                kind,
                contamination_db,
            } => passive_loss_db(kind.typical_loss_db() + contamination_db),
            Component::Splitter { ratio } => ratio.insertion_loss_db(),
            Component::Macrobend { excess_loss_db } => passive_loss_db(*excess_loss_db),
            Component::Amplifier { gain_db } => -gain_db,
            Component::Tap { tap_loss_db } => passive_loss_db(*tap_loss_db),
            Component::CoaxSpan { length_m } => non_negative_or_nan(*length_m) * COAX_LOSS_DB_PER_M,
            Component::WirelessHop {
                distance_m,
                frequency_mhz,
            } => {
                // A NaN distance or frequency is corrupt level data: fail
                // closed with NaN loss. The floors below must not see it —
                // f64::max silently keeps the non-NaN operand, which would
                // turn a corrupt hop into a free near-field 0 dB hop.
                if distance_m.is_nan() || frequency_mhz.is_nan() {
                    f64::NAN
                } else {
                    // Floor the distance at 0.1 m: log10(0) is −inf, and a
                    // zero-length hop is an authoring error, not −infinite
                    // gain. Floor the loss at 0 dB too: a 0 MHz hop hits
                    // the same −inf, and in the near field the formula goes
                    // negative — free space is passive either way.
                    free_space_path_loss_db(distance_m.max(0.1), *frequency_mhz).max(0.0)
                }
            }
            Component::Repeater { .. } => 0.0,
            Component::EthernetRun { .. } | Component::Switch { .. } => 0.0,
        }
    }

    /// Advance a running signal level (dBm for fiber/wireless, dBmV for
    /// coax) through this component. Passive components subtract loss,
    /// amplifiers add gain, and a repeater regenerates: the running level
    /// is replaced by the repeater's own transmit power.
    pub fn apply_level(&self, level: f64, wavelength: Wavelength) -> f64 {
        match self {
            Component::Repeater { tx_dbm } => *tx_dbm,
            _ => level - self.loss_db(wavelength),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn amplifier_gain_is_negative_loss() {
        let amp = Component::Amplifier { gain_db: 20.0 };
        assert_relative_eq!(
            amp.apply_level(10.0, Wavelength::Nm1490),
            30.0,
            epsilon = 1e-9
        );
        assert_relative_eq!(amp.loss_db(Wavelength::Nm1490), -20.0, epsilon = 1e-9);
    }

    #[test]
    fn negative_gain_acts_as_an_attenuator_without_panicking() {
        // Adversarial: a misconfigured amp is just a pad, not a crash.
        let amp = Component::Amplifier { gain_db: -5.0 };
        assert_relative_eq!(
            amp.apply_level(10.0, Wavelength::Nm1490),
            5.0,
            epsilon = 1e-9
        );
    }

    #[test]
    fn coax_span_uses_rg6_attenuation() {
        let span = Component::CoaxSpan { length_m: 100.0 };
        assert_relative_eq!(span.loss_db(Wavelength::Nm1490), 5.5, epsilon = 1e-9);
    }

    #[test]
    fn negative_coax_length_clamps_to_zero_loss() {
        let span = Component::CoaxSpan { length_m: -50.0 };
        assert_relative_eq!(span.loss_db(Wavelength::Nm1490), 0.0, epsilon = 1e-9);
    }

    #[test]
    fn free_space_path_loss_doubling_costs_six_db() {
        // Inverse-square law: doubling distance costs ~6 dB, any frequency.
        let near = free_space_path_loss_db(100.0, 2400.0);
        let far = free_space_path_loss_db(200.0, 2400.0);
        assert_relative_eq!(far - near, 6.0206, epsilon = 1e-3);
    }

    #[test]
    fn free_space_path_loss_known_value() {
        // 100 m at 2.4 GHz: 40 + 67.60 − 27.55 ≈ 80.05 dB.
        assert_relative_eq!(
            free_space_path_loss_db(100.0, 2400.0),
            80.05,
            epsilon = 0.01
        );
    }

    #[test]
    fn zero_distance_hop_floors_instead_of_returning_neg_infinity() {
        let hop = Component::WirelessHop {
            distance_m: 0.0,
            frequency_mhz: 2400.0,
        };
        let loss = hop.loss_db(Wavelength::Nm1490);
        assert!(loss.is_finite(), "0 m hop must not produce −inf loss");
    }

    #[test]
    fn repeater_regenerates_at_its_own_tx_power() {
        let rpt = Component::Repeater { tx_dbm: 20.0 };
        // Whatever arrived — even deep below the noise — the retransmit
        // is a fresh 20 dBm shout.
        assert_relative_eq!(
            rpt.apply_level(-95.0, Wavelength::Nm1490),
            20.0,
            epsilon = 1e-9
        );
    }

    #[test]
    fn cable_category_bandwidths() {
        assert_eq!(CableCategory::Cat5e.bandwidth_mbps(), 1_000);
        assert_eq!(CableCategory::Cat6.bandwidth_mbps(), 10_000);
    }

    #[test]
    fn tap_reports_its_tap_port_loss() {
        let tap = Component::Tap { tap_loss_db: 8.0 };
        assert_relative_eq!(tap.loss_db(Wavelength::Nm1490), 8.0, epsilon = 1e-9);
    }

    // ---- validation: reference values (docs/GAME_DESIGN.md + physics) ----

    /// Real PLC splitters lose a little more than the ideal 10·log₁₀(N)
    /// power division; 3 dB of excess covers every catalog part we model.
    const SPLITTER_EXCESS_LOSS_DB_MAX: f64 = 3.0;
    /// A passive component may lose power but never add it.
    const PASSIVE_LOSS_DB_MIN: f64 = 0.0;
    const WIFI_MHZ: f64 = 2400.0;

    #[test]
    fn fusion_splice_beats_mechanical_and_matches_design_ranges() {
        let fusion = SpliceType::Fusion;
        let mechanical = SpliceType::Mechanical;
        assert!((0.05..=0.1).contains(&fusion.typical_loss_db()));
        assert!((0.3..=0.5).contains(&mechanical.typical_loss_db()));
        // The trade-off the puzzle is built on: fusion is better but slower.
        assert!(fusion.typical_loss_db() < mechanical.typical_loss_db());
        assert!(fusion.place_time_seconds() > mechanical.place_time_seconds());
    }

    #[test]
    fn apc_reflects_less_than_upc_with_comparable_insertion_loss() {
        for kind in [ConnectorType::Upc, ConnectorType::Apc] {
            assert!((0.3..=0.5).contains(&kind.typical_loss_db()), "{kind:?}");
        }
        // More negative return loss = less light reflected back.
        assert!(
            ConnectorType::Apc.typical_return_loss_db()
                < ConnectorType::Upc.typical_return_loss_db()
        );
    }

    #[test]
    fn contamination_stacks_on_top_of_nominal_connector_loss() {
        let dirty = Component::Connector {
            kind: ConnectorType::Upc,
            contamination_db: 3.0,
        };
        assert_relative_eq!(dirty.loss_db(Wavelength::Nm1490), 3.35, epsilon = 1e-9);
    }

    #[test]
    fn splitter_loss_is_ideal_division_plus_bounded_excess() {
        let ratios = [
            SplitterRatio::OneByTwo,
            SplitterRatio::OneByFour,
            SplitterRatio::OneByEight,
            SplitterRatio::OneBySixteen,
            SplitterRatio::OneByThirtyTwo,
        ];
        let mut previous_loss_db = 0.0;
        for ratio in ratios {
            let ideal_db = 10.0 * f64::from(ratio.branch_count()).log10();
            let excess_db = ratio.insertion_loss_db() - ideal_db;
            assert!(
                (0.0..=SPLITTER_EXCESS_LOSS_DB_MAX).contains(&excess_db),
                "{ratio:?}: excess {excess_db} dB"
            );
            assert!(ratio.insertion_loss_db() > previous_loss_db);
            previous_loss_db = ratio.insertion_loss_db();
        }
    }

    #[test]
    fn span_loss_depends_on_wavelength() {
        let span = Component::Span {
            length_km: 10.0,
            plant: PlantType::Buried,
        };
        assert_relative_eq!(span.loss_db(Wavelength::Nm1310), 3.5, epsilon = 1e-9);
        assert_relative_eq!(span.loss_db(Wavelength::Nm1550), 2.1, epsilon = 1e-9);
    }

    #[test]
    fn rg6_attenuation_sits_in_the_real_5_to_6_db_per_100m_band() {
        let per_100m_db = COAX_LOSS_DB_PER_M * 100.0;
        assert!((5.0..=6.0).contains(&per_100m_db));
    }

    #[test]
    fn free_space_path_loss_matches_the_textbook_constant() {
        // FSPL(dB) = 20·log₁₀(d_km) + 20·log₁₀(f_MHz) + 32.45, so 1 km at
        // 1 MHz is exactly the 32.45 dB constant.
        assert_relative_eq!(free_space_path_loss_db(1_000.0, 1.0), 32.45, epsilon = 1e-9);
        // 1 m at 2.4 GHz ≈ 40 dB — the Wi-Fi rule of thumb.
        assert_relative_eq!(
            free_space_path_loss_db(1.0, WIFI_MHZ),
            40.05,
            epsilon = 0.01
        );
    }

    #[test]
    fn doubling_frequency_costs_six_db() {
        let low = free_space_path_loss_db(100.0, WIFI_MHZ);
        let high = free_space_path_loss_db(100.0, 2.0 * WIFI_MHZ);
        assert_relative_eq!(high - low, 6.0206, epsilon = 1e-3);
    }

    // ---- adversarial: authoring errors and signal-floor edges ----

    #[test]
    fn zero_length_span_is_lossless() {
        let span = Component::Span {
            length_km: 0.0,
            plant: PlantType::Aerial,
        };
        assert_eq!(span.loss_db(Wavelength::Nm1310), 0.0);
    }

    #[test]
    fn negative_fiber_span_never_adds_power() {
        // A negative length in a level file is an authoring error, not an
        // optical amplifier. CoaxSpan and EthernetRun already clamp.
        let span = Component::Span {
            length_km: -10.0,
            plant: PlantType::Buried,
        };
        assert!(span.loss_db(Wavelength::Nm1310) >= PASSIVE_LOSS_DB_MIN);
        assert!(span.apply_level(-20.0, Wavelength::Nm1310) <= -20.0);
    }

    #[test]
    fn zero_frequency_hop_does_not_produce_infinite_gain() {
        let hop = Component::WirelessHop {
            distance_m: 100.0,
            frequency_mhz: 0.0,
        };
        let loss = hop.loss_db(Wavelength::Nm1490);
        assert!(loss.is_finite(), "0 MHz hop produced {loss} dB loss");
        assert!(loss >= PASSIVE_LOSS_DB_MIN);
    }

    #[test]
    fn near_field_hop_never_reports_gain() {
        // 0.1 m at 10 MHz is far inside the near field where the FSPL
        // formula goes negative (−27.55 dB). Free space is passive.
        let hop = Component::WirelessHop {
            distance_m: 0.1,
            frequency_mhz: 10.0,
        };
        assert!(hop.loss_db(Wavelength::Nm1490) >= PASSIVE_LOSS_DB_MIN);
    }

    #[test]
    fn negative_distance_hop_floors_like_a_zero_length_hop() {
        let at = |distance_m: f64| {
            Component::WirelessHop {
                distance_m,
                frequency_mhz: WIFI_MHZ,
            }
            .loss_db(Wavelength::Nm1490)
        };
        assert_eq!(at(-50.0), at(0.0));
        assert!(at(-50.0).is_finite());
    }

    #[test]
    fn tap_at_signal_floor_still_subtracts_linearly() {
        let tap = Component::Tap { tap_loss_db: 8.0 };
        let out = tap.apply_level(-95.0, Wavelength::Nm1490);
        assert_relative_eq!(out, -103.0, epsilon = 1e-9);
    }

    #[test]
    fn amplifier_cannot_rescue_a_dead_neg_infinity_level() {
        let amp = Component::Amplifier { gain_db: 30.0 };
        assert_eq!(
            amp.apply_level(f64::NEG_INFINITY, Wavelength::Nm1490),
            f64::NEG_INFINITY
        );
    }

    #[test]
    fn infinite_macrobend_fails_closed() {
        let kink = Component::Macrobend {
            excess_loss_db: f64::INFINITY,
        };
        assert_eq!(kink.apply_level(3.0, Wavelength::Nm1550), f64::NEG_INFINITY);
    }

    #[test]
    fn repeater_regenerates_even_from_a_nan_level() {
        // Decode-and-forward discards upstream history, garbage included.
        let rpt = Component::Repeater { tx_dbm: 20.0 };
        assert_eq!(rpt.apply_level(f64::NAN, Wavelength::Nm1490), 20.0);
    }

    #[test]
    fn ethernet_components_carry_no_db_loss() {
        let run = Component::EthernetRun {
            length_m: 130.0,
            category: CableCategory::Cat5e,
        };
        let switch = Component::Switch { poe_budget_w: 30.0 };
        assert_eq!(run.apply_level(-10.0, Wavelength::Nm1490), -10.0);
        assert_eq!(switch.apply_level(-10.0, Wavelength::Nm1490), -10.0);
    }

    #[test]
    fn unknown_component_variant_is_rejected_by_serde() {
        let parsed: Result<Component, _> = serde_json::from_str(r#"{"Laser":{"gain_db":99.0}}"#);
        assert!(parsed.is_err());
    }
}
