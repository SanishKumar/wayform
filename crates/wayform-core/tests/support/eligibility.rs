use serde_json::Value;
use wayform_core::{
    EdgeFacts, EdgeKind, EdgeRequirements, EntranceRegistration, EvidencePolicy, EvidenceRecord,
    EvidenceSnapshot, EvidenceSource, MeasurementAttribute, MeasurementObservation,
    MeasurementRequirement, MeasurementValue, Point3, PolicySnapshot, RegistrationControl,
    ReviewStatus,
};

#[path = "policy.rs"]
mod policy;

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap().to_owned()
}
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect()
}
fn point(value: &Value) -> Point3 {
    std::array::from_fn(|index| value[index].as_f64().unwrap())
}
fn source(value: &str) -> EvidenceSource {
    match value {
        "field-survey" => EvidenceSource::FieldSurvey,
        "operator" => EvidenceSource::Operator,
        "imported-map" => EvidenceSource::ImportedMap,
        "inference" => EvidenceSource::Inference,
        _ => panic!("unexpected source"),
    }
}
fn attribute(value: &str) -> MeasurementAttribute {
    match value {
        "clearWidthMeters" => MeasurementAttribute::ClearWidthMeters,
        "slopePercent" => MeasurementAttribute::SlopePercent,
        "crossSlopePercent" => MeasurementAttribute::CrossSlopePercent,
        "stepCount" => MeasurementAttribute::StepCount,
        "surface" => MeasurementAttribute::Surface,
        "doorOperation" => MeasurementAttribute::DoorOperation,
        _ => panic!("unexpected attribute"),
    }
}

pub fn inputs(
    case: &Value,
) -> (
    EdgeFacts,
    EdgeRequirements,
    EvidenceSnapshot,
    PolicySnapshot,
) {
    let (bundle, context) = policy::inputs(&case["policyInput"]);
    let policies = PolicySnapshot::new(&bundle, &context).unwrap();
    let evidence = &case["evidenceInput"];
    let records: Vec<_> = evidence["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            let review = &item["review"];
            EvidenceRecord {
                id: text(item, "id"),
                source: source(&text(item, "source")),
                actor_id: text(item, "actorId"),
                method: text(item, "method"),
                observed_at_ms: item["observedAtMs"].as_i64().unwrap(),
                valid_until_ms: item["validUntilMs"].as_i64(),
                review_status: match review["status"].as_str().unwrap() {
                    "reviewed" => ReviewStatus::Reviewed,
                    "pending" => ReviewStatus::Pending,
                    "rejected" => ReviewStatus::Rejected,
                    _ => panic!("unexpected review"),
                },
                reviewer_id: review["reviewerId"].as_str().map(str::to_owned),
                reviewed_at_ms: review["reviewedAtMs"].as_i64(),
            }
        })
        .collect();
    let evidence = EvidenceSnapshot::new(
        &records,
        &EvidencePolicy {
            accepted_sources: strings(&evidence["policy"]["acceptedSources"])
                .iter()
                .map(|value| source(value))
                .collect(),
            require_reviewed: evidence["policy"]["requireReviewed"].as_bool().unwrap(),
            max_age_seconds: evidence["policy"]["maxAgeSeconds"].as_f64().unwrap(),
        },
        evidence["evaluatedAtMs"].as_i64().unwrap(),
    )
    .unwrap();
    let requirements = &case["requirements"];
    let hard = &requirements["hard"];
    let mut measurements = Vec::new();
    if let Some(value) = hard["minClearWidthMeters"].as_f64() {
        measurements.push(MeasurementRequirement::MinimumWidth(value));
    }
    if !hard["maxUphillPercent"].is_null() || !hard["maxDownhillPercent"].is_null() {
        measurements.push(MeasurementRequirement::Slope {
            max_uphill: hard["maxUphillPercent"].as_f64(),
            max_downhill: hard["maxDownhillPercent"].as_f64(),
        });
    }
    if let Some(value) = hard["maxCrossSlopePercent"].as_f64() {
        measurements.push(MeasurementRequirement::MaximumCrossSlope(value));
    }
    if let Some(value) = hard["maxStepCount"].as_u64() {
        measurements.push(MeasurementRequirement::MaximumSteps(value));
    }
    if !hard["allowedSurfaces"].is_null() {
        measurements.push(MeasurementRequirement::Surfaces(strings(
            &hard["allowedSurfaces"],
        )));
    }
    if !hard["allowedDoorOperations"].is_null() {
        measurements.push(MeasurementRequirement::DoorOperations(strings(
            &hard["allowedDoorOperations"],
        )));
    }
    let requirements = EdgeRequirements {
        allow_restricted: requirements["allowRestricted"].as_bool().unwrap(),
        respect_legacy_exclusions: requirements["respectLegacyExclusions"].as_bool().unwrap(),
        measurements,
        max_transform_error_meters: requirements["maxTransformErrorMeters"].as_f64().unwrap(),
    };
    let facts = &case["facts"];
    let registration = &facts["registration"];
    let registration = (!registration.is_null()).then(|| EntranceRegistration {
        local_y_down: registration["localYDown"].as_bool().unwrap(),
        rotation_degrees: registration["rotationDegrees"].as_f64().unwrap(),
        translation_meters: point(&registration["translationMeters"]),
        uncertainty_meters: registration["uncertaintyMeters"].as_f64().unwrap(),
        evidence_id: text(registration, "evidenceId"),
        local_endpoint: point(&registration["localEndpoint"]),
        enu_endpoint: point(&registration["enuEndpoint"]),
        controls: registration["controls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|control| RegistrationControl {
                local: point(&control["local"]),
                enu: point(&control["enu"]),
            })
            .collect(),
    });
    let facts = EdgeFacts {
        id: text(facts, "id"),
        kind: match facts["kind"].as_str().unwrap() {
            "walkway" => EdgeKind::Walkway,
            "door" => EdgeKind::Door,
            "ramp" => EdgeKind::Ramp,
            "stairs" => EdgeKind::Stairs,
            "lift" => EdgeKind::Lift,
            "crossing" => EdgeKind::Crossing,
            "entrance" => EdgeKind::Entrance,
            _ => panic!("unexpected kind"),
        },
        restricted: facts["restricted"].as_bool().unwrap(),
        legacy_accessible: facts["legacyAccessible"].as_bool(),
        crosses_frame: facts["crossesFrame"].as_bool().unwrap(),
        registration,
        observations: facts["observations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| MeasurementObservation {
                id: text(item, "id"),
                attribute: attribute(&text(item, "attribute")),
                value: if item["value"].is_null() {
                    MeasurementValue::Unknown
                } else if let Some(number) = item["value"].as_f64() {
                    MeasurementValue::Number(number)
                } else {
                    MeasurementValue::Category(text(item, "value"))
                },
                uncertainty: item["uncertainty"].as_f64(),
                unknown_reason: item["unknownReason"].as_str().map(str::to_owned),
                evidence_id: text(item, "evidenceId"),
            })
            .collect(),
    };
    (facts, requirements, evidence, policies)
}
