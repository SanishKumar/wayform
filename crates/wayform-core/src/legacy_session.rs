//! Route ownership and freshness checks for the legacy navigation contract.
//! Routing and lease behavior derive from VoiceGIS commit
//! 305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f; original source and rights remain
//! under compat/voicegis/upstream/. This session API is a native integration.

use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use serde_json::Value;

use crate::{
    LegacyEdge, LegacyFreshnessStatus, LegacyGraph, LegacyNode, LegacyOperationalFreshness,
    LegacyOperationalLease, LegacyPolicyError, LegacyPolicyIndex, LegacyPolicyPackage,
    LegacyRouteOptions, LegacySearchError, LegacySearchOutcome, MAX_EDGES,
};

#[derive(Clone, Debug, PartialEq)]
pub enum LegacySessionError {
    Search(LegacySearchError),
    Policy(LegacyPolicyError),
    PackageInventoryMismatch,
    PolicyUnavailable(LegacyOperationalFreshness),
    NoRoute,
    UnknownEndpoint,
}

impl fmt::Display for LegacySessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Search(error) => fmt::Display::fmt(error, f),
            Self::Policy(error) => fmt::Display::fmt(error, f),
            Self::PackageInventoryMismatch => {
                f.write_str("policy and search edge inventories must match exactly")
            }
            Self::PolicyUnavailable(freshness) => write!(
                f,
                "legacy session policy unavailable: {}",
                freshness.reason.unwrap_or("unavailable")
            ),
            Self::NoRoute => f.write_str("no route satisfies the legacy request"),
            Self::UnknownEndpoint => f.write_str("unknown legacy route endpoint"),
        }
    }
}
impl std::error::Error for LegacySessionError {}

/// Immutable, matching projections from one host-verified venue package.
/// Matching IDs do not verify package bytes, geometry or source attribution.
#[derive(Debug)]
pub struct LegacySessionVenue {
    graph: LegacyGraph,
    policy: Arc<LegacyPolicyIndex>,
}

impl LegacySessionVenue {
    pub fn new(
        nodes: Vec<LegacyNode>,
        edges: Vec<LegacyEdge>,
        package: LegacyPolicyPackage,
    ) -> Result<Self, LegacySessionError> {
        if edges.len() > MAX_EDGES {
            return Err(LegacySessionError::Search(LegacySearchError::InvalidGraph));
        }
        if package.edges.len() > MAX_EDGES {
            return Err(LegacySessionError::Policy(
                LegacyPolicyError::InvalidPackage,
            ));
        }
        let ids: HashSet<_> = edges.iter().map(|edge| edge.id.as_str()).collect();
        if package.edges.len() != ids.len()
            || package
                .edges
                .iter()
                .any(|edge| !ids.contains(edge.id.as_str()))
        {
            return Err(LegacySessionError::PackageInventoryMismatch);
        }
        Ok(Self {
            graph: LegacyGraph::new(nodes, edges).map_err(LegacySessionError::Search)?,
            policy: Arc::new(LegacyPolicyIndex::new(package).map_err(LegacySessionError::Policy)?),
        })
    }
}

/// Closures come only from the session's resolved overlay, never this request.
#[derive(Clone, Debug)]
pub struct LegacySessionRequest {
    pub start_id: String,
    pub destination_id: String,
    pub accessible_only: bool,
    pub allow_restricted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacySessionInvalidation {
    PolicyChanged,
    PolicyRemoved,
    RouteContextChanged,
    Cancelled,
}

/// Ready is a view valid at the supplied clocks, not a durable authorization.
/// Hosts must clear queued/displayed guidance whenever a read is not Ready.
#[derive(Debug, PartialEq)]
pub enum LegacySessionRead<'a> {
    Ready {
        path_ids: &'a [String],
        total_distance: f64,
        overlay_id: String,
    },
    PolicyUnavailable(LegacyOperationalFreshness),
    Invalidated(LegacySessionInvalidation),
}

/// Owns one planned path and the exact lease used to choose its closures.
/// There is no route getter, reset, policy replacement or Clone implementation.
pub struct LegacyRouteSession {
    lease: LegacyOperationalLease,
    path_ids: Vec<String>,
    total_distance: f64,
    invalidated: Option<LegacySessionInvalidation>,
}

impl fmt::Debug for LegacyRouteSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Diagnostics must not expose an unchecked path through Debug output.
        f.debug_struct("LegacyRouteSession")
            .field("invalidated", &self.invalidated)
            .finish_non_exhaustive()
    }
}

impl LegacyRouteSession {
    /// Plans once but exposes no path. Sample both clocks again and call read()
    /// after planning, before presenting guidance. Failed planning makes no session.
    pub fn new(
        venue: Arc<LegacySessionVenue>,
        request: LegacySessionRequest,
        overlay: Value,
        wall_ms: f64,
        monotonic_ms: f64,
    ) -> Result<Self, LegacySessionError> {
        let mut lease =
            LegacyOperationalLease::new(venue.policy.clone(), overlay, wall_ms, monotonic_ms)
                .map_err(LegacySessionError::Policy)?;
        let freshness = lease
            .read(wall_ms, monotonic_ms)
            .map_err(LegacySessionError::Policy)?;
        if freshness.status != LegacyFreshnessStatus::Current {
            return Err(LegacySessionError::PolicyUnavailable(freshness));
        }
        let resolution = lease
            .resolution()
            .expect("a current lease has a valid resolution");
        let options = LegacyRouteOptions {
            accessible_only: request.accessible_only,
            allow_restricted: request.allow_restricted,
            closed_edge_ids: resolution.closed_edge_ids.clone(),
        };
        let (path_ids, total_distance) = match venue
            .graph
            .route(&request.start_id, &request.destination_id, &options)
            .map_err(LegacySessionError::Search)?
        {
            LegacySearchOutcome::Routed {
                path_ids,
                total_distance,
            } => (path_ids, total_distance),
            LegacySearchOutcome::NoRoute => return Err(LegacySessionError::NoRoute),
            LegacySearchOutcome::UnknownEndpoint => {
                return Err(LegacySessionError::UnknownEndpoint);
            }
        };
        Ok(Self {
            lease,
            path_ids,
            total_distance,
            invalidated: None,
        })
    }

    /// Read before each use and after suspension. Old views/copies do not grant
    /// future permission to display or speak the route.
    pub fn read(
        &mut self,
        wall_ms: f64,
        monotonic_ms: f64,
    ) -> Result<LegacySessionRead<'_>, LegacySessionError> {
        if let Some(reason) = self.invalidated {
            return Ok(LegacySessionRead::Invalidated(reason));
        }
        let freshness = self
            .lease
            .read(wall_ms, monotonic_ms)
            .map_err(LegacySessionError::Policy)?;
        if freshness.status != LegacyFreshnessStatus::Current {
            return Ok(LegacySessionRead::PolicyUnavailable(freshness));
        }
        Ok(LegacySessionRead::Ready {
            path_ids: &self.path_ids,
            total_distance: self.total_distance,
            overlay_id: freshness.overlay_id,
        })
    }

    /// Permanently withdraw this route. The first reason is retained. A policy,
    /// package, profile or location change requires an explicitly planned new session.
    pub fn invalidate(&mut self, reason: LegacySessionInvalidation) {
        self.invalidated.get_or_insert(reason);
        self.path_ids.clear();
    }
}
