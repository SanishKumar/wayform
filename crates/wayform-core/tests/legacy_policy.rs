use serde_json::{Value, json};
use wayform_core::{
    LEGACY_SOURCE_COMMIT, LegacyPolicyEdge, LegacyPolicyError, LegacyPolicyIndex,
    LegacyPolicyPackage, MAX_EDGES,
};

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/operational-overlays.json"
    ))
    .unwrap()
}

#[test]
fn resolutions_match_the_pinned_resolver_including_issue_messages_and_order() {
    let corpus = corpus();
    assert_eq!(corpus["sourceCommit"], LEGACY_SOURCE_COMMIT);
    let mut count = 0;
    for group in corpus["groups"].as_array().unwrap() {
        let package: LegacyPolicyPackage =
            serde_json::from_value(group["package"].clone()).unwrap();
        let index = LegacyPolicyIndex::new(package.clone()).unwrap();
        let mut reversed = package;
        reversed.edges.reverse();
        let reversed = LegacyPolicyIndex::new(reversed).unwrap();
        for case in group["cases"].as_array().unwrap() {
            let now = case["evaluatedAt"].as_str().unwrap();
            let actual = index.resolve(&case["overlay"], now).unwrap();
            assert_eq!(actual, reversed.resolve(&case["overlay"], now).unwrap());
            assert_eq!(
                serde_json::to_value(&actual).unwrap(),
                case["expected"],
                "{}/{}",
                group["id"],
                case["id"]
            );
            if !actual.valid {
                assert!(actual.active_closure_ids.is_empty());
                assert!(actual.closed_edge_ids.is_empty());
                assert!(!actual.issues.is_empty());
            }
            count += 1;
        }
    }
    assert_eq!(count, 58);
}

#[test]
fn invalid_package_projections_never_construct_an_index() {
    let base: LegacyPolicyPackage =
        serde_json::from_value(corpus()["groups"][0]["package"].clone()).unwrap();
    let mut duplicate = base.clone();
    duplicate.edges.push(duplicate.edges[0].clone());
    let mut no_source = base.clone();
    no_source.edges[0].source_id.clear();
    let mut no_edge_id = base.clone();
    no_edge_id.edges[0].id.clear();
    let mut no_building = base.clone();
    no_building.building_id.clear();
    let mut bad_hash = base;
    bad_hash.package_hash = "unverified".into();
    for package in [duplicate, no_source, no_edge_id, no_building, bad_hash] {
        assert!(matches!(
            LegacyPolicyIndex::new(package),
            Err(LegacyPolicyError::InvalidPackage)
        ));
    }
}

#[test]
fn timestamps_outside_the_canonical_utc_subset_are_explicitly_invalid() {
    let corpus = corpus();
    let group = &corpus["groups"][0];
    let index =
        LegacyPolicyIndex::new(serde_json::from_value(group["package"].clone()).unwrap()).unwrap();
    let overlay = group["cases"][0]["overlay"].clone();
    let now = "2026-07-22T12:00:00.000Z";
    // Some of these are accepted/normalized by Date.parse. Rejecting them is
    // a documented native boundary, not claimed legacy parsing equivalence.
    for value in [
        "2026-07-22T12:00:00Z",
        "2026-07-22T12:00:00.000+00:00",
        "2026-07-22",
        "2026-02-30T12:00:00.000Z",
        "2026-07-22T24:00:00.000Z",
    ] {
        for field in ["evaluatedAt", "validFrom", "validUntil"] {
            let mut changed = overlay.clone();
            let clock = if field == "evaluatedAt" {
                value
            } else {
                changed[field] = json!(value);
                now
            };
            let result = index.resolve(&changed, clock).unwrap();
            assert!(!result.valid, "{field}: {value}");
            assert_eq!(result.issues[0].code, "invalid-time");
            assert!(result.closed_edge_ids.is_empty());
        }
    }
    assert!(index.resolve(&overlay, now).unwrap().valid);
}

#[test]
fn closure_expansion_and_count_limits_fail_without_partial_resolution() {
    let package = LegacyPolicyPackage {
        building_id: "fixture".into(),
        package_hash: "a".repeat(64),
        edges: (0..2_000)
            .map(|i| LegacyPolicyEdge {
                id: format!("edge-{i}"),
                source_id: "lift".into(),
            })
            .collect(),
    };
    let index = LegacyPolicyIndex::new(package).unwrap();
    let mut overlay = corpus()["groups"][0]["cases"][0]["overlay"].clone();
    overlay["closures"] = json!((0..501).map(|i| json!({
        "id": format!("close-{i}"), "target": { "type": "source", "id": "lift" }, "reason": "Fixture"
    })).collect::<Vec<_>>());
    let now = "2026-07-22T12:00:00.000Z";
    assert_eq!(
        index.resolve(&overlay, now),
        Err(LegacyPolicyError::WorkLimitExceeded)
    );
    overlay["closures"] = json!([overlay["closures"][0]]);
    let result = index.resolve(&overlay, now).unwrap();
    assert!(result.valid);
    assert_eq!(result.closed_edge_ids.len(), 2_000);
    overlay["closures"] = Value::Array(vec![Value::Null; MAX_EDGES + 1]);
    assert_eq!(
        index.resolve(&overlay, now),
        Err(LegacyPolicyError::WorkLimitExceeded)
    );
}
