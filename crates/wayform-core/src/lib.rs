//! Native graph primitives. Physical eligibility is evaluated separately from topology.

mod graph;

pub use graph::{Edge, Graph, GraphError, MAX_EDGES, MAX_NODES};
