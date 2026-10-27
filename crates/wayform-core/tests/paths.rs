use serde_json::{Value, json};
use wayform_core::{MAX_PATH_EDGES, MobilitySnapshot, PathError, canonical_json};

fn load(case: &Value) -> MobilitySnapshot {
    MobilitySnapshot::from_json(
        &serde_json::to_vec(&case["bundle"]).unwrap(),
        &serde_json::to_vec(&case["profile"]).unwrap(),
        &serde_json::to_vec(&case["context"]).unwrap(),
    )
    .unwrap()
}
fn check(case: &Value) {
    let snapshot = load(case);
    let request = &case["request"];
    let edges: Vec<String> = serde_json::from_value(request["edgeIds"].clone()).unwrap();
    let actual = snapshot
        .assess_path(
            request["startId"].as_str().unwrap(),
            request["destinationId"].as_str().unwrap(),
            &edges,
        )
        .unwrap();
    assert_eq!(
        canonical_json(&serde_json::to_value(actual).unwrap()).unwrap(),
        canonical_json(&case["expected"]).unwrap(),
        "{}",
        case["id"]
    );
}
fn permute(case: &mut Value) {
    for field in [
        "frames",
        "nodes",
        "edges",
        "evidence",
        "observations",
        "transforms",
        "policyRequirements",
    ] {
        case["bundle"][field].as_array_mut().unwrap().reverse();
    }
    for transform in case["bundle"]["transforms"].as_array_mut().unwrap() {
        transform["controls"].as_array_mut().unwrap().reverse();
    }
    for requirement in case["bundle"]["policyRequirements"].as_array_mut().unwrap() {
        requirement["edgeIds"].as_array_mut().unwrap().reverse();
    }
    case["context"]["policies"]
        .as_array_mut()
        .unwrap()
        .reverse();
    case["context"]["trustedIssuerIds"]
        .as_array_mut()
        .unwrap()
        .reverse();
    case["profile"]["evidencePolicy"]["acceptedSources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    case["profile"]["preferences"]["avoidKinds"]
        .as_array_mut()
        .unwrap()
        .reverse();
}
#[test]
fn complete_portable_receipts_match_in_both_collection_orders() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json"
    ))
    .unwrap();
    for mut case in document["examples"].as_array().unwrap().clone() {
        check(&case);
        permute(&mut case);
        check(&case);
    }
}
#[test]
fn path_failures_costs_cycles_and_policy_receipts_match_typescript() {
    let mut count = 0;
    for line in
        include_str!("../../../packages/mobility-contract/examples/paths-v0.1.0-draft.1.jsonl")
            .lines()
    {
        let mut case: Value = serde_json::from_str(line).unwrap();
        check(&case);
        permute(&mut case);
        check(&case);
        count += 1;
    }
    assert_eq!(count, 22);
}
#[test]
fn path_resource_errors_are_not_suitability_or_infeasibility_claims() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json"
    ))
    .unwrap();
    let mut case = document["examples"][0].clone();
    let snapshot = load(&case);
    assert_eq!(
        snapshot.assess_path("a", "b", &vec!["ab".into(); MAX_PATH_EDGES + 1]),
        Err(PathError::ResourceLimit)
    );
    let id = "x".repeat(4096);
    case["bundle"]["edges"][0]["id"] = json!(id);
    case["bundle"]["observations"][0]["edgeId"] = json!(id);
    let snapshot = load(&case);
    assert_eq!(
        snapshot.assess_path("a", "b", &vec![id; 4097]),
        Err(PathError::ResourceLimit)
    );
}
