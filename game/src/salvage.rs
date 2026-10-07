//! Salvage: components that go TOO HOT on a job are fried, hauled back,
//! and traded to the Warehouse for cores — 1 core per fried part, banked
//! at level results on a win or a loss (spec:
//! `docs` and `~/workspace/light-show-ux/cores-and-reactions-design.md`).
//!
//! osp_sim renders TOO HOT as a whole-route verdict (the received level
//! lands above the receive window); it does not attribute the overdrive
//! to individual components. The attribution here is game-side, built on
//! the same chain walk the HUD's `frontier_budget` already does:
//!
//! - **Crossing rule.** Walking the connected chain from the source, a
//!   placed component fries at the first moment the level leaving it is
//!   above the window's max while the level entering it was not — the
//!   component itself drove the signal over the top (an amplifier staged
//!   too hot, a repeater re-transmitting too loud).
//! - **Delivery rule.** If the completed route delivers TOO HOT but no
//!   component crossed (the launch was already over the top and passive
//!   parts merely carried it — the usual fiber case), the last placed
//!   component on the route takes the hit: it fed the overdrive straight
//!   into the receiver and cooked.
//!
//! Only player-placed components fry. Fixed plant is the level's own
//! scenery — the player hauls back what they carried in, not the site's
//! hardware. Counting is per attempt and per placed part: once a part is
//! counted it stays counted for that attempt even if the player removes
//! it afterwards (dead is dead), and the same part can never count twice
//! in one attempt. No caps: replay farming at 1 core per fried part is
//! accepted by design (measured, not speculatively capped).

use bevy::prelude::Resource;
use osp_sim::{Component, ConnectorType, PathGraph, ReceiveWindow, SpliceType, Wavelength};
use std::collections::HashSet;

/// Display name of a component for the results salvage line
/// ("Recovered cores: Amplifier ×1, Fusion Splice ×1"). Names are the
/// part names a tech would write on a salvage tag, not the pill codes.
pub fn part_name(component: &Component) -> &'static str {
    match component {
        Component::Span { .. } => "Fiber Span",
        Component::Splice {
            kind: SpliceType::Fusion,
            ..
        } => "Fusion Splice",
        Component::Splice {
            kind: SpliceType::Mechanical,
            ..
        } => "Mechanical Splice",
        Component::Connector {
            kind: ConnectorType::Upc,
            ..
        } => "UPC Connector",
        Component::Connector {
            kind: ConnectorType::Apc,
            ..
        } => "APC Connector",
        Component::Splitter { .. } => "Splitter",
        Component::Macrobend { .. } => "Macrobend",
        Component::Amplifier { .. } => "Amplifier",
        Component::Tap { .. } => "Tap",
        Component::CoaxSpan { .. } => "Coax Span",
        Component::WirelessHop { .. } => "Wireless Hop",
        Component::Repeater { .. } => "Repeater",
        Component::EthernetRun { .. } => "Ethernet Run",
        Component::Switch { .. } => "Switch",
    }
}

/// One fried component on the current route, identified by the edge it
/// was placed on plus its part name. The `(from, to, part)` triple is
/// the dedupe key: re-placing the identical part on the identical edge
/// within one attempt is the same salvage entry, never a second core.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FriedPart {
    pub from: u32,
    pub to: u32,
    pub part: &'static str,
}

