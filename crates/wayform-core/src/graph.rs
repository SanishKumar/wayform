use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

pub const MAX_NODES: usize = 100_000;
pub const MAX_EDGES: usize = 500_000;

/// One directed arc. Reverse travel requires a separate arc with its own identity.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub length_meters: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    EmptyGraph,
    TooManyNodes,
    TooManyEdges,
    EmptyNodeId,
    DuplicateNode(String),
    EmptyEdgeId,
    DuplicateEdge(String),
    UnknownEndpoint { edge_id: String, node_id: String },
    SelfLoop(String),
    InvalidLength(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(formatter, "a graph requires at least one node"),
            Self::TooManyNodes => write!(formatter, "graph exceeds {MAX_NODES} nodes"),
            Self::TooManyEdges => write!(formatter, "graph exceeds {MAX_EDGES} edges"),
            Self::EmptyNodeId => write!(formatter, "node ID must not be empty"),
            Self::DuplicateNode(id) => write!(formatter, "duplicate node ID: {id}"),
            Self::EmptyEdgeId => write!(formatter, "edge ID must not be empty"),
            Self::DuplicateEdge(id) => write!(formatter, "duplicate edge ID: {id}"),
            Self::UnknownEndpoint { edge_id, node_id } => {
                write!(
                    formatter,
                    "edge {edge_id} references unknown node {node_id}"
                )
            }
            Self::SelfLoop(id) => write!(formatter, "edge {id} must connect distinct nodes"),
            Self::InvalidLength(id) => {
                write!(formatter, "edge {id} needs a positive finite length")
            }
        }
    }
}

impl std::error::Error for GraphError {}

/// Immutable topology, indexed only after all input references have been validated.
///
/// Edges are sorted by UTF-16 lexical ID order to match the TypeScript contract.
/// Parallel arcs are retained; no endpoint-pair map is used.
#[derive(Clone, Debug)]
pub struct Graph {
    pub(crate) nodes: HashMap<String, usize>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) outgoing: Vec<Vec<usize>>,
}

pub(crate) fn compare_ids(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

impl Graph {
    pub fn new(mut nodes: Vec<String>, mut edges: Vec<Edge>) -> Result<Self, GraphError> {
        if nodes.is_empty() {
            return Err(GraphError::EmptyGraph);
        }
        if nodes.len() > MAX_NODES {
            return Err(GraphError::TooManyNodes);
        }
        if edges.len() > MAX_EDGES {
            return Err(GraphError::TooManyEdges);
        }
        nodes.sort_by(|left, right| compare_ids(left, right));
        let mut index = HashMap::with_capacity(nodes.len());
        for (position, id) in nodes.into_iter().enumerate() {
            if id.is_empty() {
                return Err(GraphError::EmptyNodeId);
            }
            if index.insert(id.clone(), position).is_some() {
                return Err(GraphError::DuplicateNode(id));
            }
        }
        edges.sort_by(|left, right| compare_ids(&left.id, &right.id));
        let mut outgoing = vec![Vec::new(); index.len()];
        for (position, edge) in edges.iter().enumerate() {
            if edge.id.is_empty() {
                return Err(GraphError::EmptyEdgeId);
            }
            if position > 0 && edges[position - 1].id == edge.id {
                return Err(GraphError::DuplicateEdge(edge.id.clone()));
            }
            if !edge.length_meters.is_finite() || edge.length_meters <= 0.0 {
                return Err(GraphError::InvalidLength(edge.id.clone()));
            }
            let endpoint = |id: &str| {
                index
                    .get(id)
                    .copied()
                    .ok_or_else(|| GraphError::UnknownEndpoint {
                        edge_id: edge.id.clone(),
                        node_id: id.to_owned(),
                    })
            };
            let from = endpoint(&edge.from)?;
            let to = endpoint(&edge.to)?;
            if from == to {
                return Err(GraphError::SelfLoop(edge.id.clone()));
            }
            outgoing[from].push(position);
        }
        Ok(Self {
            nodes: index,
            edges,
            outgoing,
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edges(&self) -> impl ExactSizeIterator<Item = &Edge> {
        self.edges.iter()
    }

    pub fn outgoing_edges(&self, node_id: &str) -> Option<impl Iterator<Item = &Edge>> {
        self.nodes
            .get(node_id)
            .map(|&node| self.outgoing[node].iter().map(|&edge| &self.edges[edge]))
    }

    pub fn destination(&self, edge_id: &str) -> Option<&str> {
        self.edges
            .binary_search_by(|edge| compare_ids(&edge.id, edge_id))
            .ok()
            .map(|position| self.edges[position].to.as_str())
    }
}
