mod json;
mod path;
mod route;
mod wire;

pub use route::{MobilityRouteError, MobilityRouteOutcome, RouteDiagnostic};

pub use path::{
    AssessmentStatus, MAX_PATH_EDGES, PathAssessment, PathError, PathReason, PathReceipt,
    PolicyRevision,
};

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::measurement::valid_observation;
use crate::validation::valid_digest;
use crate::{
    BundleIdentity, Edge, EdgeAssessment, EdgeDecision, EdgeFacts, EdgeRequirements,
    EligibilityError, EntranceRegistration, EvidencePolicy, EvidenceRecord, EvidenceSnapshot,
    Graph, InputDigests, MeasurementObservation, MeasurementRequirement, MeasurementValue,
    OperationalPolicy, PolicyBundle, PolicyContext, PolicyRequirement, PolicySnapshot,
    RevisionFloor, assess_edge, input_digests,
};

pub const MOBILITY_VERSION: &str = "0.1.0-draft.1";
pub use json::MAX_DOCUMENT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Document {
    Bundle,
    Profile,
    Context,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    InvalidJson(Document),
    InvalidContract { document: Document, detail: String },
    ResourceLimit,
    Identity(crate::IdentityError),
}
impl LoadError {
    fn contract(document: Document, detail: impl ToString) -> Self {
        Self::InvalidContract {
            document,
            detail: detail.to_string(),
        }
    }
}
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(document) => write!(f, "invalid or ambiguous {document:?} JSON"),
            Self::InvalidContract { document, detail } => {
                write!(f, "invalid {document:?}: {detail}")
            }
            Self::ResourceLimit => f.write_str("mobility document exceeds input limit"),
            Self::Identity(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for LoadError {}

fn decode<T: DeserializeOwned>(value: &Value, document: Document) -> Result<T, LoadError> {
    serde_json::from_value(value.clone()).map_err(|error| LoadError::contract(document, error))
}
fn time(value: &str, document: Document) -> Result<i64, LoadError> {
    json::timestamp(value).ok_or_else(|| LoadError::contract(document, "invalid UTC timestamp"))
}
fn unique<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .all(|value| !value.is_empty() && seen.insert(value))
}

/// An immutable, fully validated draft evaluation at an explicit supplied time.
/// Publisher authentication and durable revision floors remain host duties.
#[derive(Clone, Debug)]
pub struct MobilitySnapshot {
    graph: Graph,
    facts: HashMap<String, EdgeFacts>,
    requirements: EdgeRequirements,
    evidence: EvidenceSnapshot,
    policies: PolicySnapshot,
    policy_bundle: PolicyBundle,
    policy_context: PolicyContext,
    profile_id: String,
    evaluated_at: String,
    digests: InputDigests,
    avoid_kinds: Vec<crate::EdgeKind>,
    penalty_meters: f64,
}

fn load_requirements(
    profile: &wire::Profile,
) -> Result<(EdgeRequirements, EvidencePolicy), LoadError> {
    let hard = &profile.hard;
    let mut measurements = Vec::new();
    if let Some(value) = hard.min_clear_width_meters {
        measurements.push(MeasurementRequirement::MinimumWidth(value));
    }
    if hard.max_uphill_percent.is_some() || hard.max_downhill_percent.is_some() {
        measurements.push(MeasurementRequirement::Slope {
            max_uphill: hard.max_uphill_percent,
            max_downhill: hard.max_downhill_percent,
        });
    }
    if let Some(value) = hard.max_cross_slope_percent {
        measurements.push(MeasurementRequirement::MaximumCrossSlope(value));
    }
    if let Some(value) = &hard.max_step_count {
        measurements.push(MeasurementRequirement::MaximumSteps(value.0));
    }
    if let Some(value) = &hard.allowed_surfaces {
        measurements.push(MeasurementRequirement::Surfaces(value.clone()));
    }
    if let Some(value) = &hard.allowed_door_operations {
        measurements.push(MeasurementRequirement::DoorOperations(value.clone()));
    }
    if measurements.iter().any(|value| !value.valid())
        || !profile.preferences.penalty_meters.is_finite()
        || profile.preferences.penalty_meters < 0.0
        || profile
            .preferences
            .avoid_kinds
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != profile.preferences.avoid_kinds.len()
        || !profile
            .evidence_policy
            .max_transform_error_meters
            .is_finite()
        || profile.evidence_policy.max_transform_error_meters < 0.0
    {
        return Err(LoadError::contract(
            Document::Profile,
            "invalid mobility requirements or preferences",
        ));
    }
    let policy = EvidencePolicy {
        accepted_sources: profile.evidence_policy.accepted_sources.clone(),
        require_reviewed: profile.evidence_policy.require_reviewed,
        max_age_seconds: profile.evidence_policy.max_age_seconds,
    };
    EvidenceSnapshot::new(&[], &policy, 0)
        .map_err(|error| LoadError::contract(Document::Profile, error))?;
    Ok((
        EdgeRequirements {
            allow_restricted: profile.allow_restricted,
            respect_legacy_exclusions: profile.respect_legacy_exclusions,
            measurements,
            max_transform_error_meters: profile.evidence_policy.max_transform_error_meters,
        },
        policy,
    ))
}