/// Detect which placed components fry on the route as currently built.
///
/// Mirrors `PathGraph::frontier_budget`'s walk exactly (first edge out
/// of each node, loop cut by a visited set, bounded by the edge count)
/// so the levels reasoned about here are the levels the HUD shows.
/// `placed_edges` is the set of `(from, to)` pairs the player placed;
/// fixed edges are walked for level purposes but can never fry.
/// Non-finite levels fail closed: every comparison against NaN is
/// false, so a corrupt level can never manufacture salvage.
pub fn detect_fried(
    graph: &PathGraph,
    placed_edges: &HashSet<(u32, u32)>,
    source: u32,
    target: u32,
    tx_dbm: f64,
    wavelength: Wavelength,
    window: ReceiveWindow,
) -> Vec<FriedPart> {
    let mut fried: Vec<FriedPart> = Vec::new();
    let mut level_dbm = tx_dbm;
    let mut current = source;
    let mut hops = 0usize;
    let mut visited: HashSet<u32> = HashSet::new();
    visited.insert(source);
    let mut last_placed: Option<FriedPart> = None;
    let mut any_crossing = false;
    while hops < graph.edges.len() {
        let Some(edge) = graph.edges.iter().find(|e| e.from == current) else {
            break;
        };
        let out_dbm = edge.component.apply_level(level_dbm, wavelength);
        if level_dbm <= window.max_dbm && out_dbm > window.max_dbm {
            any_crossing = true;
        }
        let is_placed = placed_edges.contains(&(edge.from, edge.to));
        if is_placed {
            last_placed = Some(FriedPart {
                from: edge.from,
                to: edge.to,
                part: part_name(&edge.component),
            });
            // Crossing rule: this component pushed the level over the
            // top of the window.
            if level_dbm <= window.max_dbm && out_dbm > window.max_dbm {
                let part = FriedPart {
                    from: edge.from,
                    to: edge.to,
                    part: part_name(&edge.component),
                };
                if !fried.contains(&part) {
                    fried.push(part);
                }
            }
        }
        level_dbm = out_dbm;
        current = edge.to;
        hops += 1;
        if !visited.insert(current) {
            break;
        }
    }
    // Delivery rule: the route completed TOO HOT with no crossing —
    // the last placed part fed the overdrive into the receiver.
    if !any_crossing && fried.is_empty() && hops > 0 && current == target && level_dbm > window.max_dbm {
        if let Some(part) = last_placed {
            fried.push(part);
        }
    }
    fried
}

/// Per-attempt salvage ledger. Reset when a level starts; banked
/// exactly once at results (win or lose) so re-entering the results
/// screen can never double-pay.
#[derive(Resource, Debug, Default)]
pub struct SalvageTracker {
    counted: Vec<FriedPart>,
    banked: bool,
}

impl SalvageTracker {
    /// Record freshly detected fried parts. Returns the parts that are
    /// new to this attempt (already-counted parts are dropped).
    pub fn record(&mut self, detected: &[FriedPart]) -> Vec<FriedPart> {
        let mut fresh = Vec::new();
        for part in detected {
            if !self.counted.contains(part) {
                self.counted.push(part.clone());
                fresh.push(part.clone());
            }
        }
        fresh
    }

    /// Parts counted so far this attempt, in the order they fried.
    pub fn counted(&self) -> &[FriedPart] {
        &self.counted
    }

    /// Pay out the attempt's salvage: the first call returns every
    /// counted part and marks the attempt banked; later calls return
    /// nothing, so results can never double-bank an attempt.
    pub fn bank(&mut self) -> Vec<FriedPart> {
        if self.banked {
            return Vec::new();
        }
        self.banked = true;
        self.counted.clone()
    }

    /// Start a fresh attempt: nothing counted, nothing banked.
    pub fn reset(&mut self) {
        self.counted.clear();
        self.banked = false;
    }
}

/// Salvage from the most recent results screen that the Warehouse has
/// not reacted to yet. Set by `states::results` when it banks salvage;
/// claimed by the Warehouse on entry so the hosts react to the haul
/// exactly once.
#[derive(Resource, Debug, Default, Clone)]
pub struct PendingHaul {
    /// Number of fried parts in the haul (== cores it paid).
    pub count: u32,
    /// Grouped part list, e.g. "Amplifier ×2, Fusion Splice ×1".
    pub summary: String,
}

impl PendingHaul {
    /// Record a freshly banked haul, replacing any unclaimed one (the
    /// player hauled again before visiting — the Warehouse reacts to
    /// the latest load).
    pub fn set(&mut self, parts: &[FriedPart]) {
        self.count = parts.len() as u32;
        self.summary = salvage_summary(parts);
    }

    /// Claim the haul: returns it and clears the slot, so the next
    /// Warehouse visit starts clean. `None` when there is no haul.
    pub fn take(&mut self) -> Option<PendingHaul> {
        if self.count == 0 {
            return None;
        }
        Some(std::mem::take(self))
    }
}

