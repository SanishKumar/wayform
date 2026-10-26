//! Native directed search over explicit eligibility snapshots.
//! Physical eligibility is evaluated separately from topology and search.

mod evidence;
mod graph;
mod search;

pub use evidence::{
    EvidenceError, EvidenceIssue, EvidencePolicy, EvidenceRecord, EvidenceSnapshot, EvidenceSource,
    ReviewStatus,
};
pub use graph::{Edge, Graph, GraphError, MAX_EDGES, MAX_NODES};
pub use search::{EdgeDecision, Path, SearchError, SearchOutcome, Suitability};
