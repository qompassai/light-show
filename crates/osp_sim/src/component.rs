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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantType {
    Aerial,
    Buried,
    Conduit,
}

impl Component {
    /// Loss this component contributes at the given wavelength, in dB.
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
        }
    }
}
