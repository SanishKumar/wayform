mod identity;
mod runtime;
mod validation;

use crate::{
    IdentityError, JsonError, LegacyEdge, LegacyFloor, LegacyGraph, LegacyNode, LegacyPolicyEdge,
    LegacyPolicyPackage, LegacyRouteOptions, LegacySearchError, LegacySearchOutcome,
    LegacySessionError, LegacySessionVenue, parse_bounded_json,
};
use serde_json::Value;
use std::fmt;
use std::sync::Arc;

pub use identity::legacy_package_content_hash;
pub use runtime::{LegacyPoi, LegacyRuntime, LegacyRuntimeEdge, LegacyRuntimeNode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyPackageError {
    Json(JsonError),
    Identity(IdentityError),
    Invalid { path: String, code: &'static str },
    HashMismatch,
    Search(LegacySearchError),
}
impl fmt::Display for LegacyPackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => error.fmt(f),
            Self::Identity(error) => error.fmt(f),
            Self::Invalid { path, code } => write!(f, "invalid legacy package at {path}: {code}"),
            Self::HashMismatch => f.write_str("legacy package content hash mismatch"),
            Self::Search(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for LegacyPackageError {}

/// Owns a bounded, shape-checked and content-verified package. Integrity does
/// not authenticate the publisher or turn authored flags into surveyed facts.
#[derive(Debug)]
pub struct LegacyVenuePackage {
    value: Value,
    nodes: Vec<LegacyNode>,
    edges: Vec<LegacyEdge>,
    graph: LegacyGraph,
    runtime: LegacyRuntime,
}

impl LegacyVenuePackage {
    pub fn from_json(bytes: &[u8]) -> Result<Self, LegacyPackageError> {
        let value = parse_bounded_json(bytes).map_err(LegacyPackageError::Json)?;
        validation::validate(&value)?;
        let actual = legacy_package_content_hash(&value).map_err(LegacyPackageError::Identity)?;
        if value["manifest"]["contentHash"].as_str() != Some(&actual) {
            return Err(LegacyPackageError::HashMismatch);
        }
        let nodes: Vec<_> = value["routing"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| LegacyNode {
                id: node["id"].as_str().unwrap().into(),
                x: node["position"][0].as_f64().unwrap(),
                y: node["position"][1].as_f64().unwrap(),
                floor: LegacyFloor::Text(node["floorId"].as_str().unwrap().into()),
            })
            .collect();
        let edges: Vec<_> = value["routing"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| LegacyEdge {
                id: edge["id"].as_str().unwrap().into(),
                from: edge["from"].as_str().unwrap().into(),
                to: edge["to"].as_str().unwrap().into(),
                distance: edge["distanceMeters"].as_f64().unwrap(),
                accessible: edge["accessible"].as_bool().unwrap(),
                restricted: edge["restricted"].as_bool().unwrap(),
            })
            .collect();
        let graph =
            LegacyGraph::new(nodes.clone(), edges.clone()).map_err(LegacyPackageError::Search)?;
        let runtime = LegacyRuntime::from_validated(&value);
        Ok(Self {
            value,
            nodes,
            edges,
            graph,
            runtime,
        })
    }

    pub fn document(&self) -> &Value {
        &self.value
    }
    pub fn runtime(&self) -> &LegacyRuntime {
        &self.runtime
    }
    pub fn building_id(&self) -> &str {
        self.value["building"]["id"].as_str().unwrap()
    }
    pub fn content_hash(&self) -> &str {
        self.value["manifest"]["contentHash"].as_str().unwrap()
    }

    /// Consume a verified package when only its immutable search graph is needed.
    pub fn into_search_graph(self) -> LegacyGraph {
        self.graph
    }

    pub fn route(
        &self,
        start: &str,
        destination: &str,
        options: &LegacyRouteOptions,
    ) -> Result<LegacySearchOutcome, LegacySearchError> {
        self.graph.route(start, destination, options)
    }

    pub fn policy_package(&self) -> LegacyPolicyPackage {
        LegacyPolicyPackage {
            building_id: self.building_id().into(),
            package_hash: self.content_hash().into(),
            edges: self.value["routing"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|edge| LegacyPolicyEdge {
                    id: edge["id"].as_str().unwrap().into(),
                    source_id: edge["sourceId"].as_str().unwrap().into(),
                })
                .collect(),
        }
    }

    pub fn session_venue(&self) -> Result<Arc<LegacySessionVenue>, LegacySessionError> {
        LegacySessionVenue::new(
            self.nodes.clone(),
            self.edges.clone(),
            self.policy_package(),
        )
        .map(Arc::new)
    }
}
