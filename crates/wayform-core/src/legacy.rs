//! Search-only compatibility with the pinned VoiceGIS routingCore.ts.
//! Source: SanishKumar/voicegis-indoor-ar, commit 305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f.
//! Original source and rights statement remain under compat/voicegis/upstream/.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{MAX_EDGES, MAX_NODES};

const MAX_SEARCH_WORK: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LegacyFloor {
    Text(String),
    Number(f64),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub floor: LegacyFloor,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub distance: f64,
    pub accessible: bool,
    pub restricted: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyRouteOptions {
    pub accessible_only: bool,
    pub allow_restricted: bool,
    pub closed_edge_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum LegacySearchOutcome {
    Routed {
        #[serde(rename = "pathIds")]
        path_ids: Vec<String>,
        #[serde(rename = "totalDistance")]
        total_distance: f64,
    },
    UnknownEndpoint,
    NoRoute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacySearchError {
    InvalidGraph,
    InvalidClosure(String),
    UnrepresentableCost,
    WorkLimitExceeded,
}
impl fmt::Display for LegacySearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGraph => f.write_str("invalid legacy search graph"),
            Self::InvalidClosure(id) => write!(f, "unknown legacy closure edge: {id}"),
            Self::UnrepresentableCost => {
                f.write_str("legacy search arithmetic exceeds its numeric range or precision")
            }
            Self::WorkLimitExceeded => f.write_str("legacy search exceeded its work limit"),
        }
    }
}
impl std::error::Error for LegacySearchError {}

#[derive(Clone, Copy, Debug)]
struct Entry {
    node: usize,
    cost: f64,
    priority: f64,
}

// This queue deliberately has no secondary tie-breaker. Equal priorities stop
// bubbling, and sink-down considers the left child before the right, just as the
// pinned implementation does. BinaryHeap's ordering is not interchangeable.
#[derive(Default)]
struct LegacyQueue(Vec<Entry>);
impl LegacyQueue {
    fn push(&mut self, entry: Entry) {
        self.0.push(entry);
        let mut index = self.0.len() - 1;
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.0[parent].priority <= self.0[index].priority {
                break;
            }
            self.0.swap(parent, index);
            index = parent;
        }
    }
    fn pop(&mut self) -> Option<Entry> {
        let first = *self.0.first()?;
        let last = self.0.pop()?;
        if !self.0.is_empty() {
            self.0[0] = last;
            let mut index = 0;
            loop {
                let left = index * 2 + 1;
                let right = left + 1;
                let mut smallest = index;
                if left < self.0.len() && self.0[left].priority < self.0[smallest].priority {
                    smallest = left;
                }
                if right < self.0.len() && self.0[right].priority < self.0[smallest].priority {
                    smallest = right;
                }
                if smallest == index {
                    break;
                }
                self.0.swap(index, smallest);
                index = smallest;
            }
        }
        Some(first)
    }
}

// Scaled Euclidean norm with explicit binary64 operations. Keep the operation
// order used by the reference runtime's two-argument Math.hypot; see the native
// legacy-search notes for the source comparison and numerical limitations.
fn coordinate_distance(a: &LegacyNode, b: &LegacyNode) -> Result<f64, LegacySearchError> {
    let x = (b.x - a.x).abs();
    let y = (b.y - a.y).abs();
    let scale = x.max(y);
    if !scale.is_finite() {
        return Err(LegacySearchError::UnrepresentableCost);
    }
    if scale == 0.0 {
        return Ok(0.0);
    }
    let nx = x / scale;
    let ny = y / scale;
    let length = (nx * nx + ny * ny).sqrt() * scale;
    if length.is_finite() {
        Ok(length)
    } else {
        Err(LegacySearchError::UnrepresentableCost)
    }
}

#[derive(Clone, Debug)]
pub struct LegacyGraph {
    nodes: Vec<LegacyNode>,
    node_index: HashMap<String, usize>,
    edges: Vec<LegacyEdge>,
    edge_index: HashMap<String, usize>,
    adjacency: Vec<Vec<(usize, usize)>>,
    heuristic_scale: f64,
}

impl LegacyGraph {
    /// Expose the preserved scale for numerical conformance diagnostics.
    pub fn heuristic_scale(&self) -> f64 {
        self.heuristic_scale
    }

