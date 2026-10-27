use std::collections::HashSet;
use std::fmt;

use crate::graph::compare_ids;
use crate::measurement::valid_observation;
use crate::{
    EntranceRegistration, EvidenceError, EvidenceIssue, EvidenceSnapshot, MeasurementAttribute,
    MeasurementError, MeasurementObservation, MeasurementReasonCode, MeasurementRequirement,
    PolicyError, PolicyIssue, PolicySnapshot, Suitability, assess_measurement,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Walkway,
    Door,
    Ramp,
    Stairs,
    Lift,
    Crossing,
    Entrance,
}

/// Typed per-edge facts extracted from a validated bundle by the host. Legacy
/// booleans retain their authored meaning; they never substitute for a survey.
#[derive(Clone, Debug)]
pub struct EdgeFacts {
    pub id: String,
    pub kind: EdgeKind,
    pub restricted: bool,
    pub legacy_accessible: Option<bool>,
    pub observations: Vec<MeasurementObservation>,
    pub crosses_frame: bool,
    pub registration: Option<EntranceRegistration>,
}

#[derive(Clone, Debug)]
pub struct EdgeRequirements {
    pub allow_restricted: bool,
    pub respect_legacy_exclusions: bool,
    pub measurements: Vec<MeasurementRequirement>,
    pub max_transform_error_meters: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeReasonCode {
    RestrictedAccess,
    LegacyExclusion,
    StairsExcluded,
    Measurement(MeasurementReasonCode),
    Policy(PolicyIssue),
    TransformEvidence(EvidenceIssue),
    TransformError,
}

impl EdgeReasonCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::RestrictedAccess => "restricted-access",
            Self::LegacyExclusion => "legacy-declared-inaccessible",
            Self::StairsExcluded => "stairs-excluded",
            Self::Measurement(reason) => reason.code(),
            Self::Policy(issue) => issue.code(),
            Self::TransformEvidence(EvidenceIssue::UnacceptedSource) => {
                "transform-unaccepted-evidence-source"
            }
            Self::TransformEvidence(EvidenceIssue::Unreviewed) => "transform-unreviewed-evidence",
            Self::TransformEvidence(EvidenceIssue::Future) => "transform-future-evidence",
            Self::TransformEvidence(EvidenceIssue::Stale) => "transform-stale-evidence",
            Self::TransformError => "transform-error-exceeds-budget",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeReason {
    pub code: EdgeReasonCode,
    pub attribute: Option<MeasurementAttribute>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeAssessment {
    pub edge_id: String,
    pub status: Suitability,
    pub reasons: Vec<EdgeReason>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EligibilityError {
    InvalidRequirements,
    InvalidRegistration,
    MismatchedEvaluationTime,
    Evidence(EvidenceError),
    Measurement(MeasurementError),
    Policy(PolicyError),
}

impl fmt::Display for EligibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequirements => {
                write!(formatter, "invalid or duplicate edge requirements")
            }
            Self::InvalidRegistration => write!(
                formatter,
                "invalid entrance registration or frame transition"
            ),
            Self::MismatchedEvaluationTime => write!(
                formatter,
                "evidence and policy snapshots use different evaluation times"
            ),
            Self::Evidence(error) => error.fmt(formatter),
            Self::Measurement(error) => error.fmt(formatter),
            Self::Policy(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EligibilityError {}

impl EdgeAssessment {
    fn note(
        &mut self,
        status: Suitability,
        code: EdgeReasonCode,
        attribute: Option<MeasurementAttribute>,
        evidence_ids: Vec<String>,
    ) {
        if status == Suitability::Unsuitable
            || (self.status == Suitability::Suitable && status == Suitability::Unknown)
        {
            self.status = status;
        }
        self.evidence_ids.extend(evidence_ids.iter().cloned());
        self.reasons.push(EdgeReason {
            code,
            attribute,
            evidence_ids,
        });
    }
}

/// Combine all draft edge rules. Topology, original JSON shapes, cross-frame
/// references, source provenance and bundle hashing remain host validation tasks.
pub fn assess_edge(
    facts: &EdgeFacts,
    requirements: &EdgeRequirements,
    evidence: &EvidenceSnapshot,
    policies: &PolicySnapshot,
) -> Result<EdgeAssessment, EligibilityError> {
    if evidence.evaluated_at_ms() != policies.evaluated_at_ms() {
        return Err(EligibilityError::MismatchedEvaluationTime);
    }
    let policy = policies
        .assess_edge(&facts.id)
        .map_err(EligibilityError::Policy)?;
    let mut attributes = HashSet::new();
    if !requirements.max_transform_error_meters.is_finite()
        || requirements.max_transform_error_meters < 0.0
        || requirements
            .measurements
            .iter()
            .any(|requirement| !requirement.valid() || !attributes.insert(requirement.attribute()))
    {
        return Err(EligibilityError::InvalidRequirements);
    }
    let mut observations = HashSet::new();
    for observation in &facts.observations {
        if !valid_observation(observation, observation.attribute) {
            return Err(EligibilityError::Measurement(
                MeasurementError::InvalidObservation(observation.id.clone()),
            ));
        }
        if !observations.insert(&observation.id) {
            return Err(EligibilityError::Measurement(
                MeasurementError::DuplicateObservation(observation.id.clone()),
            ));
        }
        evidence
            .issue(&observation.evidence_id)
            .map_err(EligibilityError::Evidence)?;
    }
    if facts.crosses_frame {
        if facts.kind != EdgeKind::Entrance
            || facts
                .registration
                .as_ref()
                .is_none_or(|registration| !registration.valid())
        {
            return Err(EligibilityError::InvalidRegistration);
        }
    } else if facts.registration.is_some() {
        return Err(EligibilityError::InvalidRegistration);
    }

    let mut result = EdgeAssessment {
        edge_id: facts.id.clone(),
        status: Suitability::Suitable,
        reasons: Vec::new(),
        evidence_ids: Vec::new(),
    };
    if facts.restricted && !requirements.allow_restricted {
        result.note(
            Suitability::Unsuitable,
            EdgeReasonCode::RestrictedAccess,
            None,
            vec![],
        );
    }
    if facts.legacy_accessible == Some(false) && requirements.respect_legacy_exclusions {
        result.note(
            Suitability::Unsuitable,
            EdgeReasonCode::LegacyExclusion,
            None,
            vec![],
        );
    }
    if facts.kind == EdgeKind::Stairs
        && requirements
            .measurements
            .iter()
            .any(|requirement| matches!(requirement, MeasurementRequirement::MaximumSteps(0)))
    {
        result.note(
            Suitability::Unsuitable,
            EdgeReasonCode::StairsExcluded,
            Some(MeasurementAttribute::StepCount),
            vec![],
        );
    }
    for reason in &policy.reasons {
        result.note(
            if reason.issue == PolicyIssue::Closed {
                Suitability::Unsuitable
            } else {
                Suitability::Unknown
            },
            EdgeReasonCode::Policy(reason.issue),
            None,
            vec![],
        );
    }
    if let Some(registration) = &facts.registration {
        let issue = evidence
            .issue(&registration.evidence_id)
            .map_err(EligibilityError::Evidence)?;
        result.evidence_ids.push(registration.evidence_id.clone());
        if let Some(issue) = issue {
            result.note(
                Suitability::Unknown,
                EdgeReasonCode::TransformEvidence(issue),
                None,
                vec![registration.evidence_id.clone()],
            );
        }
        let error = registration.error_meters();
        if !error.is_finite() || error > requirements.max_transform_error_meters {
            result.note(
                Suitability::Unknown,
                EdgeReasonCode::TransformError,
                None,
                vec![registration.evidence_id.clone()],
            );
        }
    }
    for requirement in &requirements.measurements {
        if requirement.attribute() == MeasurementAttribute::DoorOperation
            && facts.kind != EdgeKind::Door
        {
            continue;
        }
        let matching: Vec<_> = facts
            .observations
            .iter()
            .filter(|observation| observation.attribute == requirement.attribute())
            .cloned()
            .collect();
        let assessment = assess_measurement(requirement, &matching, evidence)
            .map_err(EligibilityError::Measurement)?;
        result.evidence_ids.extend(assessment.evidence_ids);
        for reason in assessment.reasons {
            result.note(
                assessment.status,
                EdgeReasonCode::Measurement(reason.code),
                Some(requirement.attribute()),
                reason.evidence_ids,
            );
        }
    }
    result.reasons.sort_by(|left, right| {
        let key = |reason: &EdgeReason| {
            format!(
                "{}/{}",
                reason.attribute.map_or("", MeasurementAttribute::name),
                reason.code.code()
            )
        };
        key(left).cmp(&key(right))
    });
    result
        .evidence_ids
        .sort_by(|left, right| compare_ids(left, right));
    result.evidence_ids.dedup();
    Ok(result)
}
