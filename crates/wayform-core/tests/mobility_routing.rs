use serde_json::{Value, json};
use wayform_core::{
    MobilityRouteError, MobilityRouteOutcome, MobilitySnapshot, SearchError, canonical_json,
};

fn load(case: &Value) -> MobilitySnapshot {
    MobilitySnapshot::from_json(
        &serde_json::to_vec(&case["bundle"]).unwrap(),
        &serde_json::to_vec(&case["profile"]).unwrap(),
        &serde_json::to_vec(&case["context"]).unwrap(),
    )
    .unwrap()
}
fn result(snapshot: &MobilitySnapshot, start: &str, destination: &str) -> Value {
    let mut output = match snapshot.route(start, destination) {
        Ok(MobilityRouteOutcome::Routed(assessment)) => {
            json!({"outcome": "routed", "assessment": assessment})
        }
        Ok(MobilityRouteOutcome::InsufficientEvidence(diagnostic)) => {
            json!({"outcome": "insufficient-evidence", "diagnostic": diagnostic})
        }
        Ok(MobilityRouteOutcome::Infeasible(diagnostic)) => {
            json!({"outcome": "infeasible", "diagnostic": diagnostic})
        }
        Err(MobilityRouteError::Search(SearchError::UnknownNode(_))) => {
            json!({"outcome": "rejected", "errorCode": "invalid-search"})
        }
        Err(MobilityRouteError::Search(SearchError::UnrepresentableCost(_))) => {
            json!({"outcome": "rejected", "errorCode": "unrepresentable-cost"})
        }
        other => panic!("unexpected route outcome: {other:?}"),
    };
    output["snapshot"] = serde_json::to_value(snapshot.input_digests()).unwrap();
    output
}

#[test]
fn native_routes_return_matching_receipts_or_pathless_diagnostics() {
    let mut count = 0;
    for line in
        include_str!("../../../packages/mobility-contract/examples/routes-v0.1.0-draft.1.jsonl")
            .lines()
    {
        let mut case: Value = serde_json::from_str(line).unwrap();
        for _ in 0..2 {
            let snapshot = load(&case);
            let actual = result(
                &snapshot,
                case["request"]["startId"].as_str().unwrap(),
                case["request"]["destinationId"].as_str().unwrap(),
            );
            assert_eq!(
                canonical_json(&actual).unwrap(),
                canonical_json(&case["expected"]).unwrap(),
                "{}",
                case["id"]
            );
            for key in [
                "frames",
                "nodes",
                "edges",
                "observations",
                "evidence",
                "transforms",
                "policyRequirements",
            ] {
                case["bundle"][key].as_array_mut().unwrap().reverse();
            }
            case["context"]["policies"]
                .as_array_mut()
                .unwrap()
                .reverse();
        }
        count += 1;
    }
    assert_eq!(count, 29);
}

#[test]
fn evidence_and_context_changes_require_a_new_snapshot() {
    let mut case: Value = serde_json::from_str(
        include_str!("../../../packages/mobility-contract/examples/routes-v0.1.0-draft.1.jsonl")
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let original = load(&case);
    let before = result(&original, "a", "b");
    case["bundle"]["observations"] = json!([]);
    let changed = load(&case);
    assert_eq!(
        result(&changed, "a", "b")["outcome"],
        "insufficient-evidence"
    );
    assert_ne!(original.input_digests(), changed.input_digests());
    assert_eq!(result(&original, "a", "b"), before);
}
