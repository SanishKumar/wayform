use std::collections::{HashMap, HashSet};
use std::fmt;

// The draft contract permits four-digit UTC years, including year zero.
const MIN_TIMESTAMP_MS: i64 = -62_167_219_200_000;
const MAX_TIMESTAMP_MS: i64 = 253_402_300_799_999;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EvidenceSource {
    FieldSurvey,
    Operator,
    ImportedMap,
    Inference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewStatus {
    Reviewed,
    Pending,
    Rejected,
}

/// Parsed evidence metadata. The host must validate the original timestamp syntax
/// before converting it to Unix milliseconds; this type does not parse JSON.
#[derive(Clone, Debug)]
pub struct EvidenceRecord {
    pub id: String,
    pub source: EvidenceSource,
    pub actor_id: String,
    pub method: String,
    pub observed_at_ms: i64,
    pub valid_until_ms: Option<i64>,
    pub review_status: ReviewStatus,
    pub reviewer_id: Option<String>,
    pub reviewed_at_ms: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct EvidencePolicy {
    pub accepted_sources: Vec<EvidenceSource>,
    pub require_reviewed: bool,
    pub max_age_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceIssue {
    UnacceptedSource,
    Unreviewed,
    Future,
    Stale,
}

impl EvidenceIssue {
    pub fn code(self) -> &'static str {
        match self {
            Self::UnacceptedSource => "unaccepted-evidence-source",
            Self::Unreviewed => "unreviewed-evidence",
            Self::Future => "future-evidence",
            Self::Stale => "stale-evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceError {
    InvalidPolicy,
    InvalidEvaluationTime,
    InvalidRecord(String),
    DuplicateId(String),
    UnknownId(String),
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(formatter, "invalid evidence policy"),
            Self::InvalidEvaluationTime => write!(
                formatter,
                "evaluation time is outside the draft contract's UTC range"
            ),
            Self::InvalidRecord(id) => write!(formatter, "invalid evidence metadata: {id}"),
            Self::DuplicateId(id) => write!(formatter, "duplicate evidence ID: {id}"),
            Self::UnknownId(id) => write!(formatter, "unknown evidence ID: {id}"),
        }
    }
}

impl std::error::Error for EvidenceError {}

fn valid_time(value: i64) -> bool {
    (MIN_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&value)
}

fn valid_record(record: &EvidenceRecord) -> bool {
    !record.id.is_empty()
        && !record.actor_id.is_empty()
        && !record.method.is_empty()
        && valid_time(record.observed_at_ms)
        && record
            .valid_until_ms
            .is_none_or(|until| valid_time(until) && until > record.observed_at_ms)
        && record.reviewer_id.as_ref().is_none_or(|id| !id.is_empty())
        && record
            .reviewed_at_ms
            .is_none_or(|reviewed| valid_time(reviewed) && reviewed >= record.observed_at_ms)
        && (record.review_status != ReviewStatus::Reviewed
            || (record.reviewer_id.is_some() && record.reviewed_at_ms.is_some()))
}

fn evaluate(record: &EvidenceRecord, policy: &EvidencePolicy, now: i64) -> Option<EvidenceIssue> {
    if !policy.accepted_sources.contains(&record.source) {
        Some(EvidenceIssue::UnacceptedSource)
    } else if record.review_status == ReviewStatus::Rejected
        || (policy.require_reviewed && record.review_status != ReviewStatus::Reviewed)
    {
        Some(EvidenceIssue::Unreviewed)
    } else if record.observed_at_ms > now
        || record.reviewed_at_ms.is_some_and(|reviewed| reviewed > now)
    {
        Some(EvidenceIssue::Future)
    } else if (now - record.observed_at_ms) as f64 / 1000.0 > policy.max_age_seconds
        || record.valid_until_ms.is_some_and(|until| now >= until)
    {
        Some(EvidenceIssue::Stale)
    } else {
        None
    }
}

/// Immutable results for one explicit policy/time snapshot. Construction checks
/// all records, including unaccepted evidence, before making any result available.
/// Acceptance is a policy decision, not authentication of an actor or a survey.
#[derive(Clone, Debug)]
pub struct EvidenceSnapshot {
    issues: HashMap<String, Option<EvidenceIssue>>,
}

impl EvidenceSnapshot {
    pub fn new(
        records: &[EvidenceRecord],
        policy: &EvidencePolicy,
        evaluated_at_ms: i64,
    ) -> Result<Self, EvidenceError> {
        if !valid_time(evaluated_at_ms) {
            return Err(EvidenceError::InvalidEvaluationTime);
        }
        if policy.accepted_sources.is_empty()
            || policy.accepted_sources.iter().collect::<HashSet<_>>().len()
                != policy.accepted_sources.len()
            || !policy.max_age_seconds.is_finite()
            || policy.max_age_seconds < 0.0
        {
            return Err(EvidenceError::InvalidPolicy);
        }
        let mut issues = HashMap::with_capacity(records.len());
        for record in records {
            if !valid_record(record) {
                return Err(EvidenceError::InvalidRecord(record.id.clone()));
            }
            if issues
                .insert(record.id.clone(), evaluate(record, policy, evaluated_at_ms))
                .is_some()
            {
                return Err(EvidenceError::DuplicateId(record.id.clone()));
            }
        }
        Ok(Self { issues })
    }

    /// An unknown reference is invalid input, never an accepted observation.
    pub fn issue(&self, evidence_id: &str) -> Result<Option<EvidenceIssue>, EvidenceError> {
        self.issues
            .get(evidence_id)
            .copied()
            .ok_or_else(|| EvidenceError::UnknownId(evidence_id.to_owned()))
    }
}