fn load_records(bundle: &wire::Bundle) -> Result<Vec<EvidenceRecord>, LoadError> {
    bundle
        .evidence
        .iter()
        .map(|record| {
            Ok(EvidenceRecord {
                id: record.id.clone(),
                source: record.source,
                actor_id: record.actor_id.clone(),
                method: record.method.clone(),
                observed_at_ms: time(&record.observed_at, Document::Bundle)?,
                valid_until_ms: record
                    .valid_until
                    .as_deref()
                    .map(|value| time(value, Document::Bundle))
                    .transpose()?,
                review_status: record.review.status,
                reviewer_id: record.review.reviewer_id.clone(),
                reviewed_at_ms: record
                    .review
                    .reviewed_at
                    .as_deref()
                    .map(|value| time(value, Document::Bundle))
                    .transpose()?,
            })
        })
        .collect()
}

fn load_facts(
    bundle: &wire::Bundle,
    evidence: &EvidenceSnapshot,
) -> Result<HashMap<String, EdgeFacts>, LoadError> {
    let invalid = |detail: &str| LoadError::contract(Document::Bundle, detail);
    if bundle.schema_version != MOBILITY_VERSION
        || bundle.frames.is_empty()
        || !unique(bundle.frames.iter().map(wire::Frame::id))
        || !unique(bundle.observations.iter().map(|value| value.id.as_str()))
        || !unique(bundle.transforms.iter().map(|value| value.id.as_str()))
    {
        return Err(invalid(
            "wrong schema version, empty frames or duplicate IDs",
        ));
    }
    let frames: HashMap<_, _> = bundle
        .frames
        .iter()
        .map(|frame| (frame.id(), frame))
        .collect();
    for frame in &bundle.frames {
        if let wire::Frame::Enu { datum, anchor, .. } = frame {
            if !matches!(datum, wire::Datum::WGS84)
                || !(-180.0..=180.0).contains(&anchor.longitude)
                || !(-90.0..=90.0).contains(&anchor.latitude)
                || !anchor.height_meters.is_finite()
            {
                return Err(invalid("invalid geographic anchor"));
            }
        }
    }
    let nodes: HashMap<_, _> = bundle
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    for node in &bundle.nodes {
        if !frames.contains_key(node.frame_id.as_str())
            || node.position.iter().any(|value| !value.is_finite())
            || node.floor_id.as_ref().is_some_and(|value| value.is_empty())
        {
            return Err(invalid("invalid node frame, position or floor"));
        }
    }
    let mut registrations = HashMap::new();
    for transform in &bundle.transforms {
        let Some(wire::Frame::Local { axes, .. }) = frames.get(transform.local_frame_id.as_str())
        else {
            return Err(invalid("transform requires a local frame"));
        };
        if !matches!(
            frames.get(transform.enu_frame_id.as_str()),
            Some(wire::Frame::Enu { .. })
        ) {
            return Err(invalid("transform requires an ENU frame"));
        }
        evidence
            .issue(&transform.evidence_id)
            .map_err(|error| LoadError::contract(Document::Bundle, error))?;
        let registration = EntranceRegistration {
            local_y_down: matches!(axes, wire::Axes::Down),
            rotation_degrees: transform.rotation_degrees,
            translation_meters: transform.translation_meters,
            uncertainty_meters: transform.uncertainty_meters,
            evidence_id: transform.evidence_id.clone(),
            controls: transform.controls.clone(),
            local_endpoint: [0.0; 3],
            enu_endpoint: [0.0; 3],
        };
        if !registration.valid() {
            return Err(invalid("invalid or degenerate transform controls"));
        }
        registrations.insert(transform.id.as_str(), (transform, registration));
    }
    let mut facts = HashMap::new();
    // Graph construction has already checked every endpoint and edge ID.
    for edge in &bundle.edges {
        let from = nodes[edge.from.as_str()];
        let to = nodes[edge.to.as_str()];
        let crosses_frame = from.frame_id != to.frame_id;
        let registration = if crosses_frame {
            let Some((transform, registration)) = edge
                .transform_id
                .as_deref()
                .and_then(|id| registrations.get(id))
            else {
                return Err(invalid("missing entrance transform"));
            };
            let (local, enu) = if from.frame_id == transform.local_frame_id {
                (from, to)
            } else {
                (to, from)
            };
            if edge.kind != crate::EdgeKind::Entrance
                || local.frame_id != transform.local_frame_id
                || enu.frame_id != transform.enu_frame_id
            {
                return Err(invalid("wrong entrance transform or edge kind"));
            }
            Some(EntranceRegistration {
                local_endpoint: local.position,
                enu_endpoint: enu.position,
                ..registration.clone()
            })
        } else {
            if edge.transform_id.is_some() {
                return Err(invalid("unnecessary transform"));
            }
            None
        };
        if let Some(legacy) = &edge.legacy {
            if legacy.meaning != "author-assertion"
                || legacy.source_commit.len() != 40
                || !legacy
                    .source_commit
                    .bytes()
                    .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
                || !valid_digest(&legacy.package_hash)
                || legacy.source_edge_id.is_empty()
            {
                return Err(invalid("invalid legacy provenance"));
            }
        }
        facts.insert(
            edge.id.clone(),
            EdgeFacts {
                id: edge.id.clone(),
                kind: edge.kind,
                restricted: matches!(edge.access, wire::Access::Restricted),
                legacy_accessible: edge.legacy.as_ref().map(|value| value.accessible),
                observations: Vec::new(),
                crosses_frame,
                registration,
            },
        );
    }
    for observation in &bundle.observations {
        let fact = facts
            .get_mut(&observation.edge_id)
            .ok_or_else(|| invalid("unknown observation edge"))?;
        evidence
            .issue(&observation.evidence_id)
            .map_err(|error| LoadError::contract(Document::Bundle, error))?;
        let value = MeasurementObservation {
            id: observation.id.clone(),
            attribute: observation.attribute,
            value: match &observation.value {
                None => MeasurementValue::Unknown,
                Some(wire::Measurement::Number(value)) => MeasurementValue::Number(*value),
                Some(wire::Measurement::Category(value)) => {
                    MeasurementValue::Category(value.clone())
                }
            },
            uncertainty: observation.uncertainty,
            unknown_reason: observation.unknown_reason.clone(),
            evidence_id: observation.evidence_id.clone(),
        };
        if !valid_observation(&value, value.attribute) {
            return Err(invalid("invalid measurement observation"));
        }
        fact.observations.push(value);
    }
    Ok(facts)
}

