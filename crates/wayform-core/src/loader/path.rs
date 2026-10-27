use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::Serialize;

use super::{MOBILITY_VERSION, MobilitySnapshot};
use crate::graph::compare_ids;
use crate::{EdgeAssessment, EligibilityError, MeasurementAttribute, Suitability};

pub const MAX_PATH_EDGES: usize = 100_000;
const MAX_OUTPUT_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_OUTPUT_ITEMS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AssessmentStatus {
    Suitable,
    Unsuitable,
    Unknown,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathReason {
    pub code: String,
    pub edge_id: String,
    pub attribute: Option<MeasurementAttribute>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRevision {
    pub requirement_id: String,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathReceipt {
    pub schema_version: String,
    pub bundle_id: String,
    pub bundle_revision: String,
    pub bundle_digest: Option<String>,
    pub profile_digest: Option<String>,
    pub context_digest: Option<String>,
    pub profile_id: String,
    pub evaluated_at: String,
    pub edge_ids: Vec<String>,
    pub policy_revisions: Vec<PolicyRevision>,
    pub distance_meters: Option<f64>,
    pub preference_cost_meters: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathAssessment {
    pub status: AssessmentStatus,
    pub reasons: Vec<PathReason>,
    pub evidence_ids: Vec<String>,
    pub receipt: PathReceipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathError {
    ResourceLimit,
    Eligibility(EligibilityError),
}
impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceLimit => f.write_str("path assessment exceeds output or step limits"),
            Self::Eligibility(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for PathError {}

fn invalid(code: &str, receipt: PathReceipt) -> PathAssessment {
    PathAssessment {
        status: AssessmentStatus::Invalid,
        reasons: vec![PathReason {
            code: code.to_owned(),
            edge_id: String::new(),
            attribute: None,
            evidence_ids: Vec::new(),
        }],
        evidence_ids: Vec::new(),
        receipt,
    }
}

impl MobilitySnapshot {
    /// Assess the supplied directed sequence and issue a receipt for this exact
    /// immutable snapshot. A supplied unsuitable/unknown path is not a route recommendation.
    pub fn assess_path(
        &self,
        start_id: &str,
        destination_id: &str,
        edge_ids: &[String],
    ) -> Result<PathAssessment, PathError> {
        if edge_ids.len() > MAX_PATH_EDGES {
            return Err(PathError::ResourceLimit);
        }
        let mut text_bytes = edge_ids.iter().map(String::len).sum::<usize>();
        if text_bytes > MAX_OUTPUT_TEXT_BYTES {
            return Err(PathError::ResourceLimit);
        }
        let mut receipt = PathReceipt {
            schema_version: MOBILITY_VERSION.to_owned(),
            bundle_id: self.policy_bundle.identity.id.clone(),
            bundle_revision: self.policy_bundle.identity.revision.clone(),
            bundle_digest: Some(self.digests.bundle_digest.clone()),
            profile_digest: Some(self.digests.profile_digest.clone()),
            context_digest: Some(self.digests.context_digest.clone()),
            profile_id: self.profile_id.clone(),
            evaluated_at: self.evaluated_at.clone(),
            edge_ids: edge_ids.to_vec(),
            policy_revisions: Vec::new(),
            distance_meters: None,
            preference_cost_meters: None,
        };
        if edge_ids.iter().any(String::is_empty) {
            receipt.edge_ids.clear();
            receipt.bundle_digest = None;
            receipt.profile_digest = None;
            receipt.context_digest = None;
            return Ok(invalid("invalid-contract", receipt));
        }
        if !self.graph.nodes.contains_key(start_id)
            || !self.graph.nodes.contains_key(destination_id)
        {
            return Ok(invalid("unknown-endpoint", receipt));
        }
        let mut current = start_id;
        let mut distance = 0.0;
        let mut cost = 0.0;
        for id in edge_ids {
            let Ok(index) = self
                .graph
                .edges
                .binary_search_by(|edge| compare_ids(&edge.id, id))
            else {
                return Ok(invalid("disconnected-or-reversed-edge", receipt));
            };
            let edge = &self.graph.edges[index];
            if edge.from != current {
                return Ok(invalid("disconnected-or-reversed-edge", receipt));
            }
            current = &edge.to;
            distance += edge.length_meters;
            cost += edge.length_meters
                + if self.avoid_kinds.contains(&self.facts[id].kind) {
                    self.penalty_meters
                } else {
                    0.0
                };
        }
        if current != destination_id {
            return Ok(invalid("destination-not-reached", receipt));
        }
        if !distance.is_finite() || !cost.is_finite() {
            return Ok(invalid("cost-overflow", receipt));
        }

        let mut status = AssessmentStatus::Suitable;
        let mut reasons = Vec::new();
        let mut evidence = HashSet::new();
        let mut cache: HashMap<&str, EdgeAssessment> = HashMap::new();
        let mut items = edge_ids.len();
        for id in edge_ids {
            if !cache.contains_key(id.as_str()) {
                cache.insert(id, self.assess_edge(id).map_err(PathError::Eligibility)?);
            }
            let assessment = &cache[id.as_str()];
            if assessment.status == Suitability::Unsuitable {
                status = AssessmentStatus::Unsuitable;
            } else if status == AssessmentStatus::Suitable
                && assessment.status == Suitability::Unknown
            {
                status = AssessmentStatus::Unknown;
            }
            // Bound repeated-edge expansion before cloning reason text and IDs.
            let ids = assessment
                .reasons
                .iter()
                .flat_map(|reason| &reason.evidence_ids)
                .chain(&assessment.evidence_ids);
            for value in ids {
                text_bytes += value.len();
                items += 1;
            }
            for reason in &assessment.reasons {
                text_bytes += id.len() + reason.code.code().len();
                items += 1;
            }
            if text_bytes > MAX_OUTPUT_TEXT_BYTES || items > MAX_OUTPUT_ITEMS {
                return Err(PathError::ResourceLimit);
            }
            reasons.extend(assessment.reasons.iter().map(|reason| PathReason {
                code: reason.code.code().to_owned(),
                edge_id: id.clone(),
                attribute: reason.attribute,
                evidence_ids: reason.evidence_ids.clone(),
            }));
            evidence.extend(assessment.evidence_ids.iter().cloned());
        }
        reasons.sort_by(|left, right| {
            let key = |reason: &PathReason| {
                format!(
                    "{}/{}/{}",
                    reason.edge_id,
                    reason.attribute.map_or("", MeasurementAttribute::name),
                    reason.code
                )
            };
            compare_ids(&key(left), &key(right))
        });
        let mut evidence_ids: Vec<_> = evidence.into_iter().collect();
        evidence_ids.sort_by(|left, right| compare_ids(left, right));
        let traversed: HashSet<_> = edge_ids.iter().collect();
        let used: HashSet<_> = self
            .policy_bundle
            .requirements
            .iter()
            .filter(|requirement| requirement.edge_ids.iter().any(|id| traversed.contains(id)))
            .map(|requirement| &requirement.id)
            .collect();
        receipt.policy_revisions = self
            .policy_context
            .policies
            .iter()
            .filter(|policy| used.contains(&policy.requirement_id))
            .map(|policy| PolicyRevision {
                requirement_id: policy.requirement_id.clone(),
                revision: policy.revision,
            })
            .collect();
        receipt
            .policy_revisions
            .sort_by(|left, right| compare_ids(&left.requirement_id, &right.requirement_id));
        receipt.distance_meters = Some(distance);
        receipt.preference_cost_meters = Some(cost);
        Ok(PathAssessment {
            status,
            reasons,
            evidence_ids,
            receipt,
        })
    }
}
