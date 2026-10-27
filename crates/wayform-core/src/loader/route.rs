use std::collections::{HashSet, VecDeque};
use std::fmt;

use serde::Serialize;

use super::{AssessmentStatus, MobilitySnapshot, PathAssessment, PathError};
use crate::{EdgeDecision, EligibilityError, SearchError, SearchOutcome, Suitability};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteDiagnostic {
    pub suitable_reachable_nodes: usize,
    pub unknown_frontier_edges: usize,
    pub unsuitable_frontier_edges: usize,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MobilityRouteOutcome {
    Routed(Box<PathAssessment>),
    InsufficientEvidence(RouteDiagnostic),
    Infeasible(RouteDiagnostic),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobilityRouteError {
    Eligibility(EligibilityError),
    Search(SearchError),
    Path(PathError),
    InconsistentReceipt,
}
impl fmt::Display for MobilityRouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eligibility(error) => error.fmt(f),
            Self::Search(error) => error.fmt(f),
            Self::Path(error) => error.fmt(f),
            Self::InconsistentReceipt => f.write_str("selected route and receipt disagree"),
        }
    }
}
impl std::error::Error for MobilityRouteError {}

impl MobilitySnapshot {
    /// Evaluate every directed edge, select a suitable path and issue a receipt
    /// against the same immutable inputs. Failure diagnostics never include a path.
    pub fn route(
        &self,
        start_id: &str,
        destination_id: &str,
    ) -> Result<MobilityRouteOutcome, MobilityRouteError> {
        let decisions = self.decisions().map_err(MobilityRouteError::Eligibility)?;
        let outcome = self
            .graph
            .route(start_id, destination_id, &decisions)
            .map_err(MobilityRouteError::Search)?;
        match outcome {
            SearchOutcome::Routed(path) => {
                let assessment = self
                    .assess_path(start_id, destination_id, &path.edge_ids)
                    .map_err(MobilityRouteError::Path)?;
                if assessment.status != AssessmentStatus::Suitable
                    || assessment.receipt.distance_meters != Some(path.distance_meters)
                    || assessment.receipt.preference_cost_meters != Some(path.cost_meters)
                {
                    return Err(MobilityRouteError::InconsistentReceipt);
                }
                Ok(MobilityRouteOutcome::Routed(Box::new(assessment)))
            }
            SearchOutcome::InsufficientEvidence => Ok(MobilityRouteOutcome::InsufficientEvidence(
                self.route_diagnostic(start_id, &decisions)?,
            )),
            SearchOutcome::Infeasible => Ok(MobilityRouteOutcome::Infeasible(
                self.route_diagnostic(start_id, &decisions)?,
            )),
        }
    }

    // Decisions come from this snapshot in graph edge order. Describe only edges
    // leaving the region reachable through suitable edges, without a candidate path.
    fn route_diagnostic(
        &self,
        start_id: &str,
        decisions: &[EdgeDecision],
    ) -> Result<RouteDiagnostic, MobilityRouteError> {
        let mut reachable = vec![false; self.graph.node_count()];
        let start = self.graph.nodes[start_id];
        reachable[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(node) = queue.pop_front() {
            for &index in &self.graph.outgoing[node] {
                if decisions[index].suitability != Suitability::Suitable {
                    continue;
                }
                let target = self.graph.nodes[&self.graph.edges[index].to];
                if !reachable[target] {
                    reachable[target] = true;
                    queue.push_back(target);
                }
            }
        }
        let mut diagnostic = RouteDiagnostic {
            suitable_reachable_nodes: reachable.iter().filter(|&&value| value).count(),
            unknown_frontier_edges: 0,
            unsuitable_frontier_edges: 0,
            reason_codes: Vec::new(),
        };
        let mut codes = HashSet::new();
        for (index, edge) in self.graph.edges.iter().enumerate() {
            if !reachable[self.graph.nodes[&edge.from]] || reachable[self.graph.nodes[&edge.to]] {
                continue;
            }
            match decisions[index].suitability {
                Suitability::Suitable => continue,
                Suitability::Unknown => diagnostic.unknown_frontier_edges += 1,
                Suitability::Unsuitable => diagnostic.unsuitable_frontier_edges += 1,
            }
            for reason in self
                .assess_edge(&edge.id)
                .map_err(MobilityRouteError::Eligibility)?
                .reasons
            {
                codes.insert(reason.code.code());
            }
        }
        diagnostic.reason_codes = codes.into_iter().map(str::to_owned).collect();
        diagnostic.reason_codes.sort();
        Ok(diagnostic)
    }
}
