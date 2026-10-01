//! Transmission media. Light Show teaches four disciplines — one per
//! companion — and each medium gets genuinely different simulation, not a
//! reskin:
//!
//! - **Fiber** (Séraphine): optical power in dBm; the path is loss-only
//!   (splices, connectors, splitters, spans) against a receive window.
//! - **Coax** (Ondine): RF level in dBmV; cascades include *gain*
//!   (amplifiers), so the puzzle is landing inside a window that punishes
//!   both under-drive (snow) and over-drive (distortion).
//! - **Wireless** (Linka): RSSI in dBm; hops lose power to free-space path
//!   loss (geometry, not cable), and regenerative repeaters reset the
//!   level by retransmitting at their own power.
//! - **Ethernet** (Lattice): no dB budget at all — structured-cabling
//!   constraints (segment length, PoE power budget, bandwidth), evaluated
//!   by [`PathGraph::evaluate_ethernet`](crate::graph::PathGraph::evaluate_ethernet).
//!
//! The dB arithmetic is shared (a decibel is a decibel); what differs is
//! the component set, the units on the receive window, and — for
//! Ethernet — the win condition itself.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Medium {
    Fiber,
    Coax,
    Wireless,
    Ethernet,
}

impl Medium {
    /// Unit label for the receive window on this medium's levels, shown
    /// in the ledger and results screens. Ethernet has no signal level —
    /// its levels report constraint checks instead.
    pub fn units_label(&self) -> &'static str {
        match self {
            Medium::Fiber => "dBm",
            Medium::Coax => "dBmV",
            Medium::Wireless => "dBm",
            Medium::Ethernet => "",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Medium::Fiber => "Fiber",
            Medium::Coax => "Coax",
            Medium::Wireless => "Wireless",
            Medium::Ethernet => "Ethernet",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_medium_has_a_display_name() {
        for medium in [
            Medium::Fiber,
            Medium::Coax,
            Medium::Wireless,
            Medium::Ethernet,
        ] {
            assert!(!medium.display_name().is_empty());
        }
    }

    #[test]
    fn only_ethernet_lacks_signal_units() {
        assert_eq!(Medium::Fiber.units_label(), "dBm");
        assert_eq!(Medium::Coax.units_label(), "dBmV");
        assert_eq!(Medium::Wireless.units_label(), "dBm");
        assert_eq!(Medium::Ethernet.units_label(), "");
    }
}
