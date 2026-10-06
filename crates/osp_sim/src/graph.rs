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

/// Maximum path length the path search will explore, in hops. A hostile
/// level file could otherwise hand the search a chain long enough to
/// exhaust memory building the path vector; past this bound the search
/// refuses with [`PathError::PathTooDeep`] instead. Set an order of
/// magnitude above the hostile-chain test's 200 000 hops, and far above
/// any sane level (bundled levels have < 10 hops).
pub const MAX_PATH_HOPS: usize = 1_000_000;

#[derive(Debug, Error, PartialEq)]
pub enum PathError {
    #[error("no continuous path exists from source to target")]
    Disconnected,
    #[error("path contains a cycle")]
    Cycle,
    #[error("path exceeds the {MAX_PATH_HOPS}-hop search bound")]
    PathTooDeep,
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
    /// A NaN input reached the evaluator: comparisons against NaN are
    /// always false, so a NaN segment limit, PoE draw, run length, or
    /// switch budget would silently pass every check. `field` names the
    /// offending input (`"max_segment_m"`, `"poe_draw_w"`, `"length_m"`,
    /// or `"poe_budget_w"`). Infinity is deliberately not reported here —
    /// it flows through the ordinary comparisons, which already fail
    /// closed on it.
    NonFiniteInput {
        field: &'static str,
    },
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

