//! Path graph: a level's OSP layout as nodes connected by `Component` edges.
//! Supports both simple point-to-point levels and branching PON trees
//! (one OLT feeding many ONTs through splitters).

#[cfg(test)]
use crate::component::CableCategory;
use crate::component::{free_space_path_loss_db, Component, COAX_LOSS_DB_PER_M};
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
    SegmentTooLong {
        longest_m: f64,
        limit_m: f64,
    },
    PoeOverBudget {
        draw_w: f64,
        budget_w: f64,
    },
    BandwidthTooLow {
        min_mbps: u64,
        required_mbps: u64,
    },
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

/// Receiver-referred noise floor for the wireless evaluator, in dBm.
/// Thermal noise in a 20 MHz channel is about −101 dBm and a typical
/// receiver noise figure adds about 6 dB, landing near −95 dBm. It is
/// the floor every wireless reception is measured against: free space
/// attenuates the carrier, never the floor, so a hop's received SNR is
/// decided the moment the carrier lands.
pub const WIRELESS_NOISE_FLOOR_DBM: f64 = -95.0;

/// Noise figure of a single [`Component::Amplifier`] stage, in dB. An
/// amplifier boosts signal and noise alike and adds this much of its
/// own, so each amplified stage permanently costs the carried signal
/// this much SNR — the loss a regenerative [`Component::Repeater`]
/// avoids by decoding the signal and retransmitting it clean. 5 dB is
/// the textbook figure for a consumer RF amplifier stage.
pub const AMPLIFIER_NOISE_FIGURE_DB: f64 = 5.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CoaxEval {
    pub violations: Vec<CoaxViolation>,
    /// Carrier level at the target (the customer's tap), in dBmV.
    pub received_dbmv: f64,
    /// Carrier-to-noise ratio at the target, in dB: how far the carrier
    /// stands above the evaluated noise floor. Negative when the floor
    /// has swallowed the carrier entirely.
    pub carrier_to_noise_db: f64,
    pub hop_count: usize,
}

