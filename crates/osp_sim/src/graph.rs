//! Path graph: a level's OSP layout as nodes connected by `Component` edges.
//! Supports both simple point-to-point levels and branching PON trees
//! (one OLT feeding many ONTs through splitters).

use crate::component::Component;
#[cfg(test)]
use crate::component::CableCategory;
use crate::outage::Outage;
use crate::wavelength::Wavelength;
use crate::ReceiveWindow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub type NodeId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathNode {
    pub id: NodeId,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub component: Component,
}

/// A full OSP layout: nodes (OLT, splices, splitters, ONTs) plus the edges
/// (fiber/hardware) the player has routed between them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathGraph {
    pub nodes: Vec<PathNode>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Error, PartialEq)]
pub enum PathError {
    #[error("no continuous path exists from source to target")]
    Disconnected,
    #[error("path contains a cycle")]
    Cycle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkBudgetResult {
    pub total_loss_db: f64,
    pub received_dbm: f64,
    pub in_window: bool,
    pub margin_db: f64,
    pub hop_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EthernetEval {
    pub violations: Vec<EthernetViolation>,
    /// Longest unbroken copper stretch (source/switch → switch/target).
    pub longest_segment_m: f64,
    pub poe_draw_w: f64,
    pub poe_budget_w: f64,
    /// Bottleneck run category on the path, 0 when the path has no runs.
    pub min_bandwidth_mbps: u64,
}

impl EthernetEval {
    pub fn passes(&self) -> bool {
        self.violations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EthernetViolation {
    SegmentTooLong { longest_m: f64, limit_m: f64 },
    PoeOverBudget { draw_w: f64, budget_w: f64 },
    BandwidthTooLow { min_mbps: u64, required_mbps: u64 },
    UnsupportedComponent {
        edge_from: u32,
        edge_to: u32,
        detail: String,
    },
}

impl PathGraph {
    pub fn add_node(&mut self, id: NodeId, label: impl Into<String>) {
        self.nodes.push(PathNode {
            id,
            label: label.into(),
        });
    }

    pub fn connect(&mut self, from: NodeId, to: NodeId, component: Component) {
        self.edges.push(Edge {
            from,
            to,
            component,
        });
    }

    /// Depth-first search with backtracking from `source` to `target`.
    ///
    /// The original implementation was a greedy walk that committed to
    /// the first matching edge and could never reach a target behind a
    /// dead branch — protection-switched levels (e.g. Storm Season's
    /// 0→2→3 buried route, reachable only past the dead 0→1 aerial leg
    /// once the storm severs it) were unresolvable. DFS explores the
    /// whole reachable graph in edge-insertion order, so it stays
    /// deterministic while actually finding the protection path. The
    /// `visited` set is kept to avoid looping forever on cycles; reaching
    /// the target is always preferred over a cycle, so the `Cycle`
    /// variant is now effectively unreachable, but the public error type
    /// is preserved for API stability.
    fn resolve_path(&self, source: NodeId, target: NodeId) -> Result<Vec<&Edge>, PathError> {
        let mut adjacency: HashMap<NodeId, Vec<&Edge>> = HashMap::new();
        for edge in &self.edges {
            adjacency.entry(edge.from).or_default().push(edge);
        }

        let mut stack = Vec::new();
        let mut visited = std::collections::HashSet::new();
        if Self::dfs(&adjacency, source, target, &mut stack, &mut visited) {
            Ok(stack)
        } else {
            Err(PathError::Disconnected)
        }
    }

    fn dfs<'a>(
        adjacency: &HashMap<NodeId, Vec<&'a Edge>>,
        node: NodeId,
        target: NodeId,
        stack: &mut Vec<&'a Edge>,
        visited: &mut std::collections::HashSet<NodeId>,
    ) -> bool {
        if node == target {
            return true;
        }
        if !visited.insert(node) {
            return false;
        }
        if let Some(candidates) = adjacency.get(&node) {
            for edge in candidates {
                stack.push(edge);
                if Self::dfs(adjacency, edge.to, target, stack, visited) {
                    return true;
                }
                stack.pop();
            }
        }
        visited.remove(&node);
        false
    }

    /// Compute the received power at `target` given a launch power at
    /// `source`, following the routed path at the given wavelength, and
    /// evaluate it against a receive window.
    ///
    /// The running level is threaded edge by edge via
    /// [`Component::apply_level`], so gain and regenerative components are
    /// honored: an amplifier adds its gain, a repeater resets the level to
    /// its own transmit power. `total_loss_db` is the net dB between
    /// transmit and receive (`tx_dbm − received_dbm`), which equals the
    /// summed passive loss on fiber-only paths (backwards compatible).
    pub fn compute_link_budget(
        &self,
        source: NodeId,
        target: NodeId,
        tx_dbm: f64,
        wavelength: Wavelength,
        window: ReceiveWindow,
    ) -> Result<LinkBudgetResult, PathError> {
        let path = self.resolve_path(source, target)?;
        let mut received_dbm = tx_dbm;
        for edge in &path {
            received_dbm = edge.component.apply_level(received_dbm, wavelength);
        }
        let total_loss_db = tx_dbm - received_dbm;
        Ok(LinkBudgetResult {
            total_loss_db,
            received_dbm,
            in_window: window.contains(received_dbm),
            margin_db: window.margin(received_dbm),
            hop_count: path.len(),
        })
    }

    /// Like `compute_link_budget`, but also accounts for an in-progress
    /// `Outage`.
    ///
    /// Full-cut hazards (see `OutageKind::is_full_cut`) are expected to
    /// already be excluded from `self` by the caller — the affected edge
    /// should simply not be `connect`ed while the cut is unresolved (see
    /// `board::rebuild_live_graph` in the game crate) — so a still-severed
    /// full cut naturally resolves through the ordinary `Disconnected`
    /// path error above, not through this method. Degrade-type hazards
    /// (water intrusion, connector contamination, macrobend) instead keep
    /// their edge connected but worsen over time, so this method adds the
    /// hazard's currently-accumulated extra loss on top of the plain
    /// budget and re-evaluates the receive window against that adjusted
    /// figure.
    pub fn compute_link_budget_with_outage(
        &self,
        source: NodeId,
        target: NodeId,
        tx_dbm: f64,
        wavelength: Wavelength,
        window: ReceiveWindow,
        outage: Option<&Outage>,
    ) -> Result<LinkBudgetResult, PathError> {
        let mut result = self.compute_link_budget(source, target, tx_dbm, wavelength, window)?;
        if let Some(outage) = outage {
            if !outage.resolved && !outage.kind.is_full_cut() {
                let extra_db = outage.accumulated_extra_loss_db();
                result.total_loss_db += extra_db;
                result.received_dbm -= extra_db;
                result.in_window = window.contains(result.received_dbm);
                result.margin_db = window.margin(result.received_dbm);
            }
        }
        Ok(result)
    }

    /// Evaluate an Ethernet path against structured-cabling constraints.
    /// Unlike the dB media, Ethernet has no signal-level budget: the path
    /// is a sequence of copper runs and switches, and it passes when every
    /// unbroken copper segment respects `max_segment_m`, the pooled PoE
    /// budget covers `poe_draw_w`, and the slowest run's category meets
    /// `required_bandwidth_mbps`. Components that are not
    /// [`Component::EthernetRun`] or [`Component::Switch`] are reported as
    /// [`EthernetViolation::UnsupportedComponent`].
    pub fn evaluate_ethernet(
        &self,
        source: NodeId,
        target: NodeId,
        max_segment_m: f64,
        poe_draw_w: f64,
        required_bandwidth_mbps: u64,
    ) -> Result<EthernetEval, PathError> {
        let path = self.resolve_path(source, target)?;
        let mut violations = Vec::new();
        let mut segment_len = 0.0f64;
        let mut longest_segment_m = 0.0f64;
        let mut poe_budget_w = 0.0f64;
        let mut min_bandwidth_mbps = u64::MAX;
        for edge in &path {
            match &edge.component {
                Component::EthernetRun { length_m, category } => {
                    segment_len += length_m.max(0.0);
                    longest_segment_m = longest_segment_m.max(segment_len);
                    min_bandwidth_mbps = min_bandwidth_mbps.min(category.bandwidth_mbps());
                }
                Component::Switch { poe_budget_w: b } => {
                    segment_len = 0.0;
                    poe_budget_w += b;
                }
                other => {
                    violations.push(EthernetViolation::UnsupportedComponent {
                        edge_from: edge.from,
                        edge_to: edge.to,
                        detail: format!("{other:?} is not an Ethernet component"),
                    });
                }
            }
        }
        if longest_segment_m > max_segment_m {
            violations.push(EthernetViolation::SegmentTooLong {
                longest_m: longest_segment_m,
                limit_m: max_segment_m,
            });
        }
        if poe_draw_w > poe_budget_w {
            violations.push(EthernetViolation::PoeOverBudget {
                draw_w: poe_draw_w,
                budget_w: poe_budget_w,
            });
        }
        if min_bandwidth_mbps < required_bandwidth_mbps {
            violations.push(EthernetViolation::BandwidthTooLow {
                min_mbps: min_bandwidth_mbps,
                required_mbps: required_bandwidth_mbps,
            });
        }
        Ok(EthernetEval {
            violations,
            longest_segment_m,
            poe_draw_w,
            poe_budget_w,
            min_bandwidth_mbps: if min_bandwidth_mbps == u64::MAX {
                0
            } else {
                min_bandwidth_mbps
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{Component, PlantType};
    use crate::outage::OutageKind;
    use crate::DEFAULT_TX_DBM;
    use approx::assert_relative_eq;

    #[test]
    fn simple_span_budget() {
        let mut g = PathGraph::default();
        g.add_node(0, "OLT");
        g.add_node(1, "ONT");
        g.connect(
            0,
            1,
            Component::Span {
                length_km: 10.0,
                plant: PlantType::Buried,
            },
        );

        let result = g
            .compute_link_budget(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .unwrap();

        // 10km * 0.28 dB/km = 2.8 dB loss; 3.0 - 2.8 = 0.2 dBm received.
        assert_relative_eq!(result.total_loss_db, 2.8, epsilon = 1e-9);
        assert_relative_eq!(result.received_dbm, 0.2, epsilon = 1e-9);
        // 0.2 dBm is above the -8 dBm max — too hot, out of window.
        assert!(!result.in_window);
    }

    #[test]
    fn disconnected_path_errors() {
        let mut g = PathGraph::default();
        g.add_node(0, "OLT");
        g.add_node(1, "ONT");
        let err = g
            .compute_link_budget(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .unwrap_err();
        assert_eq!(err, PathError::Disconnected);
    }

    #[test]
    fn degrade_type_outage_adds_extra_loss_on_top_of_the_plain_budget() {
        let mut g = PathGraph::default();
        g.add_node(0, "OLT");
        g.add_node(1, "ONT");
        g.connect(
            0,
            1,
            Component::Span {
                length_km: 1.0,
                plant: PlantType::Buried,
            },
        );
        let mut outage = Outage::new(OutageKind::WaterIntrusion, 0, 1);
        outage.tick(50.0); // 5.0 dB accumulated (see accumulated_extra_loss_db)

        let plain = g
            .compute_link_budget(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .unwrap();
        let with_outage = g
            .compute_link_budget_with_outage(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
                Some(&outage),
            )
            .unwrap();

        assert_relative_eq!(
            with_outage.total_loss_db - plain.total_loss_db,
            5.0,
            epsilon = 1e-9
        );
        assert_relative_eq!(
            with_outage.received_dbm,
            plain.received_dbm - 5.0,
            epsilon = 1e-9
        );
    }

    #[test]
    fn resolved_outage_no_longer_contributes_extra_loss() {
        let mut g = PathGraph::default();
        g.add_node(0, "OLT");
        g.add_node(1, "ONT");
        g.connect(
            0,
            1,
            Component::Span {
                length_km: 1.0,
                plant: PlantType::Buried,
            },
        );
        let mut outage = Outage::new(OutageKind::WaterIntrusion, 0, 1);
        outage.tick(50.0);
        outage.resolved = true;

        let plain = g
            .compute_link_budget(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .unwrap();
        let with_outage = g
            .compute_link_budget_with_outage(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
                Some(&outage),
            )
            .unwrap();

        assert_relative_eq!(
            with_outage.total_loss_db,
            plain.total_loss_db,
            epsilon = 1e-9
        );
    }

    #[test]
    fn full_cut_outage_relies_on_the_caller_excluding_the_edge() {
        // A full-cut hazard on an edge that's simply absent from the graph
        // (as `board::rebuild_live_graph` ensures while unresolved) surfaces
        // as an ordinary `Disconnected` error, not a special outage code path.
        let mut g = PathGraph::default();
        g.add_node(0, "OLT");
        g.add_node(1, "ONT");
        let outage = Outage::new(OutageKind::FiberCut, 0, 1);
        let err = g
            .compute_link_budget_with_outage(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
                Some(&outage),
            )
            .unwrap_err();
        assert_eq!(err, PathError::Disconnected);
    }

    #[test]
    fn dfs_backtracks_past_a_dead_branch_to_the_protection_path() {
        // Storm Season's shape: 0 → 1 is a dead end once the storm severs
        // 1 → 3, and the target is reachable via 0 → 2 → 3. The old greedy
        // walk always committed to 0 → 1 and reported Disconnected — the
        // protection path was unroutable and the level unwinnable.
        let mut g = PathGraph::default();
        for (id, label) in [(0, "OLT"), (1, "Aerial"), (2, "Protection"), (3, "ONT")] {
            g.add_node(id, label);
        }
        g.connect(
            0,
            1,
            Component::Span {
                length_km: 5.0,
                plant: PlantType::Aerial,
            },
        );
        g.connect(
            0,
            2,
            Component::Span {
                length_km: 6.5,
                plant: PlantType::Buried,
            },
        );
        // 1 → 3 severed by the storm: absent from the graph, as
        // `board::rebuild_live_graph` ensures while unresolved.
        g.connect(
            2,
            3,
            Component::Splice {
                kind: crate::component::SpliceType::Mechanical,
                degradation_db: 0.0,
            },
        );
        let result = g
            .compute_link_budget(
                0,
                3,
                -8.0,
                Wavelength::Nm1490,
                ReceiveWindow {
                    min_dbm: -27.0,
                    max_dbm: -8.0,
                },
            )
            .unwrap();
        // 6.5 km * 0.28 + 0.4 mechanical = 2.22 dB; −8 − 2.22 = −10.22.
        assert_relative_eq!(result.received_dbm, -10.22, epsilon = 1e-9);
        assert!(result.in_window);
    }

    #[test]
    fn walk_honors_amplifier_gain() {
        let mut g = PathGraph::default();
        g.add_node(0, "Headend");
        g.add_node(1, "Amp");
        g.add_node(2, "Tap");
        g.connect(0, 1, Component::CoaxSpan { length_m: 100.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 20.0 });
        let result = g
            .compute_link_budget(
                0,
                2,
                30.0,
                Wavelength::Nm1490,
                ReceiveWindow {
                    min_dbm: 0.0,
                    max_dbm: 50.0,
                },
            )
            .unwrap();
        // 30 − 5.5 + 20 = 44.5; net dB = −14.5 (net gain).
        assert_relative_eq!(result.received_dbm, 44.5, epsilon = 1e-9);
        assert_relative_eq!(result.total_loss_db, -14.5, epsilon = 1e-9);
    }

    #[test]
    fn walk_honors_repeater_regeneration_across_two_hops() {
        let mut g = PathGraph::default();
        for (id, label) in [(0, "Site A"), (1, "Repeater"), (2, "Site B")] {
            g.add_node(id, label);
        }
        let hop = |d: f64| Component::WirelessHop {
            distance_m: d,
            frequency_mhz: 2400.0,
        };
        g.connect(0, 1, hop(400.0));
        g.connect(1, 2, Component::Repeater { tx_dbm: 20.0 });
        g.add_node(3, "Site C");
        g.connect(2, 3, hop(400.0));
        let result = g
            .compute_link_budget(
                0,
                3,
                20.0,
                Wavelength::Nm1490,
                ReceiveWindow {
                    min_dbm: -75.0,
                    max_dbm: -40.0,
                },
            )
            .unwrap();
        // 400 m @ 2.4 GHz ≈ 92.1 dB: arrives at −72.1, the repeater
        // regenerates to 20, the second hop lands at −72.1 again.
        assert_relative_eq!(result.received_dbm, -72.1, epsilon = 0.01);
        assert!(result.in_window);
    }

    #[test]
    fn amplifier_between_two_hops_cannot_save_a_dead_signal() {
        // Same geometry, but an amplifier instead of a repeater: it boosts
        // the first hop's wreckage (+30 dB) and the second hop buries it
        // again — 20 − 92.1 + 30 − 92.1 ≈ −134.2 dBm.
        let mut g = PathGraph::default();
        for (id, label) in [(0, "Site A"), (1, "Amp"), (2, "Site B")] {
            g.add_node(id, label);
        }
        let hop = |d: f64| Component::WirelessHop {
            distance_m: d,
            frequency_mhz: 2400.0,
        };
        g.connect(0, 1, hop(400.0));
        g.connect(1, 2, Component::Amplifier { gain_db: 30.0 });
        g.add_node(3, "Site C");
        g.connect(2, 3, hop(400.0));
        let result = g
            .compute_link_budget(
                0,
                3,
                20.0,
                Wavelength::Nm1490,
                ReceiveWindow {
                    min_dbm: -75.0,
                    max_dbm: -40.0,
                },
            )
            .unwrap();
        assert_relative_eq!(result.received_dbm, -134.19, epsilon = 0.01);
        assert!(!result.in_window);
    }

    #[test]
    fn ethernet_eval_passes_a_sound_design() {
        let mut g = PathGraph::default();
        for (id, label) in [(0, "Closet"), (1, "IDF"), (2, "Desk")] {
            g.add_node(id, label);
        }
        g.connect(
            0,
            1,
            Component::EthernetRun {
                length_m: 65.0,
                category: CableCategory::Cat5e,
            },
        );
        g.connect(1, 2, Component::Switch { poe_budget_w: 30.0 });
        let eval = g
            .evaluate_ethernet(0, 2, 100.0, 25.0, 1_000)
            .unwrap();
        assert!(eval.passes());
        assert_relative_eq!(eval.longest_segment_m, 65.0, epsilon = 1e-9);
        assert_relative_eq!(eval.poe_budget_w, 30.0, epsilon = 1e-9);
        assert_eq!(eval.min_bandwidth_mbps, 1_000);
    }

    #[test]
    fn ethernet_eval_flags_a_130m_single_segment() {
        let mut g = PathGraph::default();
        g.add_node(0, "Closet");
        g.add_node(1, "Desk");
        g.connect(
            0,
            1,
            Component::EthernetRun {
                length_m: 130.0,
                category: CableCategory::Cat6,
            },
        );
        let eval = g.evaluate_ethernet(0, 1, 100.0, 0.0, 1_000).unwrap();
        assert!(!eval.passes());
        assert!(matches!(
            eval.violations.as_slice(),
            [EthernetViolation::SegmentTooLong { .. }]
        ));
    }

    #[test]
    fn ethernet_eval_boundary_100m_is_allowed() {
        let mut g = PathGraph::default();
        g.add_node(0, "Closet");
        g.add_node(1, "Desk");
        g.connect(
            0,
            1,
            Component::EthernetRun {
                length_m: 100.0,
                category: CableCategory::Cat5e,
            },
        );
        let eval = g.evaluate_ethernet(0, 1, 100.0, 0.0, 1_000).unwrap();
        assert!(eval.passes(), "exactly 100 m must not violate");
    }

    #[test]
    fn ethernet_eval_flags_poe_over_budget() {
        let mut g = PathGraph::default();
        g.add_node(0, "Closet");
        g.add_node(1, "Switch");
        g.add_node(2, "AP");
        g.connect(
            0,
            1,
            Component::EthernetRun {
                length_m: 10.0,
                category: CableCategory::Cat6,
            },
        );
        g.connect(1, 2, Component::Switch { poe_budget_w: 15.0 });
        let eval = g.evaluate_ethernet(0, 2, 100.0, 25.0, 1_000).unwrap();
        assert!(!eval.passes());
        assert!(eval
            .violations
            .iter()
            .any(|v| matches!(v, EthernetViolation::PoeOverBudget { .. })));
    }

    #[test]
    fn ethernet_eval_flags_slow_cable() {
        let mut g = PathGraph::default();
        g.add_node(0, "Closet");
        g.add_node(1, "Desk");
        g.connect(
            0,
            1,
            Component::EthernetRun {
                length_m: 10.0,
                category: CableCategory::Cat5e,
            },
        );
        let eval = g.evaluate_ethernet(0, 1, 100.0, 0.0, 2_000).unwrap();
        assert!(!eval.passes());
        assert!(eval
            .violations
            .iter()
            .any(|v| matches!(v, EthernetViolation::BandwidthTooLow { .. })));
    }

    #[test]
    fn ethernet_eval_flags_non_ethernet_components() {
        // Adversarial: a fiber span smuggled into an Ethernet level.
        let mut g = PathGraph::default();
        g.add_node(0, "Closet");
        g.add_node(1, "Desk");
        g.connect(
            0,
            1,
            Component::Span {
                length_km: 1.0,
                plant: PlantType::Buried,
            },
        );
        let eval = g.evaluate_ethernet(0, 1, 100.0, 0.0, 1_000).unwrap();
        assert!(eval
            .violations
            .iter()
            .any(|v| matches!(v, EthernetViolation::UnsupportedComponent { .. })));
    }

    #[test]
    fn ethernet_eval_disconnected_still_errors() {
        let mut g = PathGraph::default();
        g.add_node(0, "Closet");
        g.add_node(1, "Desk");
        let err = g
            .evaluate_ethernet(0, 1, 100.0, 0.0, 1_000)
            .unwrap_err();
        assert_eq!(err, PathError::Disconnected);
    }
}
