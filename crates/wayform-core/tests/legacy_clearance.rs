use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use wayform_core::{
    LegacyRouteOptions, LegacySearchError, LegacyVenuePackage, canonical_json,
    legacy_package_content_hash,
};

fn package(venue: &str) -> LegacyVenuePackage {
    let path = format!(
        "{}/../../compat/voicegis/upstream/buildings/{venue}/compiled/building.package.json",
        env!("CARGO_MANIFEST_DIR")
    );
    LegacyVenuePackage::from_json(&std::fs::read(path).unwrap()).unwrap()
}
fn fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/clearance.json"
    ))
    .unwrap()
}

#[test]
fn graphic_assessments_match_all_7238_original_journeys() {
    let cases = fixtures();
    let queries: Vec<Value> = include_str!("../../../compat/voicegis/expected/route-digests.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for case in cases["corpus"].as_array().unwrap() {
        let venue = case["venue"].as_str().unwrap();
        let package = package(venue);
        let mut digest = Sha256::new();
        let mut count = 0;
        for query in queries
            .iter()
            .filter(|q| q["id"].as_str().unwrap().starts_with(&format!("{venue}/")))
        {
            let options = LegacyRouteOptions {
                accessible_only: query["profile"] == "wheelchair",
                ..Default::default()
            };
            let clearance = package
                .route_display_clearance(
                    query["startId"].as_str().unwrap(),
                    query["destinationId"].as_str().unwrap(),
                    &options,
                )
                .unwrap()
                .unwrap();
            digest.update(
                canonical_json(&serde_json::to_value(clearance).unwrap())
                    .unwrap()
                    .as_bytes(),
            );
            digest.update(b"\n");
            count += 1;
        }
        let actual: String = digest
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(actual, case["sha256"].as_str().unwrap(), "{venue}");
        assert_eq!(count, case["journeys"].as_u64().unwrap());
    }
}

#[test]
fn altered_geometry_and_policies_retain_ordered_reference_failures() {
    for case in fixtures()["negative"].as_array().unwrap() {
        let original = package(case["venue"].as_str().unwrap());
        let mut document = original.document().clone();
        let keys = case["path"].as_array().unwrap();
        let mut target = &mut document;
        for key in keys {
            target = if let Some(i) = key.as_u64() {
                &mut target[i as usize]
            } else {
                &mut target[key.as_str().unwrap()]
            };
        }
        *target = case["value"].clone();
        document["manifest"]["contentHash"] =
            json!(legacy_package_content_hash(&document).unwrap());
        let changed =
            LegacyVenuePackage::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
        let options = LegacyRouteOptions {
            accessible_only: true,
            ..Default::default()
        };
        let actual = changed
            .route_display_clearance(
                case["startId"].as_str().unwrap(),
                case["destinationId"].as_str().unwrap(),
                &options,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            canonical_json(&serde_json::to_value(actual).unwrap()).unwrap(),
            canonical_json(&case["expected"]).unwrap(),
            "{}",
            case["id"]
        );
    }
}

#[test]
fn absent_routes_have_no_graphic_assessment() {
    let venue = package("asterion-medical-center");
    let start = venue.runtime().default_start_node_id();
    assert!(
        venue
            .route_display_clearance(start, "missing", &LegacyRouteOptions::default())
            .unwrap()
            .is_none()
    );
    let closed = LegacyRouteOptions {
        closed_edge_ids: venue
            .runtime()
            .edges()
            .iter()
            .map(|e| e.id.clone())
            .collect(),
        ..Default::default()
    };
    assert!(
        venue
            .route_display_clearance(start, "poi:poi-cardiology", &closed)
            .unwrap()
            .is_none()
    );
}

#[test]
fn excessive_geometry_and_unrepresentable_products_are_errors() {
    let original = package("asterion-medical-center");
    for oversized in [false, true] {
        let mut document = original.document().clone();
        if oversized {
            for space in document["spaces"].as_array_mut().unwrap() {
                let polygon = space["polygon"].as_array().unwrap().clone();
                space["polygon"] =
                    json!((0..1000).flat_map(|_| polygon.clone()).collect::<Vec<_>>());
            }
        } else {
            document["portals"][0]["width"] = json!(1e200);
        }
        document["manifest"]["contentHash"] =
            json!(legacy_package_content_hash(&document).unwrap());
        let changed =
            LegacyVenuePackage::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
        let error = changed
            .route_display_clearance(
                changed.runtime().default_start_node_id(),
                "poi:poi-cardiology",
                &LegacyRouteOptions::default(),
            )
            .unwrap_err();
        assert_eq!(
            error,
            if oversized {
                LegacySearchError::WorkLimitExceeded
            } else {
                LegacySearchError::UnrepresentableCost
            }
        );
    }
}
