use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::Suitability;
use crate::graph::compare_ids;
use crate::validation::{MAX_SAFE_INTEGER, valid_digest, valid_time};

/// Identity supplied by a host that has validated and hashed the full bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleIdentity {
    pub id: String,
    pub revision: String,
    pub digest: String,
}

#[derive(Clone, Debug)]
pub struct PolicyRequirement {
    pub id: String,
    pub edge_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PolicyBundle {
    pub identity: BundleIdentity,
    pub edge_ids: Vec<String>,
    pub requirements: Vec<PolicyRequirement>,
}

#[derive(Clone, Debug)]
pub struct OperationalPolicy {
    pub requirement_id: String,
    pub bundle: BundleIdentity,
    pub revision: u64,
    pub issuer_id: String,
    pub valid_from_ms: i64,
    pub valid_until_ms: i64,
    pub closed_edge_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RevisionFloor {
    pub requirement_id: String,
    pub revision: u64,
}

/// Policies and revision floors must come from authenticated host storage.
/// These declarations do not verify signatures or persist rollback protection.
#[derive(Clone, Debug)]
pub struct PolicyContext {
    pub evaluated_at_ms: i64,
    pub trusted_issuer_ids: Vec<String>,
    pub minimum_revisions: Vec<RevisionFloor>,
    pub policies: Vec<OperationalPolicy>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyIssue {
    Missing,
    BundleMismatch,
    UntrustedIssuer,
    UntrustedRevision,
    NotCurrent,
    Closed,
}

impl PolicyIssue {
    pub fn code(self) -> &'static str {
        match self {
            Self::Missing => "missing-operational-policy",
            Self::BundleMismatch => "policy-bundle-mismatch",
            Self::UntrustedIssuer => "untrusted-policy-issuer",
            Self::UntrustedRevision => "untrusted-policy-revision",
            Self::NotCurrent => "policy-not-current",
            Self::Closed => "operationally-closed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyReason {
    pub requirement_id: String,
    pub issue: PolicyIssue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyAssessment {
    pub status: Suitability,
    pub reasons: Vec<PolicyReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    InvalidBundle,
    InvalidContext,
    InvalidRequirement(String),
    InvalidPolicy(String),
    InvalidRevisionFloor(String),
    UnknownEdge(String),
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBundle => write!(formatter, "invalid bundle identity or edge inventory"),
            Self::InvalidContext => {
                write!(formatter, "invalid policy evaluation time or issuer list")
            }
            Self::InvalidRequirement(id) => {
                write!(formatter, "invalid or duplicate policy requirement: {id}")
            }
            Self::InvalidPolicy(id) => {
                write!(formatter, "invalid or duplicate operational policy: {id}")
            }
            Self::InvalidRevisionFloor(id) => {
                write!(formatter, "invalid or duplicate revision floor: {id}")
            }
            Self::UnknownEdge(id) => write!(formatter, "unknown policy edge: {id}"),
        }
    }
}

impl std::error::Error for PolicyError {}

fn valid_identity(identity: &BundleIdentity) -> bool {
    !identity.id.is_empty() && !identity.revision.is_empty() && valid_digest(&identity.digest)
}

fn unique_ids(ids: &[String]) -> bool {
    ids.iter().all(|id| !id.is_empty()) && ids.iter().collect::<HashSet<_>>().len() == ids.len()
}

/// Immutable operational decisions for all declared edges at one time.
#[derive(Clone, Debug)]
pub struct PolicySnapshot {
    evaluated_at_ms: i64,
    assessments: HashMap<String, PolicyAssessment>,
}

impl PolicySnapshot {
    pub fn new(bundle: &PolicyBundle, context: &PolicyContext) -> Result<Self, PolicyError> {
        if !valid_identity(&bundle.identity) || !unique_ids(&bundle.edge_ids) {
            return Err(PolicyError::InvalidBundle);
        }
        if !valid_time(context.evaluated_at_ms) || !unique_ids(&context.trusted_issuer_ids) {
            return Err(PolicyError::InvalidContext);
        }
        let edges: HashSet<_> = bundle.edge_ids.iter().collect();
        let mut requirements = HashMap::new();
        for requirement in &bundle.requirements {
            if requirement.id.is_empty()
                || requirement.edge_ids.is_empty()
                || !unique_ids(&requirement.edge_ids)
                || requirement.edge_ids.iter().any(|id| !edges.contains(id))
                || requirements.insert(&requirement.id, requirement).is_some()
            {
                return Err(PolicyError::InvalidRequirement(requirement.id.clone()));
            }
        }
        let mut floors = HashMap::new();
        for floor in &context.minimum_revisions {
            if floor.revision > MAX_SAFE_INTEGER
                || !requirements.contains_key(&floor.requirement_id)
                || floors
                    .insert(&floor.requirement_id, floor.revision)
                    .is_some()
            {
                return Err(PolicyError::InvalidRevisionFloor(
                    floor.requirement_id.clone(),
                ));
            }
        }
        let mut policies = HashMap::new();
        for policy in &context.policies {
            let Some(requirement) = requirements.get(&policy.requirement_id) else {
                return Err(PolicyError::InvalidPolicy(policy.requirement_id.clone()));
            };
            let targets: HashSet<_> = requirement.edge_ids.iter().collect();
            if !valid_identity(&policy.bundle)
                || policy.revision > MAX_SAFE_INTEGER
                || policy.issuer_id.is_empty()
                || !valid_time(policy.valid_from_ms)
                || !valid_time(policy.valid_until_ms)
                || policy.valid_until_ms <= policy.valid_from_ms
                || !unique_ids(&policy.closed_edge_ids)
                || policy
                    .closed_edge_ids
                    .iter()
                    .any(|id| !targets.contains(id))
                || policies.insert(&policy.requirement_id, policy).is_some()
            {
                return Err(PolicyError::InvalidPolicy(policy.requirement_id.clone()));
            }
        }
        let trusted: HashSet<_> = context.trusted_issuer_ids.iter().collect();
        let mut assessments: HashMap<_, _> = bundle
            .edge_ids
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    PolicyAssessment {
                        status: Suitability::Suitable,
                        reasons: Vec::new(),
                    },
                )
            })
            .collect();
        for requirement in &bundle.requirements {
            let policy = policies.get(&requirement.id).copied();
            let issue = match policy {
                None => Some(PolicyIssue::Missing),
                Some(policy) if policy.bundle != bundle.identity => {
                    Some(PolicyIssue::BundleMismatch)
                }
                Some(policy) if !trusted.contains(&policy.issuer_id) => {
                    Some(PolicyIssue::UntrustedIssuer)
                }
                Some(policy)
                    if floors
                        .get(&requirement.id)
                        .is_none_or(|floor| policy.revision < *floor) =>
                {
                    Some(PolicyIssue::UntrustedRevision)
                }
                Some(policy)
                    if context.evaluated_at_ms < policy.valid_from_ms
                        || context.evaluated_at_ms >= policy.valid_until_ms =>
                {
                    Some(PolicyIssue::NotCurrent)
                }
                Some(_) => None,
            };
            let closed: HashSet<_> = policy
                .into_iter()
                .flat_map(|policy| &policy.closed_edge_ids)
                .collect();
            for edge_id in &requirement.edge_ids {
                if let Some(issue) =
                    issue.or_else(|| closed.contains(edge_id).then_some(PolicyIssue::Closed))
                {
                    assessments
                        .get_mut(edge_id)
                        .expect("validated requirement edge")
                        .reasons
                        .push(PolicyReason {
                            requirement_id: requirement.id.clone(),
                            issue,
                        });
                }
            }
        }
        for assessment in assessments.values_mut() {
            assessment.reasons.sort_by(|left, right| {
                left.issue
                    .code()
                    .cmp(right.issue.code())
                    .then_with(|| compare_ids(&left.requirement_id, &right.requirement_id))
            });
            assessment.status = if assessment
                .reasons
                .iter()
                .any(|reason| reason.issue == PolicyIssue::Closed)
            {
                Suitability::Unsuitable
            } else if assessment.reasons.is_empty() {
                Suitability::Suitable
            } else {
                Suitability::Unknown
            };
        }
        Ok(Self {
            evaluated_at_ms: context.evaluated_at_ms,
            assessments,
        })
    }

    pub fn evaluated_at_ms(&self) -> i64 {
        self.evaluated_at_ms
    }

    pub fn assess_edge(&self, edge_id: &str) -> Result<&PolicyAssessment, PolicyError> {
        self.assessments
            .get(edge_id)
            .ok_or_else(|| PolicyError::UnknownEdge(edge_id.to_owned()))
    }
}
