use serde_json::Value;
use sha2::{Digest, Sha256};
use wayform_core::{
    LegacyRouteOptions, LegacySearchError, LegacyVenuePackage, canonical_json,
    legacy_package_content_hash,
};

fn package(venue: &str) -> LegacyVenuePackage {
    let file = format!(
        "{}/../../compat/voicegis/upstream/buildings/{venue}/compiled/building.package.json",
        env!("CARGO_MANIFEST_DIR")
    );
    LegacyVenuePackage::from_json(&std::fs::read(file).unwrap()).unwrap()
}

#[test]
fn base_directions_match_every_original_journey() {
    check_directions(false);
}

#[test]
fn landmark_directions_match_every_original_journey() {
    check_directions(true);
}

fn check_directions(with_landmarks: bool) {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/base-directions.json"
    ))
    .unwrap();
    let cases: Value = if with_landmarks {
        serde_json::from_str(include_str!(
            "../../../compat/voicegis/expected/landmark-directions.json"
        ))
        .unwrap()
    } else {
        cases
    };
    let queries: Vec<Value> = include_str!("../../../compat/voicegis/expected/route-digests.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for case in cases.as_array().unwrap() {
        let venue = case["venue"].as_str().unwrap();
        let package = package(venue);
        let mut digest = Sha256::new();
        let mut journeys = 0;
        for query in queries
            .iter()
            .filter(|q| q["id"].as_str().unwrap().starts_with(&format!("{venue}/")))
        {
            let options = LegacyRouteOptions {
                accessible_only: query["profile"] == "wheelchair",
                ..Default::default()
            };
            let start = query["startId"].as_str().unwrap();
            let destination = query["destinationId"].as_str().unwrap();
            let result = if with_landmarks {
                package.route_landmark_directions(start, destination, &options)
            } else {
                package.route_directions(start, destination, &options)
            };
            let mut directions = result.unwrap().unwrap();
            for step in &mut directions.steps {
                step.bearing = (step.bearing * 1e9).round() / 1e9;
            }
            digest.update(
                canonical_json(&serde_json::to_value(directions).unwrap())
                    .unwrap()
                    .as_bytes(),
            );
            digest.update(b"\n");
            journeys += 1;
        }
        let actual: String = digest
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, case["sha256"].as_str().unwrap(), "{venue}");
        assert_eq!(journeys, case["journeys"].as_u64().unwrap());
    }
}

#[test]
fn stationary_invalid_and_closed_paths_do_not_invent_directions() {
    let package = package("asterion-medical-center");
    let start = package.runtime().default_start_node_id();
    let same = package
        .route_directions(start, start, &LegacyRouteOptions::default())
        .unwrap()
        .unwrap();
    assert_eq!(same.path.len(), 1);
    assert_eq!(same.steps.len(), 1);
    assert_eq!(same.steps[0].r#type, "arrive");
    assert!(same.steps[0].instruction.starts_with("You are already at "));
    assert_eq!(same.steps[0].distance, 0.0);
    assert!(
        package
            .route_directions(start, "missing", &LegacyRouteOptions::default())
            .unwrap()
            .is_none()
    );
    let options = LegacyRouteOptions {
        closed_edge_ids: package
            .runtime()
            .edges()
            .iter()
            .map(|e| e.id.clone())
            .collect(),
        ..Default::default()
    };
    assert!(
        package
            .route_directions(start, "poi:poi-cardiology", &options)
            .unwrap()
            .is_none()
    );
}

#[test]
fn excessive_landmark_geometry_is_rejected_before_comparison_work() {
    let original = package("asterion-medical-center");
    let mut document = original.document().clone();
    for space in document["spaces"].as_array_mut().unwrap() {
        let polygon = space["polygon"].as_array().unwrap().clone();
        space["polygon"] =
            serde_json::json!((0..1000).flat_map(|_| polygon.clone()).collect::<Vec<_>>());
    }
    document["manifest"]["contentHash"] =
        serde_json::json!(legacy_package_content_hash(&document).unwrap());
    let expanded = LegacyVenuePackage::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
    let result = expanded.route_landmark_directions(
        expanded.runtime().default_start_node_id(),
        "poi:poi-cardiology",
        &LegacyRouteOptions::default(),
    );
    assert_eq!(result.unwrap_err(), LegacySearchError::WorkLimitExceeded);
}
