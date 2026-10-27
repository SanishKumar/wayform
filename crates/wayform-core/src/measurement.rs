use std::collections::HashSet;
use std::fmt;

use crate::graph::compare_ids;
use crate::validation::MAX_SAFE_INTEGER;
use crate::{EvidenceError, EvidenceIssue, EvidenceSnapshot, Suitability};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MeasurementAttribute {
    ClearWidthMeters,
    SlopePercent,
    CrossSlopePercent,
    StepCount,
    Surface,
    DoorOperation,
}

impl MeasurementAttribute {
    pub fn name(self) -> &'static str {
        match self {
            Self::ClearWidthMeters => "clearWidthMeters",
            Self::SlopePercent => "slopePercent",
            Self::CrossSlopePercent => "crossSlopePercent",
            Self::StepCount => "stepCount",
            Self::Surface => "surface",
            Self::DoorOperation => "doorOperation",
        }
    }
}

#[derive(Clone, Debug)]
pub enum MeasurementRequirement {
    MinimumWidth(f64),
    Slope {
        max_uphill: Option<f64>,
        max_downhill: Option<f64>,
    },
    MaximumCrossSlope(f64),
    MaximumSteps(u64),
    Surfaces(Vec<String>),
    DoorOperations(Vec<String>),
}

impl MeasurementRequirement {
    pub fn attribute(&self) -> MeasurementAttribute {
        match self {
            Self::MinimumWidth(_) => MeasurementAttribute::ClearWidthMeters,
            Self::Slope { .. } => MeasurementAttribute::SlopePercent,
            Self::MaximumCrossSlope(_) => MeasurementAttribute::CrossSlopePercent,
            Self::MaximumSteps(_) => MeasurementAttribute::StepCount,
            Self::Surfaces(_) => MeasurementAttribute::Surface,
            Self::DoorOperations(_) => MeasurementAttribute::DoorOperation,
        }
    }

    pub(crate) fn valid(&self) -> bool {
        let nonnegative = |number: f64| number.is_finite() && number >= 0.0;
        match self {
            Self::MinimumWidth(value) | Self::MaximumCrossSlope(value) => nonnegative(*value),
            Self::MaximumSteps(value) => *value <= MAX_SAFE_INTEGER,
            Self::Slope {
                max_uphill,
                max_downhill,
            } => {
                (max_uphill.is_some() || max_downhill.is_some())
                    && max_uphill.is_none_or(nonnegative)
                    && max_downhill.is_none_or(nonnegative)
            }
            Self::Surfaces(values) | Self::DoorOperations(values) => {
                !values.is_empty()
                    && values.iter().all(|value| !value.is_empty())
                    && values.iter().collect::<HashSet<_>>().len() == values.len()
            }
        }
    }