impl CoaxEval {
    pub fn passes(&self) -> bool {
        self.violations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoaxViolation {
    /// Carrier below the window: under-driven plant, the picture snows.
    LevelTooLow { received_dbmv: f64, min_dbmv: f64 },
    /// Carrier above the window: the cascade overdrives the drop into
    /// distortion.
    LevelTooHot { received_dbmv: f64, max_dbmv: f64 },
    /// Carrier inside the window but too close to the noise floor:
    /// ingress (a cracked shield, a loose F-connector) raised the floor
    /// until the carrier drowns in it, however balanced the levels are.
    CarrierToNoiseTooLow {
        cnr_db: f64,
        required_cnr_db: f64,
        noise_floor_dbmv: f64,
    },
    /// A NaN input reached the evaluator: comparisons against NaN are
    /// always false, so a NaN launch level, window edge, noise floor,
    /// span length, tap loss, or amplifier gain would silently pass
    /// every check. `field` names the offending input (`"tx_dbmv"`,
    /// `"window_min_dbm"`, `"window_max_dbm"`, `"noise_floor_dbmv"`,
    /// `"min_carrier_to_noise_db"`, `"length_m"`, `"tap_loss_db"`,
    /// `"gain_db"`, or `"received_dbmv"` when individually usable
    /// infinities cancelled into a NaN cascade). Infinity is
    /// deliberately not reported here — it flows through the ordinary
    /// comparisons, which already fail closed on it.
    NonFiniteInput { field: &'static str },
    UnsupportedComponent {
        edge_from: u32,
        edge_to: u32,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WirelessEval {
    pub violations: Vec<WirelessViolation>,
    /// Received signal level at the target, in dBm.
    pub received_dbm: f64,
    /// Worst SNR the carried signal has suffered by the time it reaches
    /// the target, in dB (see [`PathGraph::evaluate_wireless`] for how
    /// the carried SNR is tracked).
    pub snr_db: f64,
    /// Headroom above the window floor (`received − window.min`), in
    /// dB: how much fading the link can absorb before it drops out of
    /// the window. Negative when the signal is already below the floor.
    pub fade_margin_db: f64,
    pub hop_count: usize,
}

impl WirelessEval {
    pub fn passes(&self) -> bool {
        self.violations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WirelessViolation {
    /// Received level below the window: the link starves.
    SignalTooWeak { received_dbm: f64, min_dbm: f64 },
    /// Received level above the window: the receiver is cooked.
    SignalTooStrong { received_dbm: f64, max_dbm: f64 },
    /// Level inside the window but the carried SNR is wrecked — the
    /// amplifier-chain failure a regenerative repeater exists to avoid.
    SignalToNoiseTooLow { snr_db: f64, required_snr_db: f64 },
    /// A NaN (or, for a hop frequency, non-positive) input reached the
    /// evaluator: comparisons against NaN are always false, so it would
    /// silently pass every check, and the FSPL logarithm is undefined
    /// at or below 0 MHz. `field` names the offending input
    /// (`"tx_dbm"`, `"window_min_dbm"`, `"window_max_dbm"`,
    /// `"min_snr_db"`, `"distance_m"`, `"frequency_mhz"`, `"gain_db"`,
    /// `"repeater_tx_dbm"`, or `"received_dbm"` when individually
    /// usable infinities cancelled into a NaN cascade). Infinity is
    /// deliberately not reported here — it flows through the ordinary
    /// comparisons, which already fail closed on it.
    NonFiniteInput { field: &'static str },
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

    /// The running signal level along the connected chain that starts at
    /// `source`, following placed edges in insertion order until the
    /// chain breaks (a loop is cut off by the visited set, and the walk
    /// is bounded by the edge count). Returns `(level_dbm, hops,
    /// frontier_node)`: the level after the last connected hop — the
    /// live projection the HUD shows while the route is still
    /// incomplete, where [`PathGraph::compute_link_budget`] can only
    /// fail with `Disconnected`. With no outgoing edge from `source`,
    /// returns `(tx_dbm, 0, source)`.
    pub fn frontier_budget(
        &self,
        source: NodeId,
        tx_dbm: f64,
        wavelength: Wavelength,
    ) -> (f64, usize, NodeId) {
        let mut level_dbm = tx_dbm;
        let mut current = source;
        let mut hops = 0usize;
        let mut visited = std::collections::HashSet::new();
        visited.insert(source);
        while hops < self.edges.len() {
            let Some(edge) = self.edges.iter().find(|e| e.from == current) else {
                break;
            };
            level_dbm = edge.component.apply_level(level_dbm, wavelength);
            current = edge.to;
            hops += 1;
            if !visited.insert(current) {
                break;
            }
        }
        (level_dbm, hops, current)
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

    /// Evaluate a coax path as a dBmV cascade: launch level, per-span
    /// loss, amplifier gain staging, and tap loss, landing on the
    /// carrier level at the target (the customer's tap). The path
    /// passes when the carrier lands inside `window` — below it the
    /// picture snows, above it the drop distorts — *and* stands at
    /// least `min_carrier_to_noise_db` above `noise_floor_dbmv`.
    /// Ingress raises the floor rather than the loss, so a cascade can
    /// be perfectly balanced and still drown; that is a distinct
    /// verdict ([`CoaxViolation::CarrierToNoiseTooLow`]), not a level
    /// miss. The caller owns the floor figure: a quiet plant's thermal
    /// floor, raised by however much an active ingress hazard has
    /// accumulated (see `OutageKind::IngressNoise`).
    ///
    /// Amplifier noise is deliberately *not* added stage by stage
    /// here: the floor is an input precisely so the caller can fold
    /// cascade noise into it. What the cascade itself decides is level
    /// and the carrier's distance above that floor.
    ///
    /// Components that are not [`Component::CoaxSpan`],
    /// [`Component::Amplifier`], or [`Component::Tap`] are reported as
    /// [`CoaxViolation::UnsupportedComponent`] and contribute nothing.
    /// NaN inputs fail closed, exactly as in
    /// [`PathGraph::evaluate_ethernet`]: each is reported as a
    /// [`CoaxViolation::NonFiniteInput`] and excluded from the cascade
    /// instead of silently passing its check.
    ///
    /// Tilt across the band is deliberately *not* modeled: a
    /// [`Component::CoaxSpan`] carries a single loss coefficient at
    /// ~750 MHz ([`COAX_LOSS_DB_PER_M`]), so the component data holds
    /// no second frequency point from which a slope could be computed.
    /// Modeling tilt needs a band-edge pair or a second coefficient on
    /// the span (or an equalizer component); until the data grows one,
    /// the evaluator reports levels at the modeled frequency only.
    pub fn evaluate_coax(
        &self,
        source: NodeId,
        target: NodeId,
        tx_dbmv: f64,
        window: ReceiveWindow,
        noise_floor_dbmv: f64,
        min_carrier_to_noise_db: f64,
    ) -> Result<CoaxEval, PathError> {
        let path = self.resolve_path(source, target)?;
        let mut violations = Vec::new();
        // NaN is not "no requirement": every comparison below is false
        // for NaN, so a NaN input would silently pass. Fail closed with
        // a dedicated violation per offending input, as in Ethernet.
        if tx_dbmv.is_nan() {
            violations.push(CoaxViolation::NonFiniteInput { field: "tx_dbmv" });
        }
        if window.min_dbm.is_nan() {
            violations.push(CoaxViolation::NonFiniteInput {
                field: "window_min_dbm",
            });
        }
        if window.max_dbm.is_nan() {
            violations.push(CoaxViolation::NonFiniteInput {
                field: "window_max_dbm",
            });
        }
        if noise_floor_dbmv.is_nan() {
            violations.push(CoaxViolation::NonFiniteInput {
                field: "noise_floor_dbmv",
            });
        }
        if min_carrier_to_noise_db.is_nan() {
            violations.push(CoaxViolation::NonFiniteInput {
                field: "min_carrier_to_noise_db",
            });
        }
        let mut received_dbmv = tx_dbmv;
        for edge in &path {
            match &edge.component {
                Component::CoaxSpan { length_m } => {
                    if length_m.is_nan() {
                        violations.push(CoaxViolation::NonFiniteInput { field: "length_m" });
                    } else {
                        // Same flooring as `Component::loss_db`: a
                        // negative length is an authoring error that
                        // floors to zero loss, never negative loss.
                        received_dbmv -= length_m.max(0.0) * COAX_LOSS_DB_PER_M;
                    }
                }
                Component::Tap { tap_loss_db } => {
                    if tap_loss_db.is_nan() {
                        violations.push(CoaxViolation::NonFiniteInput {
                            field: "tap_loss_db",
                        });
                    } else {
                        // Mirror `passive_loss_db` in component.rs: a
                        // finite negative loss floors to 0 (passive
                        // plant never adds power); ±inf propagates and
                        // fails closed in the comparisons below.
                        let loss_db = if tap_loss_db.is_finite() {
                            tap_loss_db.max(0.0)
                        } else {
                            *tap_loss_db
                        };
                        received_dbmv -= loss_db;
                    }
                }
                Component::Amplifier { gain_db } => {
                    if gain_db.is_nan() {
                        violations.push(CoaxViolation::NonFiniteInput { field: "gain_db" });
                    } else {
                        received_dbmv += gain_db;
                    }
                }
                other => {
                    violations.push(CoaxViolation::UnsupportedComponent {
                        edge_from: edge.from,
                        edge_to: edge.to,
                        detail: format!("{other:?} is not a coax component"),
                    });
                }
            }
        }
        if received_dbmv.is_nan()
            && !violations
                .iter()
                .any(|v| matches!(v, CoaxViolation::NonFiniteInput { .. }))
        {
            // Reachable only when +inf gain meets +inf loss: every input
            // was individually usable, but the cascade cancelled into
            // NaN, which would pass both level comparisons. Fail closed.
            violations.push(CoaxViolation::NonFiniteInput {
                field: "received_dbmv",
            });
        }
        if received_dbmv < window.min_dbm {
            violations.push(CoaxViolation::LevelTooLow {
                received_dbmv,
                min_dbmv: window.min_dbm,
            });
        }
        if received_dbmv > window.max_dbm {
            violations.push(CoaxViolation::LevelTooHot {
                received_dbmv,
                max_dbmv: window.max_dbm,
            });
        }
        let carrier_to_noise_db = received_dbmv - noise_floor_dbmv;
        if carrier_to_noise_db < min_carrier_to_noise_db {
            violations.push(CoaxViolation::CarrierToNoiseTooLow {
                cnr_db: carrier_to_noise_db,
                required_cnr_db: min_carrier_to_noise_db,
                noise_floor_dbmv,
            });
        }
        Ok(CoaxEval {
            violations,
            received_dbmv,
            carrier_to_noise_db,
            hop_count: path.len(),
        })
    }

    /// Evaluate a wireless path hop by hop: free-space path loss from
    /// each hop's geometry, gain staging, and — the discipline's core
    /// lesson — noise. Two quantities travel the path: the carrier
    /// level and the worst SNR the carried signal has suffered. A hop
    /// decides the SNR at its receiving end (carrier against
    /// [`WIRELESS_NOISE_FLOOR_DBM`]: free space attenuates the carrier,
    /// never the floor) and the carried SNR keeps the worse figure from
    /// then on. An [`Component::Amplifier`] preserves the damage and
    /// adds its own noise figure ([`AMPLIFIER_NOISE_FIGURE_DB`]) — it
    /// shouts the wreckage louder. A [`Component::Repeater`] decodes
    /// and retransmits at its own power, resetting the carried SNR to
    /// clean. The distinction is mechanical, not narrative: an amp
    /// chain can land the level dead-center in the window with a
    /// wrecked SNR, where the same levels through a repeater pass.
    ///
    /// The path passes when the received level lands inside `window`
    /// and the carried SNR meets `min_snr_db`. When no reception has
    /// happened since the last regeneration (or at all), the signal on
    /// the table is fresh and its reported SNR is the received level
    /// against the floor.
    ///
    /// Antenna gain is not a separate term: no antenna component
    /// exists in the data model, so a level's launch power is already
    /// the effective radiated figure its author chose, and mid-path
    /// gain stages are [`Component::Amplifier`]s. Components that are
    /// not [`Component::WirelessHop`], [`Component::Amplifier`], or
    /// [`Component::Repeater`] are reported as
    /// [`WirelessViolation::UnsupportedComponent`] and contribute
    /// nothing. NaN inputs fail closed, exactly as in
    /// [`PathGraph::evaluate_ethernet`]; a non-positive hop frequency
    /// is reported the same way, because the FSPL logarithm is
    /// undefined there and the dB-budget path's flooring would
    /// otherwise route a corrupt hop for free.
    pub fn evaluate_wireless(
        &self,
        source: NodeId,
        target: NodeId,
        tx_dbm: f64,
        window: ReceiveWindow,
        min_snr_db: f64,
    ) -> Result<WirelessEval, PathError> {
        let path = self.resolve_path(source, target)?;
        let mut violations = Vec::new();
        if tx_dbm.is_nan() {
            violations.push(WirelessViolation::NonFiniteInput { field: "tx_dbm" });
        }
        if window.min_dbm.is_nan() {
            violations.push(WirelessViolation::NonFiniteInput {
                field: "window_min_dbm",
            });
        }
        if window.max_dbm.is_nan() {
            violations.push(WirelessViolation::NonFiniteInput {
                field: "window_max_dbm",
            });
        }
        if min_snr_db.is_nan() {
            violations.push(WirelessViolation::NonFiniteInput {
                field: "min_snr_db",
            });
        }
        let mut received_dbm = tx_dbm;
        // The transmitted signal starts clean; the first reception sets
        // the carried SNR, and no downstream device can improve it —
        // only regeneration resets it.
        let mut carried_snr_db = f64::INFINITY;
        for edge in &path {
            match &edge.component {
                Component::WirelessHop {
                    distance_m,
                    frequency_mhz,
                } => {
                    let mut usable = true;
                    if distance_m.is_nan() {
                        violations.push(WirelessViolation::NonFiniteInput {
                            field: "distance_m",
                        });
                        usable = false;
                    }
                    if frequency_mhz.is_nan() || *frequency_mhz <= 0.0 {
                        violations.push(WirelessViolation::NonFiniteInput {
                            field: "frequency_mhz",
                        });
                        usable = false;
                    }
                    if usable {
                        // Same flooring as `Component::loss_db`: the
                        // distance floors at 0.1 m (log10(0) is −inf)
                        // and the loss at 0 dB (free space is passive).
                        let loss_db =
                            free_space_path_loss_db(distance_m.max(0.1), *frequency_mhz).max(0.0);
                        received_dbm -= loss_db;
                        let received_snr_db = received_dbm - WIRELESS_NOISE_FLOOR_DBM;
                        carried_snr_db = carried_snr_db.min(received_snr_db);
                    }
                }
                Component::Amplifier { gain_db } => {
                    if gain_db.is_nan() {
                        violations.push(WirelessViolation::NonFiniteInput { field: "gain_db" });
                    } else {
                        received_dbm += gain_db;
                        carried_snr_db -= AMPLIFIER_NOISE_FIGURE_DB;
                    }
                }
                Component::Repeater { tx_dbm: hop_tx_dbm } => {
                    if hop_tx_dbm.is_nan() {
                        violations.push(WirelessViolation::NonFiniteInput {
                            field: "repeater_tx_dbm",
                        });
                    } else {
                        received_dbm = *hop_tx_dbm;
                        carried_snr_db = f64::INFINITY;
                    }
                }
                other => {
                    violations.push(WirelessViolation::UnsupportedComponent {
                        edge_from: edge.from,
                        edge_to: edge.to,
                        detail: format!("{other:?} is not a wireless component"),
                    });
                }
            }
        }
        if received_dbm.is_nan()
            && !violations
                .iter()
                .any(|v| matches!(v, WirelessViolation::NonFiniteInput { .. }))
        {
            // Same +inf/−inf cancellation guard as the coax evaluator.
            violations.push(WirelessViolation::NonFiniteInput {
                field: "received_dbm",
            });
        }
        if received_dbm < window.min_dbm {
            violations.push(WirelessViolation::SignalTooWeak {
                received_dbm,
                min_dbm: window.min_dbm,
            });
        }
        if received_dbm > window.max_dbm {
            violations.push(WirelessViolation::SignalTooStrong {
                received_dbm,
                max_dbm: window.max_dbm,
            });
        }
        let snr_db = if carried_snr_db.is_finite() {
            carried_snr_db
        } else {
            // No reception since the last regeneration (or at all): the
            // signal on the table is fresh, so its SNR is the level
            // against the floor.
            received_dbm - WIRELESS_NOISE_FLOOR_DBM
        };
        if snr_db < min_snr_db {
            violations.push(WirelessViolation::SignalToNoiseTooLow {
                snr_db,
                required_snr_db: min_snr_db,
            });
        }
        Ok(WirelessEval {
            violations,
            received_dbm,
            snr_db,
            fade_margin_db: received_dbm - window.min_dbm,
            hop_count: path.len(),
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
    fn frontier_budget_empty_graph_is_tx_at_source() {
        let g = PathGraph::default();
        let (level, hops, frontier) = g.frontier_budget(0, 3.0, Wavelength::Nm1490);
        assert_relative_eq!(level, 3.0, epsilon = 1e-9);
        assert_eq!(hops, 0);
        assert_eq!(frontier, 0);
    }

    #[test]
    fn frontier_budget_walks_the_connected_chain_only() {
        let mut g = PathGraph::default();
        for id in 0..4 {
            g.add_node(id, "N");
        }
        // Connected chain 0 -> 1 -> 2; the 2 -> 3 edge is absent, and an
        // unrelated edge 3 -> 0 must not be walked.
        g.connect(
            0,
            1,
            Component::Span {
                length_km: 10.0,
                plant: PlantType::Buried,
            },
        );
        g.connect(
            1,
            2,
            Component::Splice {
                kind: SpliceType::Fusion,
                degradation_db: 0.0,
            },
        );
        g.connect(
            3,
            0,
            Component::Span {
                length_km: 50.0,
                plant: PlantType::Aerial,
            },
        );
        let (level, hops, frontier) = g.frontier_budget(0, 3.0, Wavelength::Nm1490);
        assert_eq!(hops, 2);
        assert_eq!(frontier, 2);
        // 3.0 - 2.8 (span) - 0.075 (fusion splice) = 0.125 dBm.
        assert_relative_eq!(level, 0.125, epsilon = 1e-9);
    }

    #[test]
    fn frontier_budget_loop_terminates() {
        let mut g = PathGraph::default();
        g.add_node(0, "A");
        g.add_node(1, "B");
        g.connect(
            0,
            1,
            Component::Splice {
                kind: SpliceType::Fusion,
                degradation_db: 0.0,
            },
        );
        g.connect(
            1,
            0,
            Component::Splice {
                kind: SpliceType::Fusion,
                degradation_db: 0.0,
            },
        );
        let (_level, hops, _frontier) = g.frontier_budget(0, 3.0, Wavelength::Nm1490);
        assert!(hops <= 2, "loop must be cut off by the visited set, got {hops}");
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
        let eval = g.evaluate_ethernet(0, 2, 100.0, 25.0, 1_000).unwrap();
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
        let err = g.evaluate_ethernet(0, 1, 100.0, 0.0, 1_000).unwrap_err();
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

    // ---- per-specialty evaluators: fixtures ----

    /// Ondine's receive window from the coax levels: [0, 15] dBmV.
    fn coax_window() -> ReceiveWindow {
        ReceiveWindow {
            min_dbm: 0.0,
            max_dbm: 15.0,
        }
    }

    /// Linka's receive window from wireless1: [−75, −40] dBm.
    fn wireless_window() -> ReceiveWindow {
        ReceiveWindow {
            min_dbm: -75.0,
            max_dbm: -40.0,
        }
    }

    fn hop(distance_m: f64) -> Component {
        Component::WirelessHop {
            distance_m,
            frequency_mhz: 2400.0,
        }
    }

    // ---- validation: per-specialty evaluators ----

    #[test]
    fn coax_eval_lands_coax1_unity_gain_in_window() {
        // coax1_unity_gain.json, verbatim: 490 m of coax, the 5 dB amp,
        // an 8 dB tap, launched at +35 dBmV into the [0, 15] window.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 490.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 5.0 });
        g.connect(2, 3, Component::Tap { tap_loss_db: 8.0 });
        let eval = g
            .evaluate_coax(0, 3, 35.0, coax_window(), -30.0, 25.0)
            .unwrap();
        // The briefing's plant: 490 × 0.055 = 26.95 dB of span plus the
        // 8 dB tap is 34.95 dB; 35 − 34.95 + 5 = 5.05 dBmV at the drop,
        // standing 5.05 − (−30) = 35.05 dB above the noise floor.
        assert_relative_eq!(eval.received_dbmv, 5.05, epsilon = 1e-9);
        assert_relative_eq!(eval.carrier_to_noise_db, 35.05, epsilon = 1e-9);
        assert_eq!(eval.hop_count, 3);
        assert!(eval.passes(), "violations: {:?}", eval.violations);
    }

    #[test]
    fn coax_eval_flags_coax1s_oversized_amp_as_too_hot() {
        // Same plant, the 20 dB amp from coax1's part list instead:
        // 35 − 26.95 + 20 − 8 = 20.05 dBmV, over the window top.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 490.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 20.0 });
        g.connect(2, 3, Component::Tap { tap_loss_db: 8.0 });
        let eval = g
            .evaluate_coax(0, 3, 35.0, coax_window(), -30.0, 25.0)
            .unwrap();
        assert_relative_eq!(eval.received_dbmv, 20.05, epsilon = 1e-9);
        assert!(!eval.passes());
        assert!(matches!(
            eval.violations.as_slice(),
            [CoaxViolation::LevelTooHot { .. }]
        ));
    }

    #[test]
    fn coax_eval_starved_cascade_is_too_cold() {
        // 900 m of span (49.5 dB) outruns the 5 dB amp: 35 − 49.5 + 5
        // − 8 = −17.5 dBmV, below the window — snow.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 900.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 5.0 });
        g.connect(2, 3, Component::Tap { tap_loss_db: 8.0 });
        let eval = g
            .evaluate_coax(0, 3, 35.0, coax_window(), -30.0, 10.0)
            .unwrap();
        assert_relative_eq!(eval.received_dbmv, -17.5, epsilon = 1e-9);
        assert!(matches!(
            eval.violations.as_slice(),
            [CoaxViolation::LevelTooLow { .. }]
        ));
    }

    #[test]
    fn coax_eval_window_top_edge_is_inclusive() {
        // 35 − 22 (400 m) + 10 − 8 lands exactly on the 15 dBmV top
        // edge; the window is inclusive, as in `ReceiveWindow`.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 400.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 10.0 });
        g.connect(2, 3, Component::Tap { tap_loss_db: 8.0 });
        let eval = g
            .evaluate_coax(0, 3, 35.0, coax_window(), -30.0, 25.0)
            .unwrap();
        assert_relative_eq!(eval.received_dbmv, 15.0, epsilon = 1e-9);
        assert!(eval.passes(), "violations: {:?}", eval.violations);
    }

    #[test]
    fn wireless_eval_lands_wireless1_repeater_in_window() {
        // wireless1_close_the_link.json, verbatim: two 400 m hops at
        // 2.4 GHz, the 20 dBm regenerative repeater between them,
        // launched at 20 dBm into the [−75, −40] window.
        let mut g = PathGraph::default();
        g.connect(0, 1, hop(400.0));
        g.connect(1, 2, Component::Repeater { tx_dbm: 20.0 });
        g.connect(2, 3, hop(400.0));
        let eval = g
            .evaluate_wireless(0, 3, 20.0, wireless_window(), 20.0)
            .unwrap();
        // ≈92.1 dB of FSPL per hop: the carrier lands at ≈−72.1 dBm on
        // both receptions, so the carried SNR is −72.1 − (−95) ≈ 22.9
        // and the fade margin above the window floor is ≈2.9 dB.
        assert_relative_eq!(eval.received_dbm, -72.1, epsilon = 0.01);
        assert_relative_eq!(eval.snr_db, 22.9, epsilon = 0.01);
        assert_relative_eq!(eval.fade_margin_db, 2.9, epsilon = 0.01);
        assert_eq!(eval.hop_count, 3);
        assert!(eval.passes(), "violations: {:?}", eval.violations);
    }

    #[test]
    fn wireless_eval_cooked_receiver_is_too_strong() {
        // A 1 m hop at 2.4 GHz costs only ≈40.1 dB: 20 − 40.1 ≈
        // −20.1 dBm arrives over the −40 window top.
        let mut g = PathGraph::default();
        g.connect(0, 1, hop(1.0));
        let eval = g
            .evaluate_wireless(0, 1, 20.0, wireless_window(), 20.0)
            .unwrap();
        assert_relative_eq!(eval.received_dbm, -20.05, epsilon = 0.01);
        assert!(matches!(
            eval.violations.as_slice(),
            [WirelessViolation::SignalTooStrong { .. }]
        ));
    }

    // ---- adversarial: per-specialty evaluators ----

    #[test]
    fn coax_eval_ingress_buries_an_in_window_carrier() {
        // coax2's failure mode, frozen at one instant: the cascade from
        // coax1 is balanced (5.05 dBmV, in window), but ingress has
        // raised the floor to −15 dBmV, leaving only ≈20 dB of CNR
        // against a 25 dB requirement. The level verdict must stay
        // clean — the carrier drowns, it does not move.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 490.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: 5.0 });
        g.connect(2, 3, Component::Tap { tap_loss_db: 8.0 });
        let eval = g
            .evaluate_coax(0, 3, 35.0, coax_window(), -15.0, 25.0)
            .unwrap();
        assert_relative_eq!(eval.received_dbmv, 5.05, epsilon = 1e-9);
        assert_relative_eq!(eval.carrier_to_noise_db, 20.05, epsilon = 1e-9);
        assert!(matches!(
            eval.violations.as_slice(),
            [CoaxViolation::CarrierToNoiseTooLow { .. }]
        ));
    }

    #[test]
    fn wireless_eval_amp_chain_dies_of_noise_where_a_repeater_succeeds() {
        // Three 400 m hops (≈92.1 dB each). Two 95 dB amplifiers rescue
        // the *level* to ≈−66.3 dBm — dead-center in the window — but
        // each amp stage burns 5 dB of the SNR the weak first reception
        // set (≈22.9 dB), leaving ≈12.9 dB against a 20 dB requirement.
        let mut amps = PathGraph::default();
        amps.connect(0, 1, hop(400.0));
        amps.connect(1, 2, Component::Amplifier { gain_db: 95.0 });
        amps.connect(2, 3, hop(400.0));
        amps.connect(3, 4, Component::Amplifier { gain_db: 95.0 });
        amps.connect(4, 5, hop(400.0));
        let amp_eval = amps
            .evaluate_wireless(0, 5, 20.0, wireless_window(), 20.0)
            .unwrap();
        assert_relative_eq!(amp_eval.received_dbm, -66.29, epsilon = 0.01);
        assert_relative_eq!(amp_eval.snr_db, 12.9, epsilon = 0.01);
        assert!(matches!(
            amp_eval.violations.as_slice(),
            [WirelessViolation::SignalToNoiseTooLow { .. }]
        ));

        // The twin path, identical except the second amp is a repeater
        // transmitting at the level the amp would have delivered
        // (25.81 dBm): same received level within a hundredth of a dB,
        // but regeneration hands the last hop a clean signal and the
        // link passes. The verdict difference is purely the noise
        // mechanics — which is the lesson wireless1's briefing teaches.
        let mut regen = PathGraph::default();
        regen.connect(0, 1, hop(400.0));
        regen.connect(1, 2, Component::Amplifier { gain_db: 95.0 });
        regen.connect(2, 3, hop(400.0));
        regen.connect(3, 4, Component::Repeater { tx_dbm: 25.81 });
        regen.connect(4, 5, hop(400.0));
        let regen_eval = regen
            .evaluate_wireless(0, 5, 20.0, wireless_window(), 20.0)
            .unwrap();
        assert_relative_eq!(regen_eval.received_dbm, -66.29, epsilon = 0.01);
        assert_relative_eq!(
            regen_eval.received_dbm,
            amp_eval.received_dbm,
            epsilon = 0.01
        );
        assert!(
            regen_eval.passes(),
            "violations: {:?}",
            regen_eval.violations
        );
    }

    #[test]
    fn wireless_eval_wireless1_amplifier_fails_on_level_and_noise() {
        // wireless1's trap answer, through the new evaluator: the
        // 30 dB amp boosts the first hop's wreckage and the second hop
        // buries it again (≈−134.2 dBm), and the carried SNR is the
        // wreckage's, not the level's.
        let mut g = PathGraph::default();
        g.connect(0, 1, hop(400.0));
        g.connect(1, 2, Component::Amplifier { gain_db: 30.0 });
        g.connect(2, 3, hop(400.0));
        let eval = g
            .evaluate_wireless(0, 3, 20.0, wireless_window(), 20.0)
            .unwrap();
        assert_relative_eq!(eval.received_dbm, -134.19, epsilon = 0.01);
        assert!(!eval.passes());
        assert!(eval
            .violations
            .iter()
            .any(|v| matches!(v, WirelessViolation::SignalTooWeak { .. })));
        assert!(eval
            .violations
            .iter()
            .any(|v| matches!(v, WirelessViolation::SignalToNoiseTooLow { .. })));
    }

    #[test]
    fn coax_eval_disconnected_still_errors() {
        let mut g = PathGraph::default();
        g.add_node(0, "Headend");
        g.add_node(1, "Customer Drop");
        let err = g
            .evaluate_coax(0, 1, 35.0, coax_window(), -30.0, 25.0)
            .unwrap_err();
        assert_eq!(err, PathError::Disconnected);
    }

    #[test]
    fn wireless_eval_disconnected_still_errors() {
        let mut g = PathGraph::default();
        g.add_node(0, "Site A");
        g.add_node(1, "Site B");
        let err = g
            .evaluate_wireless(0, 1, 20.0, wireless_window(), 20.0)
            .unwrap_err();
        assert_eq!(err, PathError::Disconnected);
    }

    #[test]
    fn coax_eval_flags_a_fiber_span_as_unsupported() {
        // Adversarial: a fiber span smuggled into a coax cascade.
        let mut g = PathGraph::default();
        g.connect(0, 1, span(1.0));
        let eval = g
            .evaluate_coax(0, 1, 10.0, coax_window(), -30.0, 25.0)
            .unwrap();
        assert!(matches!(
            eval.violations.as_slice(),
            [CoaxViolation::UnsupportedComponent { .. }]
        ));
    }

    #[test]
    fn wireless_eval_flags_a_coax_span_as_unsupported() {
        // Adversarial: a coax span smuggled into a wireless path.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 100.0 });
        let eval = g
            .evaluate_wireless(0, 1, -50.0, wireless_window(), 20.0)
            .unwrap();
        assert!(matches!(
            eval.violations.as_slice(),
            [WirelessViolation::UnsupportedComponent { .. }]
        ));
    }

    #[test]
    fn coax_eval_nan_amplifier_gain_fails_closed() {
        // A NaN-gain amp on coax1's plant: excluded from the cascade
        // (the level still lands at 0.05 dBmV on span + tap alone), and
        // the corrupt input is the one and only verdict.
        let mut g = PathGraph::default();
        g.connect(0, 1, Component::CoaxSpan { length_m: 490.0 });
        g.connect(1, 2, Component::Amplifier { gain_db: f64::NAN });
        g.connect(2, 3, Component::Tap { tap_loss_db: 8.0 });
        let eval = g
            .evaluate_coax(0, 3, 35.0, coax_window(), -30.0, 25.0)
            .unwrap();
        assert!(!eval.passes());
        assert!(matches!(
            eval.violations.as_slice(),
            [CoaxViolation::NonFiniteInput { field }] if *field == "gain_db"
        ));
    }

    #[test]
    fn wireless_eval_nan_hop_distance_fails_closed() {
        // A NaN-distance hop followed by a sound 400 m hop: the corrupt
        // hop contributes nothing, the sound one still lands in window,
        // and the corrupt input is the one and only verdict.
        let mut g = PathGraph::default();
        g.connect(
            0,
            1,
            Component::WirelessHop {
                distance_m: f64::NAN,
                frequency_mhz: 2400.0,
            },
        );
        g.connect(1, 2, hop(400.0));
        let eval = g
            .evaluate_wireless(0, 2, 20.0, wireless_window(), 20.0)
            .unwrap();
        assert!(!eval.passes());
        assert!(matches!(
            eval.violations.as_slice(),
            [WirelessViolation::NonFiniteInput { field }] if *field == "distance_m"
        ));
    }

    #[test]
    fn wireless_eval_zero_frequency_fails_closed() {
        // A 0 MHz hop: log10 is undefined there, and the dB-budget path
        // floors the resulting NaN loss to 0 dB — a free hop. The
        // evaluator must refuse it instead, while the sound second hop
        // still lands in window.
        let mut g = PathGraph::default();
        g.connect(
            0,
            1,
            Component::WirelessHop {
                distance_m: 400.0,
                frequency_mhz: 0.0,
            },
        );
        g.connect(1, 2, hop(400.0));
        let eval = g
            .evaluate_wireless(0, 2, 20.0, wireless_window(), 20.0)
            .unwrap();
        assert!(!eval.passes());
        assert!(matches!(
            eval.violations.as_slice(),
            [WirelessViolation::NonFiniteInput { field }] if *field == "frequency_mhz"
        ));
    }
}
