//! Operational-overlay resolution for the pinned VoiceGIS contract.
//! Source: SanishKumar/voicegis-indoor-ar, commit 305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f.
//! Original source and rights statement remain under compat/voicegis/upstream/.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::MAX_EDGES;
use crate::graph::compare_ids;
use crate::validation::{timestamp, valid_digest};

const MAX_RESOLUTION_WORK: usize = 1_000_000;

/// Projection supplied by a host that verified the complete venue package.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyPolicyPackage {
    pub building_id: String,
    pub package_hash: String,
    pub edges: Vec<LegacyPolicyEdge>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyPolicyEdge {
    pub id: String,
    pub source_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyPolicyError {
    InvalidPackage,
    WorkLimitExceeded,
}

impl fmt::Display for LegacyPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidPackage => "invalid legacy policy package projection",
            Self::WorkLimitExceeded => "legacy overlay resolution exceeded its work limit",
        })
    }
}
impl std::error::Error for LegacyPolicyError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyOverlayIssue {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closure_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyOverlayResolution {
    pub valid: bool,
    pub overlay_id: String,
    pub evaluated_at: String,
    pub active_closure_ids: Vec<String>,
    pub closed_edge_ids: Vec<String>,
    pub issues: Vec<LegacyOverlayIssue>,
}

// Legacy overlays allow extra fields. These private records intentionally do
// not impose the mobility draft's closed-object schema on the old contract.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Overlay {
    schema_version: String,
    id: String,
    building_id: String,
    package_hash: String,
    valid_from: String,
    valid_until: String,
    closures: Vec<Closure>,
}

#[derive(Deserialize)]
struct Closure {
    id: String,
    target: Target,
    reason: String,
}

#[derive(Deserialize)]
struct Target {
    #[serde(rename = "type")]
    kind: String,
    id: String,
}

fn parse_overlay(value: &Value) -> Option<Overlay> {
    // Derived struct deserializers also accept positional arrays. The legacy
    // contract requires actual objects at every record boundary.
    let object_shape = value.is_object()
        && value
            .get("closures")
            .and_then(Value::as_array)
            .is_some_and(|closures| {
                closures.iter().all(|closure| {
                    closure.is_object() && closure.get("target").is_some_and(Value::is_object)
                })
            });
    object_shape
        .then(|| Overlay::deserialize(value).ok())
        .flatten()
        .filter(|overlay| {
            overlay.closures.iter().all(|closure| {
                !closure.id.is_empty()
                    && !closure.target.id.is_empty()
                    && !closure.reason.is_empty()
                    && matches!(closure.target.kind.as_str(), "edge" | "source")
            })
        })
}

pub(crate) fn overlay_time_bounds(
    value: &Value,
) -> Result<(Option<i64>, Option<i64>), LegacyPolicyError> {
    if value
        .get("closures")
        .and_then(Value::as_array)
        .is_some_and(|items| items.len() > MAX_EDGES)
    {
        return Err(LegacyPolicyError::WorkLimitExceeded);
    }
    Ok(parse_overlay(value)
        .map(|overlay| {
            (
                timestamp(&overlay.valid_from),
                timestamp(&overlay.valid_until),
            )
        })
        .unwrap_or((None, None)))
}

fn issue(
    code: &'static str,
    message: impl Into<String>,
    closure_id: Option<&str>,
) -> LegacyOverlayIssue {
    LegacyOverlayIssue {
        code,
        message: message.into(),
        closure_id: closure_id.map(str::to_owned),
    }
}

/// Owns immutable source-to-edge mappings; does not verify package bytes or trust.
#[derive(Clone, Debug)]
pub struct LegacyPolicyIndex {
    building_id: String,
    package_hash: String,
    edge_ids: HashSet<String>,
    edges_by_source: HashMap<String, Vec<String>>,
}

impl LegacyPolicyIndex {
    pub fn new(package: LegacyPolicyPackage) -> Result<Self, LegacyPolicyError> {
        if package.building_id.is_empty()
            || !valid_digest(&package.package_hash)
            || package.edges.len() > MAX_EDGES
        {
            return Err(LegacyPolicyError::InvalidPackage);
        }
        let mut edge_ids = HashSet::new();
        let mut edges_by_source: HashMap<String, Vec<String>> = HashMap::new();
        for edge in package.edges {
            if edge.id.is_empty() || edge.source_id.is_empty() || !edge_ids.insert(edge.id.clone())
            {
                return Err(LegacyPolicyError::InvalidPackage);
            }
            edges_by_source
                .entry(edge.source_id)
                .or_default()
                .push(edge.id);
        }
        Ok(Self {
            building_id: package.building_id,
            package_hash: package.package_hash,
            edge_ids,
            edges_by_source,
        })
    }

