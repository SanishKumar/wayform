//! Directed search and typed evidence/measurement evaluation components.
//! Search still consumes explicit eligibility snapshots; full edge assessment
//! combines additional access, closure and coordinate-transform rules.

mod eligibility;
mod evidence;
mod graph;
mod identity;
mod legacy;
mod legacy_policy;
mod loader;
mod measurement;
mod policy;
mod registration;
mod search;
mod validation;

pub use eligibility::{
    EdgeAssessment, EdgeFacts, EdgeKind, EdgeReason, EdgeReasonCode, EdgeRequirements,
    EligibilityError, assess_edge,
};
pub use evidence::{
    EvidenceError, EvidenceIssue, EvidencePolicy, EvidenceRecord, EvidenceSnapshot, EvidenceSource,
    ReviewStatus,
};
pub use graph::{Edge, Graph, GraphError, MAX_EDGES, MAX_NODES};
pub use identity::{IdentityError, InputDigests, bundle_digest, canonical_json, input_digests};
pub use legacy::{
    LEGACY_SOURCE_COMMIT, LegacyEdge, LegacyFloor, LegacyGraph, LegacyNode, LegacyRouteOptions,
    LegacySearchError, LegacySearchOutcome,
};
pub use legacy_policy::{
    LegacyOverlayIssue, LegacyOverlayResolution, LegacyPolicyEdge, LegacyPolicyError,
    LegacyPolicyIndex, LegacyPolicyPackage,
};
pub use loader::{
    AssessmentStatus, Document, LoadError, MAX_DOCUMENT_BYTES, MAX_PATH_EDGES, MOBILITY_VERSION,
    MobilityRouteError, MobilityRouteOutcome, MobilitySnapshot, PathAssessment, PathError,
    PathReason, PathReceipt, PolicyRevision, RouteDiagnostic,
};
pub use measurement::{
    MeasurementAssessment, MeasurementAttribute, MeasurementError, MeasurementObservation,
    MeasurementReason, MeasurementReasonCode, MeasurementRequirement, MeasurementValue,
    assess_measurement,
};
pub use policy::{
    BundleIdentity, OperationalPolicy, PolicyAssessment, PolicyBundle, PolicyContext, PolicyError,
    PolicyIssue, PolicyReason, PolicyRequirement, PolicySnapshot, RevisionFloor,
};
pub use registration::{EntranceRegistration, Point3, RegistrationControl};
pub use search::{EdgeDecision, Path, SearchError, SearchOutcome, Suitability};
