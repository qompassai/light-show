//! Path graph: a level's OSP layout as nodes connected by `Component` edges.
//! Supports both simple point-to-point levels and branching PON trees
//! (one OLT feeding many ONTs through splitters).

use crate::component::Component;
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

    /// Walk edges from `source` to `target` (simple forward adjacency;
    /// levels are authored as DAGs so first-match traversal is sufficient
    /// and deterministic). Returns the ordered edge list actually used.
    fn resolve_path(&self, source: NodeId, target: NodeId) -> Result<Vec<&Edge>, PathError> {
        let mut adjacency: HashMap<NodeId, Vec<&Edge>> = HashMap::new();
        for edge in &self.edges {
            adjacency.entry(edge.from).or_default().push(edge);
        }

        let mut path = Vec::new();
        let mut current = source;
        let mut visited = std::collections::HashSet::new();
        visited.insert(current);

        while current != target {
            let Some(candidates) = adjacency.get(&current) else {
                return Err(PathError::Disconnected);
            };
            let Some(next_edge) = candidates.first() else {
                return Err(PathError::Disconnected);
            };
            path.push(*next_edge);
            current = next_edge.to;
            if !visited.insert(current) {
                return Err(PathError::Cycle);
            }
        }

        Ok(path)
    }

    /// Compute the received power at `target` given a launch power at
    /// `source`, following the routed path at the given wavelength, and
    /// evaluate it against a receive window.
    pub fn compute_link_budget(
        &self,
        source: NodeId,
        target: NodeId,
        tx_dbm: f64,
        wavelength: Wavelength,
        window: ReceiveWindow,
    ) -> Result<LinkBudgetResult, PathError> {
        let path = self.resolve_path(source, target)?;
        let total_loss_db: f64 = path.iter().map(|e| e.component.loss_db(wavelength)).sum();
        let received_dbm = tx_dbm - total_loss_db;
        Ok(LinkBudgetResult {
            total_loss_db,
            received_dbm,
            in_window: window.contains(received_dbm),
            margin_db: window.margin(received_dbm),
            hop_count: path.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{Component, PlantType};
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
}