    /// Depth-first search with backtracking from `source` to `target`,
    /// over an explicit work stack — never recursion. The recursive
    /// formulation overflowed the thread stack (process abort, not a
    /// catchable panic) on a deserialized 200 000-hop chain; the explicit
    /// stack cannot, and it is bounded by [`MAX_PATH_HOPS`], refusing with
    /// [`PathError::PathTooDeep`] past that bound.
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
    /// is preserved for API stability. A node stays visited after it
    /// fails: a dead end stays dead, and forgetting it on backtrack makes
    /// the search walk every simple path (exponential on dense layouts)
    /// instead of every node once.
    fn resolve_path(&self, source: NodeId, target: NodeId) -> Result<Vec<&Edge>, PathError> {
        let mut adjacency: HashMap<NodeId, Vec<&Edge>> = HashMap::new();
        for edge in &self.edges {
            adjacency.entry(edge.from).or_default().push(edge);
        }

        // The recursive formulation checked `node == target` before the
        // visited set, so a zero-hop query (source == target) succeeds
        // even though the source is trivially "visited".
        if source == target {
            return Ok(Vec::new());
        }

        // Each work-stack frame is (node, index of the next candidate edge
        // to try). This mirrors the recursive DFS exactly: candidates are
        // tried in edge-insertion order, a node is marked visited on first
        // entry and never unmarked (a failed node stays failed — unmarking
        // on backtrack walks every simple path, exponential on dense
        // layouts), and `path` holds the edges on the current
        // root-to-frame chain, popped on backtrack.
        let mut path: Vec<&Edge> = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<(NodeId, usize)> = Vec::new();
        visited.insert(source);
        stack.push((source, 0));

        while let Some((node, next)) = stack.pop() {
            let candidates: &[&Edge] = match adjacency.get(&node) {
                Some(edges) => edges,
                None => &[],
            };
            if next >= candidates.len() {
                // Every edge from this node is exhausted (or led to an
                // already-visited node): backtrack, dropping the edge that
                // led here. The node stays marked visited — a dead end
                // stays dead. (`Vec::pop` on an empty path is a no-op, so
                // the source frame's exhaustion is safe.)
                path.pop();
                continue;
            }
            stack.push((node, next + 1));
            let edge = candidates[next];
            if edge.to == target {
                path.push(edge);
                return Ok(path);
            }
            if visited.insert(edge.to) {
                if path.len() >= MAX_PATH_HOPS {
                    return Err(PathError::PathTooDeep);
                }
                path.push(edge);
                stack.push((edge.to, 0));
            }
            // Already-visited neighbor: a cycle or a known dead end. Skip
            // it and keep scanning this node's remaining candidates.
        }
        Err(PathError::Disconnected)
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
    /// [`EthernetViolation::UnsupportedComponent`]. NaN inputs fail closed:
    /// each NaN float input is reported as an
    /// [`EthernetViolation::NonFiniteInput`] instead of silently passing
    /// its check.
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
        // NaN is not "no limit" and not "no draw": every comparison below
        // is false for NaN, so a NaN input would silently pass. Fail closed
        // with a dedicated violation instead. Infinity keeps flowing
        // through the comparisons — they already fail closed on it.
        if max_segment_m.is_nan() {
            violations.push(EthernetViolation::NonFiniteInput {
                field: "max_segment_m",
            });
        }
        if poe_draw_w.is_nan() {
            violations.push(EthernetViolation::NonFiniteInput {
                field: "poe_draw_w",
            });
        }
        let mut segment_len = 0.0f64;
        let mut longest_segment_m = 0.0f64;
        let mut poe_budget_w = 0.0f64;
        let mut min_bandwidth_mbps = u64::MAX;
        for edge in &path {
            match &edge.component {
                Component::EthernetRun { length_m, category } => {
                    if length_m.is_nan() {
                        // A NaN run length poisons the segment accounting
                        // (NaN + x is NaN, and NaN > limit is false), so it
                        // is reported and excluded rather than accumulated.
                        violations.push(EthernetViolation::NonFiniteInput { field: "length_m" });
                    } else {
                        segment_len += length_m.max(0.0);
                        longest_segment_m = longest_segment_m.max(segment_len);
                        min_bandwidth_mbps = min_bandwidth_mbps.min(category.bandwidth_mbps());
                    }
                }
                Component::Switch { poe_budget_w: b } => {
                    segment_len = 0.0;
                    if b.is_nan() {
                        violations.push(EthernetViolation::NonFiniteInput {
                            field: "poe_budget_w",
                        });
                    } else {
                        poe_budget_w += b;
                    }
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
    use crate::component::{Component, ConnectorType, PlantType, SpliceType, SplitterRatio};
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

    // ---- shared fixtures for the expanded suite ----

    /// TIA-568 channel limit for a copper segment.
    const ETHERNET_SEGMENT_M_MAX: f64 = 100.0;
    const GIGABIT_MBPS: u64 = 1_000;
    /// Layers in the dense-ladder adversary: 2^64 simple paths.
    const LADDER_LAYERS: u32 = 64;

    fn span(length_km: f64) -> Component {
        Component::Span {
            length_km,
            plant: PlantType::Buried,
        }
    }

    fn run(length_m: f64) -> Component {
        Component::EthernetRun {
            length_m,
            category: CableCategory::Cat6,
        }
    }

    fn gpon_budget(g: &PathGraph, target: NodeId) -> Result<LinkBudgetResult, PathError> {
        g.compute_link_budget(
            0,
            target,
            DEFAULT_TX_DBM,
            Wavelength::Nm1490,
            ReceiveWindow::GPON_ONT,
        )
    }

    // ---- validation: end-to-end budgets per medium ----

    #[test]
    fn pon_drop_lands_inside_the_gpon_window() {
        // OLT → APC → 20 km → fusion → 1×32 → fusion → APC → ONT.
        let mut g = PathGraph::default();
        let apc = Component::Connector {
            kind: ConnectorType::Apc,
            contamination_db: 0.0,
        };
        let fusion = Component::Splice {
            kind: SpliceType::Fusion,
            degradation_db: 0.0,
        };
        let chain = [
            apc.clone(),
            span(20.0),
            fusion.clone(),
            Component::Splitter {
                ratio: SplitterRatio::OneByThirtyTwo,
            },
            fusion,
            apc,
        ];
        for (hop, component) in (0u32..).zip(chain) {
            g.connect(hop, hop + 1, component);
        }
        let result = gpon_budget(&g, 6).unwrap();
        // 0.3 + 5.6 + 0.075 + 17.7 + 0.075 + 0.3 = 24.05 dB.
        assert_relative_eq!(result.total_loss_db, 24.05, epsilon = 1e-9);
        assert_relative_eq!(result.received_dbm, -21.05, epsilon = 1e-9);
        assert!(result.in_window);
        assert_relative_eq!(result.margin_db, 5.95, epsilon = 1e-9);
        assert_eq!(result.hop_count, 6);
    }

    #[test]
    fn wavelength_choice_shifts_the_received_level() {
        let mut g = PathGraph::default();
        g.connect(0, 1, span(20.0));
        let at = |wavelength| {
            g.compute_link_budget(0, 1, DEFAULT_TX_DBM, wavelength, ReceiveWindow::GPON_ONT)
                .unwrap()
                .received_dbm
        };
        // (0.35 − 0.21) dB/km × 20 km = 2.8 dB in favor of 1550 nm.
        assert_relative_eq!(
            at(Wavelength::Nm1550) - at(Wavelength::Nm1310),
            2.8,
            epsilon = 1e-9
        );
    }

    #[test]
    fn coax_amplifier_can_hit_exact_unity_gain() {
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 100.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 5.5 });
        let window = ReceiveWindow {
            min_dbm: 10.0,
            max_dbm: 20.0,
        };
        let result = g
            .compute_link_budget(0, 2, 15.0, Wavelength::Nm1490, window)
            .unwrap();
        assert_relative_eq!(result.total_loss_db, 0.0, epsilon = 1e-9);
        assert_relative_eq!(result.received_dbm, 15.0, epsilon = 1e-9);
    }

    #[test]
    fn ethernet_switch_resets_the_segment_length() {
        let mut g = PathGraph::default();
        g.connect(0, 1, run(65.0));
        g.connect(1, 2, Component::Switch { poe_budget_w: 30.0 });
        g.connect(2, 3, run(65.0));
        let eval = g
            .evaluate_ethernet(0, 3, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS)
            .unwrap();
        assert!(eval.passes());
        assert_relative_eq!(eval.longest_segment_m, 65.0, epsilon = 1e-9);
    }

    #[test]
    fn ethernet_poe_budget_pools_across_switches_and_equality_passes() {
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::Switch { poe_budget_w: 15.0 });
        g.connect(1, 2, run(10.0));
        g.connect(2, 3, Component::Switch { poe_budget_w: 15.0 });
        let eval = g
            .evaluate_ethernet(0, 3, ETHERNET_SEGMENT_M_MAX, 30.0, GIGABIT_MBPS)
            .unwrap();
        assert!(eval.passes(), "draw == pooled budget must pass");
        assert_relative_eq!(eval.poe_budget_w, 30.0, epsilon = 1e-9);
    }

    // ---- adversarial: topology edges ----

    #[test]
    fn empty_graph_is_disconnected() {
        let g = PathGraph::default();
        assert_eq!(gpon_budget(&g, 1).unwrap_err(), PathError::Disconnected);
    }

    #[test]
    fn source_equal_to_target_is_a_zero_hop_link() {
        let mut g = PathGraph::default();
        g.add_node(0, "OLT");
        let result = gpon_budget(&g, 0).unwrap();
        assert_eq!(result.hop_count, 0);
        assert_eq!(result.total_loss_db, 0.0);
        assert_eq!(result.received_dbm, DEFAULT_TX_DBM);
    }

    #[test]
    fn edges_are_directed() {
        // Fiber routed ONT → OLT does not carry light OLT → ONT.
        let mut g = PathGraph::default();
        g.connect(1, 0, span(1.0));
        assert_eq!(gpon_budget(&g, 1).unwrap_err(), PathError::Disconnected);
    }

    #[test]
    fn cycle_without_a_route_terminates_as_disconnected() {
        let mut g = PathGraph::default();
        g.connect(0, 1, span(1.0));
        g.connect(1, 2, span(1.0));
        g.connect(2, 0, span(1.0));
        assert_eq!(gpon_budget(&g, 9).unwrap_err(), PathError::Disconnected);
    }

    #[test]
    fn cycle_inserted_first_does_not_block_the_real_route() {
        let mut g = PathGraph::default();
        g.connect(0, 1, span(1.0));
        g.connect(1, 0, span(1.0)); // loop back, tried first from 1
        g.connect(1, 2, span(1.0));
        let result = gpon_budget(&g, 2).unwrap();
        assert_eq!(result.hop_count, 2);
    }

    #[test]
    fn dense_ladder_with_unreachable_target_is_bounded_work() {
        // Red team: 64 layers × 2 nodes, every node wired to both nodes of
        // the next layer, target hanging off nothing. A search that forgets
        // dead nodes on backtrack walks all 2^64 simple paths (never
        // finishes); a reachability search touches each node once.
        let mut g = PathGraph::default();
        let source: NodeId = 1_000;
        g.connect(source, 0, span(0.0));
        g.connect(source, 1, span(0.0));
        for layer in 0..LADDER_LAYERS - 1 {
            for from in [2 * layer, 2 * layer + 1] {
                g.connect(from, 2 * layer + 2, span(0.0));
                g.connect(from, 2 * layer + 3, span(0.0));
            }
        }
        let err = g
            .compute_link_budget(
                source,
                9_999,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
            )
            .unwrap_err();
        assert_eq!(err, PathError::Disconnected);
    }

    #[test]
    fn nan_launch_power_fails_closed() {
        let mut g = PathGraph::default();
        g.connect(0, 1, span(1.0));
        let result = g
            .compute_link_budget(0, 1, f64::NAN, Wavelength::Nm1490, ReceiveWindow::GPON_ONT)
            .unwrap();
        assert!(!result.in_window);
    }

    #[test]
    fn full_cut_on_a_still_connected_edge_adds_no_phantom_loss() {
        // Full cuts are the caller's job (edge removal); this method must
        // not double-count them as a degrade.
        let mut g = PathGraph::default();
        g.connect(0, 1, span(1.0));
        let mut cut = Outage::new(OutageKind::FiberCut, 0, 1);
        cut.tick(80.0);
        let plain = gpon_budget(&g, 1).unwrap();
        let with_cut = g
            .compute_link_budget_with_outage(
                0,
                1,
                DEFAULT_TX_DBM,
                Wavelength::Nm1490,
                ReceiveWindow::GPON_ONT,
                Some(&cut),
            )
            .unwrap();
        assert_eq!(with_cut, plain);
    }

    // ---- adversarial: Ethernet constraint edges ----

    #[test]
    fn ethernet_runs_without_a_switch_accumulate_into_one_segment() {
        // Two 65 m patch runs spliced end to end are one 130 m segment.
        let mut g = PathGraph::default();
        g.connect(0, 1, run(65.0));
        g.connect(1, 2, run(65.0));
        let eval = g
            .evaluate_ethernet(0, 2, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS)
            .unwrap();
        assert!(matches!(
            eval.violations.as_slice(),
            [EthernetViolation::SegmentTooLong { longest_m, .. }] if *longest_m == 130.0
        ));
    }

    #[test]
    fn ethernet_half_meter_over_the_limit_violates() {
        let mut g = PathGraph::default();
        g.connect(0, 1, run(ETHERNET_SEGMENT_M_MAX + 0.5));
        let eval = g
            .evaluate_ethernet(0, 1, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS)
            .unwrap();
        assert!(!eval.passes());
    }

    #[test]
    fn ethernet_negative_run_cannot_shorten_a_segment() {
        let mut g = PathGraph::default();
        g.connect(0, 1, run(130.0));
        g.connect(1, 2, run(-100.0));
        let eval = g
            .evaluate_ethernet(0, 2, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS)
            .unwrap();
        assert!(!eval.passes());
        assert_relative_eq!(eval.longest_segment_m, 130.0, epsilon = 1e-9);
    }

    #[test]
    fn ethernet_switch_only_path_reports_zero_bandwidth_without_violation() {
        // Documented contract: no runs → min_bandwidth_mbps reported as 0,
        // and no copper means no bandwidth bottleneck to flag.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::Switch { poe_budget_w: 30.0 });
        let eval = g
            .evaluate_ethernet(0, 1, ETHERNET_SEGMENT_M_MAX, 0.0, GIGABIT_MBPS)
            .unwrap();
        assert_eq!(eval.min_bandwidth_mbps, 0);
        assert!(eval.passes());
    }
}
