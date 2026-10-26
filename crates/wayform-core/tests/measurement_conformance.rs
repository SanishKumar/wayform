use serde_json::{Value, json};
use wayform_core::{
    EvidencePolicy, EvidenceRecord, EvidenceSnapshot, EvidenceSource, MeasurementAttribute,
    MeasurementObservation, MeasurementRequirement, MeasurementValue, ReviewStatus, Suitability,
    assess_measurement,
};

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect()
}

fn source(value: &str) -> EvidenceSource {
    match value {
        "field-survey" => EvidenceSource::FieldSurvey,
        "operator" => EvidenceSource::Operator,
        "imported-map" => EvidenceSource::ImportedMap,
        "inference" => EvidenceSource::Inference,
        _ => panic!("unexpected fixture source: {value}"),
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
        _ => panic!("unexpected fixture attribute: {value}"),
    }
}

#[test]
fn measurement_decisions_match_typescript_in_both_collection_orders() {
    let lines = include_str!(
        "../../../packages/mobility-contract/examples/measurements-v0.1.0-draft.1.jsonl"
    )
    .lines();
    let mut count = 0;
    for line in lines {
        let case: Value = serde_json::from_str(line).unwrap();
        assert_eq!(case["synthetic"], true);
        let id = string(&case, "id");
        let hard = &case["hard"];
        let requirement = match attribute(string(&case, "attribute")) {
            MeasurementAttribute::ClearWidthMeters => {
                MeasurementRequirement::MinimumWidth(hard["minClearWidthMeters"].as_f64().unwrap())
            }
            MeasurementAttribute::SlopePercent => MeasurementRequirement::Slope {
                max_uphill: hard["maxUphillPercent"].as_f64(),
                max_downhill: hard["maxDownhillPercent"].as_f64(),
            },
            MeasurementAttribute::CrossSlopePercent => MeasurementRequirement::MaximumCrossSlope(
                hard["maxCrossSlopePercent"].as_f64().unwrap(),
            ),
            MeasurementAttribute::StepCount => {
                MeasurementRequirement::MaximumSteps(hard["maxStepCount"].as_u64().unwrap())
            }
            MeasurementAttribute::Surface => {
                MeasurementRequirement::Surfaces(strings(&hard["allowedSurfaces"]))
            }
            MeasurementAttribute::DoorOperation => {
                MeasurementRequirement::DoorOperations(strings(&hard["allowedDoorOperations"]))
            }
        };
        let mut records: Vec<_> = case["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                let review = &item["review"];
                EvidenceRecord {
                    id: string(item, "id").into(),
                    source: source(string(item, "source")),
                    actor_id: string(item, "actorId").into(),
                    method: string(item, "method").into(),
                    observed_at_ms: item["observedAtMs"].as_i64().unwrap(),
                    valid_until_ms: item["validUntilMs"].as_i64(),
                    review_status: match string(review, "status") {
                        "reviewed" => ReviewStatus::Reviewed,
                        "pending" => ReviewStatus::Pending,
                        "rejected" => ReviewStatus::Rejected,
                        other => panic!("unexpected review: {other}"),
                    },
                    reviewer_id: review["reviewerId"].as_str().map(str::to_owned),
                    reviewed_at_ms: review["reviewedAtMs"].as_i64(),
                }
            })
            .collect();
        let policy = EvidencePolicy {
            accepted_sources: strings(&case["policy"]["acceptedSources"])
                .iter()
                .map(|value| source(value))
                .collect(),
            require_reviewed: case["policy"]["requireReviewed"].as_bool().unwrap(),
            max_age_seconds: case["policy"]["maxAgeSeconds"].as_f64().unwrap(),
        };
        let now = case["evaluatedAtMs"].as_i64().unwrap();
        let mut observations: Vec<_> = case["observations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                let value = &item["value"];
                MeasurementObservation {
                    id: string(item, "id").into(),
                    attribute: attribute(string(item, "attribute")),
                    value: if value.is_null() {
                        MeasurementValue::Unknown
                    } else if let Some(number) = value.as_f64() {
                        MeasurementValue::Number(number)
                    } else {
                        MeasurementValue::Category(value.as_str().unwrap().into())
                    },
                    uncertainty: item["uncertainty"].as_f64(),
                    unknown_reason: item["unknownReason"].as_str().map(str::to_owned),
                    evidence_id: string(item, "evidenceId").into(),
                }
            })
            .collect();
        for _ in 0..2 {
            let snapshot = EvidenceSnapshot::new(&records, &policy, now).unwrap();
            let assessment = assess_measurement(&requirement, &observations, &snapshot).unwrap();
            let result = json!({
                "status": match assessment.status { Suitability::Suitable => "suitable", Suitability::Unsuitable => "unsuitable", Suitability::Unknown => "unknown" },
                "reasons": assessment.reasons.iter().map(|reason| json!({"code": reason.code.code(), "evidenceIds": reason.evidence_ids})).collect::<Vec<_>>(),
                "evidenceIds": assessment.evidence_ids,
            });
            assert_eq!(result, case["expected"], "{id}");
            records.reverse();
            observations.reverse();
        }
        count += 1;
    }
    assert_eq!(count, 63);
}