impl MobilitySnapshot {
    pub fn from_json(bundle: &[u8], profile: &[u8], context: &[u8]) -> Result<Self, LoadError> {
        let bundle_json = json::parse(bundle, Document::Bundle)?;
        let profile_json = json::parse(profile, Document::Profile)?;
        let context_json = json::parse(context, Document::Context)?;
        let bundle: wire::Bundle = decode(&bundle_json, Document::Bundle)?;
        let profile: wire::Profile = decode(&profile_json, Document::Profile)?;
        let context: wire::Context = decode(&context_json, Document::Context)?;
        let (requirements, evidence_policy) = load_requirements(&profile)?;
        let now = time(&context.evaluated_at, Document::Context)?;
        let evidence = EvidenceSnapshot::new(&load_records(&bundle)?, &evidence_policy, now)
            .map_err(|error| LoadError::contract(Document::Bundle, error))?;
        let graph = Graph::new(
            bundle.nodes.iter().map(|value| value.id.clone()).collect(),
            bundle
                .edges
                .iter()
                .map(|edge| Edge {
                    id: edge.id.clone(),
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    length_meters: edge.length_meters,
                })
                .collect(),
        )
        .map_err(|error| LoadError::contract(Document::Bundle, error))?;
        let facts = load_facts(&bundle, &evidence)?;
        if !unique(
            bundle
                .policy_requirements
                .iter()
                .map(|value| value.id.as_str()),
        ) || bundle.policy_requirements.iter().any(|value| {
            value.edge_ids.is_empty()
                || !unique(value.edge_ids.iter().map(String::as_str))
                || value.edge_ids.iter().any(|id| !facts.contains_key(id))
        }) {
            return Err(LoadError::contract(
                Document::Bundle,
                "invalid policy requirements",
            ));
        }
        let digests = input_digests(&bundle_json, &profile_json, &context_json)
            .map_err(LoadError::Identity)?;
        let policy_bundle = PolicyBundle {
            identity: BundleIdentity {
                id: bundle.id,
                revision: bundle.revision,
                digest: digests.bundle_digest.clone(),
            },
            edge_ids: bundle.edges.iter().map(|edge| edge.id.clone()).collect(),
            requirements: bundle
                .policy_requirements
                .into_iter()
                .map(|value| PolicyRequirement {
                    id: value.id,
                    edge_ids: value.edge_ids,
                })
                .collect(),
        };
        let policy_context = PolicyContext {
            evaluated_at_ms: now,
            trusted_issuer_ids: context.trusted_issuer_ids,
            minimum_revisions: context
                .minimum_policy_revisions
                .into_iter()
                .map(|(requirement_id, value)| RevisionFloor {
                    requirement_id,
                    revision: value.0,
                })
                .collect(),
            policies: context
                .policies
                .into_iter()
                .map(|policy| {
                    Ok(OperationalPolicy {
                        requirement_id: policy.requirement_id,
                        bundle: BundleIdentity {
                            id: policy.bundle_id,
                            revision: policy.bundle_revision,
                            digest: policy.bundle_digest,
                        },
                        revision: policy.revision.0,
                        issuer_id: policy.issuer_id,
                        valid_from_ms: time(&policy.valid_from, Document::Context)?,
                        valid_until_ms: time(&policy.valid_until, Document::Context)?,
                        closed_edge_ids: policy.closed_edge_ids,
                    })
                })
                .collect::<Result<Vec<_>, LoadError>>()?,
        };
        let policies = PolicySnapshot::new(&policy_bundle, &policy_context)
            .map_err(|error| LoadError::contract(Document::Context, error))?;
        Ok(Self {
            graph,
            facts,
            requirements,
            evidence,
            policies,
            policy_bundle,
            policy_context,
            profile_id: profile.id,
            evaluated_at: context.evaluated_at,
            digests,
            avoid_kinds: profile.preferences.avoid_kinds,
            penalty_meters: profile.preferences.penalty_meters,
        })
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }
    pub fn input_digests(&self) -> &InputDigests {
        &self.digests
    }
    pub fn bundle_identity(&self) -> &BundleIdentity {
        &self.policy_bundle.identity
    }
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }
    pub fn evaluated_at(&self) -> &str {
        &self.evaluated_at
    }
    pub fn policy_context(&self) -> &PolicyContext {
        &self.policy_context
    }
    pub fn policy_requirements(&self) -> &[PolicyRequirement] {
        &self.policy_bundle.requirements
    }

    pub fn assess_edge(&self, edge_id: &str) -> Result<EdgeAssessment, EligibilityError> {
        let facts = self.facts.get(edge_id).ok_or_else(|| {
            EligibilityError::Policy(crate::PolicyError::UnknownEdge(edge_id.to_owned()))
        })?;
        assess_edge(facts, &self.requirements, &self.evidence, &self.policies)
    }

    pub fn decisions(&self) -> Result<Vec<EdgeDecision>, EligibilityError> {
        self.graph
            .edges
            .iter()
            .map(|edge| {
                Ok(EdgeDecision {
                    edge_id: edge.id.clone(),
                    suitability: self.assess_edge(&edge.id)?.status,
                    penalty_meters: if self.avoid_kinds.contains(&self.facts[&edge.id].kind) {
                        self.penalty_meters
                    } else {
                        0.0
                    },
                })
            })
            .collect()
    }
}