/// Group a fried-part list into the results/haul summary shape:
/// first-fried order, duplicates folded into a `×n` count
/// ("Amplifier ×2, Fusion Splice ×1"). Empty input → empty string.
pub fn salvage_summary(parts: &[FriedPart]) -> String {
    let mut groups: Vec<(&'static str, usize)> = Vec::new();
    for part in parts {
        match groups.iter_mut().find(|(name, _)| *name == part.part) {
            Some((_, count)) => *count += 1,
            None => groups.push((part.part, 1)),
        }
    }
    groups
        .iter()
        .map(|(name, count)| {
            if *count > 1 {
                format!("{name} ×{count}")
            } else {
                name.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use osp_sim::component::PlantType;
    use osp_sim::ReceiveWindow;

    const FIBER_WINDOW: ReceiveWindow = ReceiveWindow::GPON_ONT;

    fn graph_with(edges: &[(u32, u32, Component)]) -> PathGraph {
        let mut graph = PathGraph::default();
        for id in 0..=3u32 {
            graph.add_node(id, format!("N{id}"));
        }
        for (from, to, component) in edges {
            graph.connect(*from, *to, component.clone());
        }
        graph
    }

    fn placed(pairs: &[(u32, u32)]) -> HashSet<(u32, u32)> {
        pairs.iter().copied().collect()
    }

    fn span(km: f64) -> Component {
        Component::Span {
            length_km: km,
            plant: PlantType::Aerial,
        }
    }

    // ---- validation: the crossing rule ----

    #[test]
    fn amplifier_that_pushes_the_level_over_the_top_fries() {
        // Coax-style window 0..15. Launch 0, a span drops 5.5, then a
        // +30 dB amplifier drives the chain to +24.5: the amp crossed.
        let window = ReceiveWindow {
            min_dbm: 0.0,
            max_dbm: 15.0,
        };
        let graph = graph_with(&[
            (0, 1, Component::CoaxSpan { length_m: 100.0 }),
            (1, 2, Component::Amplifier { gain_db: 30.0 }),
            (2, 3, Component::Tap { tap_loss_db: 1.0 }),
        ]);
        let fried = detect_fried(
            &graph,
            &placed(&[(0, 1), (1, 2), (2, 3)]),
            0,
            3,
            0.0,
            Wavelength::Nm1490,
            window,
        );
        assert_eq!(
            fried,
            vec![FriedPart {
                from: 1,
                to: 2,
                part: "Amplifier",
            }]
        );
    }

    #[test]
    fn in_window_route_fries_nothing() {
        let window = ReceiveWindow {
            min_dbm: 0.0,
            max_dbm: 15.0,
        };
        let graph = graph_with(&[
            (0, 1, Component::CoaxSpan { length_m: 100.0 }),
            (1, 2, Component::Amplifier { gain_db: 15.0 }),
        ]);
        let fried = detect_fried(
            &graph,
            &placed(&[(0, 1), (1, 2)]),
            0,
            2,
            0.0,
            Wavelength::Nm1490,
            window,
        );
        assert!(fried.is_empty(), "in-window route fried {fried:?}");
    }

    // ---- validation: the delivery rule (fiber case) ----

    #[test]
    fn too_hot_delivery_cooks_the_last_placed_part() {
        // Fiber: launch +3 dBm is already above the GPON window max, so
        // nothing "crosses" — passives only carry the overdrive. A short
        // route still delivers ~+2.5 dBm: TOO HOT, and the last placed
        // part (the splice feeding the receiver) takes the hit.
        let graph = graph_with(&[
            (0, 1, span(1.0)),
            (
                1,
                2,
                Component::Splice {
                    kind: SpliceType::Fusion,
                    degradation_db: 0.0,
                },
            ),
        ]);
        let fried = detect_fried(
            &graph,
            &placed(&[(0, 1), (1, 2)]),
            0,
            2,
            3.0,
            Wavelength::Nm1490,
            FIBER_WINDOW,
        );
        assert_eq!(
            fried,
            vec![FriedPart {
                from: 1,
                to: 2,
                part: "Fusion Splice",
            }]
        );
    }

    #[test]
    fn cold_route_fries_nothing() {
        // 120 km of fiber pulls +3 dBm to about -30.6 dBm, below the
        // GPON window floor: TOO LOW is a coaching moment, not a
        // salvage moment.
        let graph = graph_with(&[(0, 1, span(120.0))]);
        let fried = detect_fried(
            &graph,
            &placed(&[(0, 1)]),
            0,
            1,
            3.0,
            Wavelength::Nm1490,
            FIBER_WINDOW,
        );
        assert!(fried.is_empty());
    }

    // ---- adversarial: scenery and bookkeeping ----

    #[test]
    fn fixed_plant_never_fries() {
        // Same crossing shape as the amplifier test, but the amplifier
        // is fixed plant (not in the placed set): the player hauls back
        // only what they carried in.
        let window = ReceiveWindow {
            min_dbm: 0.0,
            max_dbm: 15.0,
        };
        let graph = graph_with(&[
            (0, 1, Component::CoaxSpan { length_m: 100.0 }),
            (1, 2, Component::Amplifier { gain_db: 30.0 }),
        ]);
        let fried = detect_fried(
            &graph,
            &placed(&[(0, 1)]),
            0,
            2,
            0.0,
            Wavelength::Nm1490,
            window,
        );
        assert!(fried.is_empty(), "fixed plant fried: {fried:?}");
    }

    #[test]
    fn disconnected_placement_cannot_fry() {
        let window = ReceiveWindow {
            min_dbm: 0.0,
            max_dbm: 15.0,
        };
        // The amplifier sits on a branch the source chain never reaches.
        let graph = graph_with(&[
            (0, 1, Component::CoaxSpan { length_m: 10.0 }),
            (2, 3, Component::Amplifier { gain_db: 40.0 }),
        ]);
        let fried = detect_fried(
            &graph,
            &placed(&[(0, 1), (2, 3)]),
            0,
            3,
            0.0,
            Wavelength::Nm1490,
            window,
        );
        assert!(fried.is_empty());
    }

    #[test]
    fn same_part_never_counts_twice_in_one_attempt() {
        let mut tracker = SalvageTracker::default();
        let part = FriedPart {
            from: 1,
            to: 2,
            part: "Amplifier",
        };
        assert_eq!(tracker.record(&[part.clone()]), vec![part.clone()]);
        // The same detection reported again (next frame's walk) is not
        // a second core.
        assert!(tracker.record(&[part]).is_empty());
        assert_eq!(tracker.counted().len(), 1);
    }

    #[test]
    fn bank_pays_once_and_reset_rearms() {
        let mut tracker = SalvageTracker::default();
        tracker.record(&[FriedPart {
            from: 0,
            to: 1,
            part: "Fiber Span",
        }]);
        assert_eq!(tracker.bank().len(), 1);
        assert!(tracker.bank().is_empty(), "second bank must pay nothing");
        tracker.reset();
        assert!(tracker.counted().is_empty());
        // After reset the same part can fry (and pay) in a new attempt.
        assert_eq!(
            tracker
                .record(&[FriedPart {
                    from: 0,
                    to: 1,
                    part: "Fiber Span",
                }])
                .len(),
            1
        );
    }

    #[test]
    fn summary_groups_duplicates_in_first_fried_order() {
        let parts = vec![
            FriedPart {
                from: 0,
                to: 1,
                part: "Amplifier",
            },
            FriedPart {
                from: 1,
                to: 2,
                part: "Fusion Splice",
            },
            FriedPart {
                from: 2,
                to: 3,
                part: "Amplifier",
            },
        ];
        assert_eq!(salvage_summary(&parts), "Amplifier ×2, Fusion Splice");
        assert_eq!(salvage_summary(&[]), "");
    }

    #[test]
    fn pending_haul_claims_exactly_once() {
        let mut haul = PendingHaul::default();
        assert!(haul.take().is_none());
        haul.set(&[FriedPart {
            from: 0,
            to: 1,
            part: "Tap",
        }]);
        let claimed = haul.take().expect("haul present");
        assert_eq!(claimed.count, 1);
        assert_eq!(claimed.summary, "Tap");
        assert!(haul.take().is_none(), "haul must not react twice");
    }
}
