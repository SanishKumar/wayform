use crate::validation::MAX_SAFE_INTEGER;
use crate::{
    EdgeKind, EvidenceSource, MeasurementAttribute, Point3, RegistrationControl, ReviewStatus,
};
use serde::{Deserialize, Deserializer, de};
use std::collections::HashMap;

// deserialize_with makes nullable properties required; a missing field is not null.
fn required<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    input: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(input)
}

#[derive(Debug)]
pub struct SafeInteger(pub u64);
impl<'de> Deserialize<'de> for SafeInteger {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(input)?;
        if !value.is_finite()
            || value < 0.0
            || value > MAX_SAFE_INTEGER as f64
            || value.fract() != 0.0
        {
            return Err(de::Error::custom("expected a nonnegative safe integer"));
        }
        Ok(Self(value as u64))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Bundle {
    pub schema_version: String,
    pub id: String,
    pub revision: String,
    pub frames: Vec<Frame>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub evidence: Vec<Evidence>,
    pub observations: Vec<Observation>,
    pub transforms: Vec<Transform>,
    pub policy_requirements: Vec<Requirement>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Frame {
    Local {
        id: String,
        axes: Axes,
    },
    Enu {
        id: String,
        datum: Datum,
        anchor: Anchor,
    },
}
impl Frame {
    pub fn id(&self) -> &str {
        match self {
            Self::Local { id, .. } | Self::Enu { id, .. } => id,
        }
    }
}
#[derive(Deserialize)]
pub enum Axes {
    #[serde(rename = "x-right-y-down-z-up")]
    Down,
    #[serde(rename = "x-right-y-up-z-up")]
    Up,
}
#[derive(Deserialize)]
pub enum Datum {
    WGS84,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Anchor {
    pub longitude: f64,
    pub latitude: f64,
    pub height_meters: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub frame_id: String,
    pub position: Point3,
    #[serde(deserialize_with = "required")]
    pub floor_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub length_meters: f64,
    pub kind: EdgeKind,
    pub access: Access,
    #[serde(deserialize_with = "required")]
    pub transform_id: Option<String>,
    #[serde(deserialize_with = "required")]
    pub legacy: Option<Legacy>,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Public,
    Restricted,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Legacy {
    pub accessible: bool,
    pub meaning: String,
    pub source_commit: String,
    pub package_hash: String,
    pub source_edge_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub source: EvidenceSource,
    pub actor_id: String,
    pub method: String,
    pub observed_at: String,
    #[serde(deserialize_with = "required")]
    pub valid_until: Option<String>,
    pub review: Review,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Review {
    pub status: ReviewStatus,
    #[serde(deserialize_with = "required")]
    pub reviewer_id: Option<String>,
    #[serde(deserialize_with = "required")]
    pub reviewed_at: Option<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Measurement {
    Number(f64),
    Category(String),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Observation {
    pub id: String,
    pub edge_id: String,
    pub attribute: MeasurementAttribute,
    #[serde(deserialize_with = "required")]
    pub value: Option<Measurement>,
    #[serde(deserialize_with = "required")]
    pub uncertainty: Option<f64>,
    #[serde(deserialize_with = "required")]
    pub unknown_reason: Option<String>,
    pub evidence_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Transform {
    pub id: String,
    pub local_frame_id: String,
    pub enu_frame_id: String,
    pub rotation_degrees: f64,
    pub translation_meters: Point3,
    pub uncertainty_meters: f64,
    pub evidence_id: String,
    pub controls: Vec<RegistrationControl>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Requirement {
    pub id: String,
    pub edge_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub allow_restricted: bool,
    pub respect_legacy_exclusions: bool,
    pub hard: Hard,
    pub preferences: Preferences,
    pub evidence_policy: EvidencePolicy,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Hard {
    #[serde(deserialize_with = "required")]
    pub min_clear_width_meters: Option<f64>,
    #[serde(deserialize_with = "required")]
    pub max_uphill_percent: Option<f64>,
    #[serde(deserialize_with = "required")]
    pub max_downhill_percent: Option<f64>,
    #[serde(deserialize_with = "required")]
    pub max_cross_slope_percent: Option<f64>,
    #[serde(deserialize_with = "required")]
    pub max_step_count: Option<SafeInteger>,
    #[serde(deserialize_with = "required")]
    pub allowed_surfaces: Option<Vec<String>>,
    #[serde(deserialize_with = "required")]
    pub allowed_door_operations: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Preferences {
    pub avoid_kinds: Vec<EdgeKind>,
    pub penalty_meters: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvidencePolicy {
    pub accepted_sources: Vec<EvidenceSource>,
    pub require_reviewed: bool,
    pub max_age_seconds: f64,
    pub max_transform_error_meters: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Context {
    pub evaluated_at: String,
    pub policies: Vec<Policy>,
    pub trusted_issuer_ids: Vec<String>,
    pub minimum_policy_revisions: HashMap<String, SafeInteger>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Policy {
    pub requirement_id: String,
    pub bundle_id: String,
    pub bundle_revision: String,
    pub bundle_digest: String,
    pub revision: SafeInteger,
    pub issuer_id: String,
    pub valid_from: String,
    pub valid_until: String,
    pub closed_edge_ids: Vec<String>,
}