    /// Accept an explicitly projected, verified legacy graph. Unlike the old
    /// low-level helper, missing flags and malformed topology are rejected.
    pub fn new(nodes: Vec<LegacyNode>, edges: Vec<LegacyEdge>) -> Result<Self, LegacySearchError> {
        if nodes.is_empty() || nodes.len() > MAX_NODES || edges.len() > MAX_EDGES {
            return Err(LegacySearchError::InvalidGraph);
        }
        let mut node_index = HashMap::new();
        for (index, node) in nodes.iter().enumerate() {
            let floor_valid = match &node.floor {
                LegacyFloor::Text(value) => !value.is_empty(),
                LegacyFloor::Number(value) => value.is_finite(),
            };
            if node.id.is_empty()
                || !node.x.is_finite()
                || !node.y.is_finite()
                || !floor_valid
                || node_index.insert(node.id.clone(), index).is_some()
            {
                return Err(LegacySearchError::InvalidGraph);
            }
        }
        let mut adjacency = vec![Vec::new(); nodes.len()];
        let mut edge_index = HashMap::new();
        let mut heuristic_scale = f64::INFINITY;
        for (index, edge) in edges.iter().enumerate() {
            if edge.id.is_empty()
                || !edge.distance.is_finite()
                || edge.distance <= 0.0
                || edge_index.insert(edge.id.clone(), index).is_some()
            {
                return Err(LegacySearchError::InvalidGraph);
            }
            let from = *node_index
                .get(&edge.from)
                .ok_or(LegacySearchError::InvalidGraph)?;
            let to = *node_index
                .get(&edge.to)
                .ok_or(LegacySearchError::InvalidGraph)?;
            adjacency[from].push((index, to));
            adjacency[to].push((index, from));
            if nodes[from].floor == nodes[to].floor {
                let length = coordinate_distance(&nodes[from], &nodes[to])?;
                if length > 0.0 {
                    heuristic_scale = heuristic_scale.min(edge.distance / length);
                }
            }
        }
        if !heuristic_scale.is_finite() {
            heuristic_scale = 0.0;
        }
        Ok(Self {
            nodes,
            node_index,
            edges,
            edge_index,
            adjacency,
            heuristic_scale,
        })
    }

    fn estimate(&self, from: usize, to: usize) -> Result<f64, LegacySearchError> {
        if self.nodes[from].floor != self.nodes[to].floor {
            return Ok(0.0);
        }
        let value = coordinate_distance(&self.nodes[from], &self.nodes[to])? * self.heuristic_scale;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(LegacySearchError::UnrepresentableCost)
        }
    }

    pub fn route(
        &self,
        start_id: &str,
        destination_id: &str,
        options: &LegacyRouteOptions,
    ) -> Result<LegacySearchOutcome, LegacySearchError> {
        if options.closed_edge_ids.len() > MAX_EDGES {
            return Err(LegacySearchError::WorkLimitExceeded);
        }
        let mut closed = HashSet::new();
        for id in &options.closed_edge_ids {
            closed.insert(
                *self
                    .edge_index
                    .get(id)
                    .ok_or_else(|| LegacySearchError::InvalidClosure(id.clone()))?,
            );
        }
        let (Some(&start), Some(&destination)) = (
            self.node_index.get(start_id),
            self.node_index.get(destination_id),
        ) else {
            return Ok(LegacySearchOutcome::UnknownEndpoint);
        };
        if start == destination {
            return Ok(LegacySearchOutcome::Routed {
                path_ids: vec![start_id.to_owned()],
                total_distance: 0.0,
            });
        }
        let mut best = vec![f64::INFINITY; self.nodes.len()];
        let mut previous = vec![None; self.nodes.len()];
        best[start] = 0.0;
        let mut queue = LegacyQueue::default();
        queue.push(Entry {
            node: start,
            cost: 0.0,
            priority: self.estimate(start, destination)?,
        });
        let mut work = 0;
        while let Some(current) = queue.pop() {
            work += 1;
            if work > MAX_SEARCH_WORK {
                return Err(LegacySearchError::WorkLimitExceeded);
            }
            if current.cost != best[current.node] {
                continue;
            }
            if current.node == destination {
                let mut path = vec![destination];
                let mut node = destination;
                while node != start {
                    node = previous[node].ok_or(LegacySearchError::InvalidGraph)?;
                    path.push(node);
                    if path.len() > self.nodes.len() {
                        return Err(LegacySearchError::InvalidGraph);
                    }
                }
                path.reverse();
                return Ok(LegacySearchOutcome::Routed {
                    path_ids: path
                        .into_iter()
                        .map(|index| self.nodes[index].id.clone())
                        .collect(),
                    total_distance: current.cost,
                });
            }
            for &(index, to) in &self.adjacency[current.node] {
                work += 1;
                if work > MAX_SEARCH_WORK {
                    return Err(LegacySearchError::WorkLimitExceeded);
                }
                let edge = &self.edges[index];
                if closed.contains(&index)
                    || (options.accessible_only && !edge.accessible)
                    || (!options.allow_restricted && edge.restricted)
                {
                    continue;
                }
                let candidate = current.cost + edge.distance;
                if !candidate.is_finite() || candidate <= current.cost {
                    return Err(LegacySearchError::UnrepresentableCost);
                }
                if candidate >= best[to] {
                    continue;
                }
                let priority = candidate + self.estimate(to, destination)?;
                if !priority.is_finite() {
                    return Err(LegacySearchError::UnrepresentableCost);
                }
                best[to] = candidate;
                previous[to] = Some(current.node);
                queue.push(Entry {
                    node: to,
                    cost: candidate,
                    priority,
                });
            }
        }
        Ok(LegacySearchOutcome::NoRoute)
    }
}
