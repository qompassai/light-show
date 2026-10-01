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

impl Component {
    /// Loss this component contributes at the given wavelength, in dB.
    /// Gain components report negative loss so per-component ledger rows
    /// stay arithmetically honest. Only meaningful for the dB-budget
    /// media (fiber, coax, wireless): Ethernet components report 0.0 here
    /// and are evaluated by `evaluate_ethernet` instead, and `Repeater`
    /// reports 0.0 because its effect (regeneration) lives in
    /// [`Component::apply_level`].
    pub fn loss_db(&self, wavelength: Wavelength) -> f64 {
        match self {
            Component::Span { length_km, .. } => length_km * wavelength.attenuation_db_per_km(),
            Component::Splice {
                kind,
                degradation_db,
            } => kind.typical_loss_db() + degradation_db,
            Component::Connector {
                kind,
                contamination_db,
            } => kind.typical_loss_db() + contamination_db,
            Component::Splitter { ratio } => ratio.insertion_loss_db(),
            Component::Macrobend { excess_loss_db } => *excess_loss_db,
            Component::Amplifier { gain_db } => -gain_db,
            Component::Tap { tap_loss_db } => *tap_loss_db,
            Component::CoaxSpan { length_m } => length_m.max(0.0) * COAX_LOSS_DB_PER_M,
            Component::WirelessHop {
                distance_m,
                frequency_mhz,
            } => {
                // Floor the distance at 0.1 m: log10(0) is −inf, and a
                // zero-length hop is an authoring error, not −infinite
                // gain.
                free_space_path_loss_db(distance_m.max(0.1), *frequency_mhz)
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
}