    fn bounds(&self) -> (f64, f64) {
        match self {
            Self::MinimumWidth(value) => (*value, f64::INFINITY),
            Self::Slope {
                max_uphill,
                max_downhill,
            } => (
                -max_downhill.unwrap_or(f64::INFINITY),
                max_uphill.unwrap_or(f64::INFINITY),
            ),
            Self::MaximumCrossSlope(value) => (f64::NEG_INFINITY, *value),
            Self::MaximumSteps(value) => (f64::NEG_INFINITY, *value as f64),
            Self::Surfaces(_) | Self::DoorOperations(_) => {
                unreachable!("categorical requirements have no numeric bounds")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum MeasurementValue {
    Number(f64),
    Category(String),
    Unknown,
}

/// Observations must be grouped by directed edge and requested attribute by the
/// caller. The attribute is retained so an accidental mixed-attribute input fails.
#[derive(Clone, Debug)]
pub struct MeasurementObservation {
    pub id: String,
    pub attribute: MeasurementAttribute,
    pub value: MeasurementValue,
    pub uncertainty: Option<f64>,
    pub unknown_reason: Option<String>,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementReasonCode {
    Evidence(EvidenceIssue),
    MissingMeasurement,
    UnknownMeasurement,
    UnknownMeasurementError,
    ContradictoryEvidence,
    HardRequirementFailed,
    MeasurementStraddlesLimit,
}

impl MeasurementReasonCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::Evidence(issue) => issue.code(),
            Self::MissingMeasurement => "missing-measurement",
            Self::UnknownMeasurement => "unknown-measurement",
            Self::UnknownMeasurementError => "unknown-measurement-error",
            Self::ContradictoryEvidence => "contradictory-evidence",
            Self::HardRequirementFailed => "hard-requirement-failed",
            Self::MeasurementStraddlesLimit => "measurement-straddles-limit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementReason {
    pub code: MeasurementReasonCode,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementAssessment {
    pub status: Suitability,
    pub reasons: Vec<MeasurementReason>,
    pub evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeasurementError {
    InvalidRequirement,
    InvalidObservation(String),
    DuplicateObservation(String),
    Evidence(EvidenceError),
}

impl fmt::Display for MeasurementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequirement => write!(formatter, "invalid measurement requirement"),
            Self::InvalidObservation(id) => {
                write!(formatter, "invalid or mismatched observation: {id}")
            }
            Self::DuplicateObservation(id) => write!(formatter, "duplicate observation ID: {id}"),
            Self::Evidence(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MeasurementError {}

pub(crate) fn valid_observation(
    observation: &MeasurementObservation,
    attribute: MeasurementAttribute,
) -> bool {
    if observation.id.is_empty()
        || observation.attribute != attribute
        || observation
            .uncertainty
            .is_some_and(|error| !error.is_finite() || error < 0.0)
    {
        return false;
    }
    if matches!(observation.value, MeasurementValue::Unknown) {
        return observation.uncertainty.is_none()
            && observation
                .unknown_reason
                .as_ref()
                .is_some_and(|reason| !reason.is_empty());
    }
    if observation.unknown_reason.is_some() {
        return false;
    }
    match (&observation.value, attribute) {
        (
            MeasurementValue::Category(value),
            MeasurementAttribute::Surface | MeasurementAttribute::DoorOperation,
        ) => !value.is_empty() && observation.uncertainty.is_none(),
        (
            MeasurementValue::Number(value),
            MeasurementAttribute::ClearWidthMeters
            | MeasurementAttribute::SlopePercent
            | MeasurementAttribute::CrossSlopePercent
            | MeasurementAttribute::StepCount,
        ) => {
            let safe_integer =
                |number: f64| number.fract() == 0.0 && number <= MAX_SAFE_INTEGER as f64;
            value.is_finite()
                && (attribute == MeasurementAttribute::SlopePercent || *value >= 0.0)
                && (attribute != MeasurementAttribute::StepCount
                    || (safe_integer(*value) && observation.uncertainty.is_none_or(safe_integer)))
        }
        _ => false,
    }
}

fn sorted_ids(observations: &[&MeasurementObservation]) -> Vec<String> {
    let mut ids: Vec<_> = observations
        .iter()
        .map(|observation| observation.evidence_id.clone())
        .collect();
    ids.sort_by(|left, right| compare_ids(left, right));
    ids.dedup();
    ids
}

// The draft's inclusive threshold comparison absorbs arithmetic noise only.
// Physical uncertainty remains in the observed interval.
fn below(left: f64, right: f64) -> bool {
    left < right
        && (!left.is_finite()
            || !right.is_finite()
            || right - left > 8.0 * f64::EPSILON * 1.0_f64.max(left.abs()).max(right.abs()))
}

/// Assess one physical requirement using an immutable evidence snapshot. This
/// component does not apply access rules, closures, stairs rules or transforms.
pub fn assess_measurement(
    requirement: &MeasurementRequirement,
    observations: &[MeasurementObservation],
    evidence: &EvidenceSnapshot,
) -> Result<MeasurementAssessment, MeasurementError> {
    if !requirement.valid() {
        return Err(MeasurementError::InvalidRequirement);
    }
    let mut seen = HashSet::new();
    let mut usable = Vec::new();
    let mut unavailable = Vec::new();
    for observation in observations {
        if !valid_observation(observation, requirement.attribute()) {
            return Err(MeasurementError::InvalidObservation(observation.id.clone()));
        }
        if !seen.insert(&observation.id) {
            return Err(MeasurementError::DuplicateObservation(
                observation.id.clone(),
            ));
        }
        let issue = evidence
            .issue(&observation.evidence_id)
            .map_err(MeasurementError::Evidence)?;
        if let Some(issue) = issue {
            unavailable.push(MeasurementReasonCode::Evidence(issue));
        } else if matches!(observation.value, MeasurementValue::Unknown) {
            unavailable.push(MeasurementReasonCode::UnknownMeasurement);
        } else if matches!(observation.value, MeasurementValue::Number(_))
            && observation.uncertainty.is_none()
        {
            unavailable.push(MeasurementReasonCode::UnknownMeasurementError);
        } else {
            usable.push(observation);
        }
    }
    if usable.is_empty() {
        if observations.is_empty() {
            unavailable.push(MeasurementReasonCode::MissingMeasurement);
        }
        unavailable.sort_by_key(|code| code.code());
        unavailable.dedup();
        let evidence_ids = sorted_ids(&observations.iter().collect::<Vec<_>>());
        return Ok(MeasurementAssessment {
            status: Suitability::Unknown,
            reasons: unavailable
                .into_iter()
                .map(|code| MeasurementReason {
                    code,
                    evidence_ids: evidence_ids.clone(),
                })
                .collect(),
            evidence_ids,
        });
    }
    let reason = match requirement {
        MeasurementRequirement::Surfaces(allowed)
        | MeasurementRequirement::DoorOperations(allowed) => {
            let values: HashSet<_> = usable
                .iter()
                .filter_map(|observation| match &observation.value {
                    MeasurementValue::Category(value) => Some(value),
                    _ => None,
                })
                .collect();
            if values.len() != 1 {
                Some(MeasurementReasonCode::ContradictoryEvidence)
            } else if !values.iter().all(|value| allowed.contains(value)) {
                Some(MeasurementReasonCode::HardRequirementFailed)
            } else {
                None
            }
        }
        _ => {
            let mut low = f64::INFINITY;
            let mut high = f64::NEG_INFINITY;
            let mut shared_low = f64::NEG_INFINITY;
            let mut shared_high = f64::INFINITY;
            for observation in &usable {
                if let MeasurementValue::Number(value) = observation.value {
                    let error = observation
                        .uncertainty
                        .expect("usable numeric observation has uncertainty");
                    low = low.min(value - error);
                    high = high.max(value + error);
                    shared_low = shared_low.max(value - error);
                    shared_high = shared_high.min(value + error);
                }
            }
            let (minimum, maximum) = requirement.bounds();
            if below(shared_high, shared_low) {
                Some(MeasurementReasonCode::ContradictoryEvidence)
            } else if below(high, minimum) || below(maximum, low) {
                Some(MeasurementReasonCode::HardRequirementFailed)
            } else if below(low, minimum) || below(maximum, high) {
                Some(MeasurementReasonCode::MeasurementStraddlesLimit)
            } else {
                None
            }
        }
    };
    let evidence_ids = sorted_ids(&usable);
    Ok(MeasurementAssessment {
        status: match reason {
            None => Suitability::Suitable,
            Some(MeasurementReasonCode::HardRequirementFailed) => Suitability::Unsuitable,
            Some(_) => Suitability::Unknown,
        },
        reasons: reason
            .into_iter()
            .map(|code| MeasurementReason {
                code,
                evidence_ids: evidence_ids.clone(),
            })
            .collect(),
        evidence_ids,
    })
}
