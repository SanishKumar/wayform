use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::fmt;

use crate::graph::{Graph, compare_ids};

/// A decision made by the policy/evidence evaluator for one input snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suitability {
    Suitable,
    Unsuitable,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct EdgeDecision {
    pub edge_id: String,
    pub suitability: Suitability,
    pub penalty_meters: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub edge_ids: Vec<String>,
    pub distance_meters: f64,
    pub cost_meters: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SearchOutcome {
    Routed(Path),
    /// Only a path containing unknown edges could connect the endpoints.
    InsufficientEvidence,
    Infeasible,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchError {
    UnknownNode(String),
    UnknownEdge(String),
    DuplicateDecision(String),
    MissingDecision(String),
    InvalidPenalty(String),
    UnrepresentableCost(String),
    WorkLimitExceeded,
}

impl fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(formatter, "unknown route endpoint: {id}"),
            Self::UnknownEdge(id) => write!(formatter, "decision references unknown edge: {id}"),
            Self::DuplicateDecision(id) => write!(formatter, "multiple decisions for edge: {id}"),
            Self::MissingDecision(id) => write!(formatter, "no decision for edge: {id}"),
            Self::InvalidPenalty(id) => {
                write!(formatter, "edge {id} needs a finite nonnegative penalty")
            }
            Self::UnrepresentableCost(id) => write!(
                formatter,
                "edge {id} cost exceeds numeric precision or range"
            ),
            Self::WorkLimitExceeded => write!(formatter, "route search exceeded its work budget"),
        }
    }
}

impl std::error::Error for SearchError {}

// Bounds path copying during this initial implementation, including discarded ties.
const MAX_PATH_WORK: usize = 1_000_000;

#[derive(Clone, Debug)]
struct Label {
    node: usize,
    cost: f64,
    distance: f64,
    path: Vec<usize>,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Label {}
impl PartialOrd for Label {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Label {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse the order for BinaryHeap's max-heap. Edge indices are sorted
        // by UTF-16 ID, so whole-path ordering matches the contract's tie rule.
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.path.cmp(&self.path))
            .then_with(|| other.node.cmp(&self.node))
    }
}

fn add_cost(total: f64, increment: f64, edge_id: &str) -> Result<f64, SearchError> {
    let next = total + increment;
    if !next.is_finite() || (increment > 0.0 && next <= total) {
        return Err(SearchError::UnrepresentableCost(edge_id.to_owned()));
    }
    Ok(next)
}

impl Graph {
    /// Search an immutable decision snapshot. Every edge needs an explicit decision.
    /// This method does not authenticate or compute those decisions.
    pub fn route(
        &self,
        start_id: &str,
        destination_id: &str,
        decisions: &[EdgeDecision],
    ) -> Result<SearchOutcome, SearchError> {
        let node = |id: &str| {
            self.nodes
                .get(id)
                .copied()
                .ok_or_else(|| SearchError::UnknownNode(id.to_owned()))
        };
        let start = node(start_id)?;
        let destination = node(destination_id)?;
        let mut by_edge = vec![None; self.edges.len()];
        for decision in decisions {
            let index = self
                .edges
                .binary_search_by(|edge| compare_ids(&edge.id, &decision.edge_id))
                .map_err(|_| SearchError::UnknownEdge(decision.edge_id.clone()))?;
            if by_edge[index].replace(decision).is_some() {
                return Err(SearchError::DuplicateDecision(decision.edge_id.clone()));
            }
            if !decision.penalty_meters.is_finite() || decision.penalty_meters < 0.0 {
                return Err(SearchError::InvalidPenalty(decision.edge_id.clone()));
            }
        }
        let by_edge = by_edge
            .into_iter()
            .enumerate()
            .map(|(index, decision)| {
                decision.ok_or_else(|| SearchError::MissingDecision(self.edges[index].id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let initial = Label {
            node: start,
            cost: 0.0,
            distance: 0.0,
            path: Vec::new(),
        };
        let mut best = vec![None::<Label>; self.nodes.len()];
        best[start] = Some(initial.clone());
        let mut queue = BinaryHeap::from([initial]);
        let mut work = 0;
        while let Some(current) = queue.pop() {
            if best[current.node].as_ref() != Some(&current) {
                continue;
            }
            if current.node == destination {
                return Ok(SearchOutcome::Routed(Path {
                    edge_ids: current
                        .path
                        .iter()
                        .map(|&index| self.edges[index].id.clone())
                        .collect(),
                    distance_meters: current.distance,
                    cost_meters: current.cost,
                }));
            }
            for &index in &self.outgoing[current.node] {
                let decision = by_edge[index];
                if decision.suitability != Suitability::Suitable {
                    continue;
                }
                let edge = &self.edges[index];
                let target = self.nodes[&edge.to];
                let weight = add_cost(edge.length_meters, decision.penalty_meters, &edge.id)?;
                let cost = add_cost(current.cost, weight, &edge.id)?;
                if best[target].as_ref().is_some_and(|old| cost > old.cost) {
                    continue;
                }
                work += (current.path.len() + 1) * 2;
                if work > MAX_PATH_WORK {
                    return Err(SearchError::WorkLimitExceeded);
                }
                let mut path = current.path.clone();
                path.push(index);
                if best[target]
                    .as_ref()
                    .is_some_and(|old| cost == old.cost && path >= old.path)
                {
                    continue;
                }
                let next = Label {
                    node: target,
                    cost,
                    distance: add_cost(current.distance, edge.length_meters, &edge.id)?,
                    path,
                };
                best[target] = Some(next.clone());
                queue.push(next);
            }
        }

        // Unknown edges are considered only to explain failure. No uncertain
        // path is returned for a user to follow.
        let mut seen = vec![false; self.nodes.len()];
        seen[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(current) = queue.pop_front() {
            if current == destination {
                return Ok(SearchOutcome::InsufficientEvidence);
            }
            for &index in &self.outgoing[current] {
                if by_edge[index].suitability == Suitability::Unsuitable {
                    continue;
                }
                let target = self.nodes[&self.edges[index].to];
                if !seen[target] {
                    seen[target] = true;
                    queue.push_back(target);
                }
            }
        }
        Ok(SearchOutcome::Infeasible)
    }
}
