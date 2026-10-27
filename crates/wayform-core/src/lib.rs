//! Directed search and typed evidence/measurement evaluation components.
//! Search still consumes explicit eligibility snapshots; full edge assessment
//! combines additional access, closure and coordinate-transform rules.

mod eligibility;
mod evidence;
mod graph;
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