    /// Resolve at an explicit canonical UTC time. Any invalid result or error
    /// must block routing; an empty closed-edge list alone does not mean open.
    pub fn resolve(
        &self,
        value: &Value,
        evaluated_at: &str,
    ) -> Result<LegacyOverlayResolution, LegacyPolicyError> {
        if value
            .get("closures")
            .and_then(Value::as_array)
            .is_some_and(|items| items.len() > MAX_EDGES)
        {
            return Err(LegacyPolicyError::WorkLimitExceeded);
        }
        let mut result = LegacyOverlayResolution {
            valid: false,
            overlay_id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("<invalid>")
                .to_owned(),
            evaluated_at: evaluated_at.to_owned(),
            active_closure_ids: Vec::new(),
            closed_edge_ids: Vec::new(),
            issues: Vec::new(),
        };
        let overlay = parse_overlay(value);
        let Some(overlay) = overlay else {
            result.issues.push(issue(
                "invalid-shape",
                "Operational overlay does not match the required runtime shape.",
                None,
            ));
            return Ok(result);
        };
        result.overlay_id = overlay.id;
        if overlay.schema_version != "0.1.0" {
            result.issues.push(issue(
                "invalid-schema-version",
                format!(
                    "Unsupported operational overlay version: {}.",
                    overlay.schema_version
                ),
                None,
            ));
        }
        if overlay.building_id != self.building_id {
            result.issues.push(issue(
                "building-mismatch",
                format!(
                    "Overlay building {} does not match {}.",
                    overlay.building_id, self.building_id
                ),
                None,
            ));
        }
        if overlay.package_hash != self.package_hash {
            result.issues.push(issue(
                "package-mismatch",
                "Overlay package hash does not match the active building package.",
                None,
            ));
        }
        match (
            timestamp(evaluated_at),
            timestamp(&overlay.valid_from),
            timestamp(&overlay.valid_until),
        ) {
            (Some(now), Some(from), Some(until)) if until > from => {
                if now < from {
                    result.issues.push(issue(
                        "overlay-not-active",
                        "Operational overlay is not active at the requested evaluation time.",
                        None,
                    ));
                } else if now >= until {
                    result.issues.push(issue(
                        "overlay-expired",
                        "Operational overlay has expired and cannot be applied.",
                        None,
                    ));
                }
            }
            _ => result.issues.push(issue(
                "invalid-time",
                "Overlay timestamps must be valid and validUntil must be after validFrom.",
                None,
            )),
        }
        let mut seen = HashSet::new();
        let mut closed = HashSet::new();
        let mut work = 0;
        for closure in overlay.closures {
            work += 1;
            if work > MAX_RESOLUTION_WORK {
                return Err(LegacyPolicyError::WorkLimitExceeded);
            }
            if !seen.insert(closure.id.clone()) {
                result.issues.push(issue(
                    "duplicate-closure",
                    format!("Duplicate closure ID: {}.", closure.id),
                    Some(&closure.id),
                ));
                continue;
            }
            let matches: &[String] = match closure.target.kind.as_str() {
                "edge" if self.edge_ids.contains(&closure.target.id) => {
                    std::slice::from_ref(&closure.target.id)
                }
                "source" => self
                    .edges_by_source
                    .get(&closure.target.id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                _ => &[],
            };
            if matches.is_empty() {
                result.issues.push(issue(
                    "unknown-target",
                    format!(
                        "Closure target does not exist in the active package: {}.",
                        closure.target.id
                    ),
                    Some(&closure.id),
                ));
                continue;
            }
            work += matches.len();
            if work > MAX_RESOLUTION_WORK {
                return Err(LegacyPolicyError::WorkLimitExceeded);
            }
            closed.extend(matches.iter().cloned());
        }
        result.valid = result.issues.is_empty();
        if result.valid {
            result.active_closure_ids = seen.into_iter().collect();
            result.closed_edge_ids = closed.into_iter().collect();
            result.active_closure_ids.sort_by(|a, b| compare_ids(a, b));
            result.closed_edge_ids.sort_by(|a, b| compare_ids(a, b));
        }
        Ok(result)
    }
}
