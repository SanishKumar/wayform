use serde_json::Value;
use sha2::{Digest, Sha256};
use wayform_core::{LegacyRouteOptions, LegacyVenuePackage, canonical_json};

fn package(venue: &str) -> LegacyVenuePackage {
    let file = format!(
        "{}/../../compat/voicegis/upstream/buildings/{venue}/compiled/building.package.json",
        env!("CARGO_MANIFEST_DIR")
    );
    LegacyVenuePackage::from_json(&std::fs::read(file).unwrap()).unwrap()
}

#[test]
fn base_directions_match_every_original_journey() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/base-directions.json"
    ))
    .unwrap();
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
            let mut directions = package
                .route_directions(
                    query["startId"].as_str().unwrap(),
                    query["destinationId"].as_str().unwrap(),
                    &options,
                )
                .unwrap()
                .unwrap();
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
