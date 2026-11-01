use serde_json::{Value, json};
use wayform_core::{
    LegacyCompiledOptions, LegacyRouteError, LegacyVenuePackage, canonical_json,
    legacy_package_content_hash,
};

fn package(venue: &str) -> LegacyVenuePackage {
    let path = format!(
        "{}/../../compat/voicegis/upstream/buildings/{venue}/compiled/building.package.json",
        env!("CARGO_MANIFEST_DIR")
    );
    LegacyVenuePackage::from_json(&std::fs::read(path).unwrap()).unwrap()
}
fn comparable(mut value: Value) -> String {
    if let Some(steps) = value.get_mut("steps").and_then(Value::as_array_mut) {
        for step in steps {
            step["bearing"] = json!((step["bearing"].as_f64().unwrap() * 1e9).round() / 1e9);
        }
    }
    canonical_json(&value).unwrap()
}
fn overlay() -> Value {
    serde_json::from_str(include_str!("../../../compat/voicegis/upstream/buildings/asterion-medical-center/operations/all-public-lifts-closed.overlay.json")).unwrap()
}

#[test]
fn complete_representative_results_keep_paths_directions_clearance_and_receipts() {
    let examples: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/representative-routes.json"
    ))
    .unwrap();
    for (key, example) in examples.as_object().unwrap() {
        let venue = package(key.split('/').next().unwrap());
        let options = LegacyCompiledOptions {
            accessible_only: key.ends_with("/wheelchair"),
            ..Default::default()
        };
        let actual = venue
            .calculate_route(
                example["startId"].as_str().unwrap(),
                example["destinationId"].as_str().unwrap(),
                &options,
            )
            .unwrap();
        assert_eq!(
            comparable(actual),
            comparable(example["result"].clone()),
            "{key}"
        );
    }
}

#[test]
fn complete_policy_outcomes_match_the_original_twelve_cases() {
    let examples: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/policy-cases.json"
    ))
    .unwrap();
    let venue = package("asterion-medical-center");
    for (name, expected) in examples["cases"].as_object().unwrap() {
        let mut options = LegacyCompiledOptions::default();
        let mut start = "poi:poi-main-entrance";
        let mut destination = "poi:poi-cardiology";
        match name.as_str() {
            "standard" => {}
            "stepFree" => options.accessible_only = true,
            "unknownStart" => start = "missing",
            "unknownDestination" => destination = "missing",
            "sameStartAndDestination" => destination = start,
            other => {
                let mut value = overlay();
                options.evaluated_at = Some("2026-07-22T12:00:00.000Z".into());
                match other {
                    "liftOutageStandard" => {}
                    "liftOutageStepFree" => options.accessible_only = true,
                    "expired" => {
                        options.evaluated_at = Some(value["validUntil"].as_str().unwrap().into())
                    }
                    "notYetActive" => {
                        options.evaluated_at = Some("2026-07-22T07:59:59.999Z".into())
                    }
                    "missingClock" => options.evaluated_at = None,
                    "wrongPackage" => {
                        value["packageHash"] = json!("0".repeat(64));
                        options.evaluated_at = Some(value["validFrom"].as_str().unwrap().into());
                    }
                    "unknownClosureTarget" => {
                        value["closures"] = json!([{"id":"unknown","target":{"type":"edge","id":"missing"},"reason":"fixture"}]);
                        options.evaluated_at = Some(value["validFrom"].as_str().unwrap().into());
                    }
                    _ => panic!("unhandled fixture: {other}"),
                }
                options.operational_overlay = Some(value);
            }
        }
        let actual = venue.calculate_route(start, destination, &options).unwrap();
        assert_eq!(comparable(actual), comparable(expected.clone()), "{name}");
    }
    let value = overlay();
    let options = LegacyCompiledOptions {
        evaluated_at: Some(value["validUntil"].as_str().unwrap().into()),
        operational_overlay: Some(value),
        ..Default::default()
    };
    let rejected = venue
        .calculate_route("poi:poi-main-entrance", "poi:poi-main-entrance", &options)
        .unwrap();
    assert_eq!(rejected["found"], false);
    assert_eq!(rejected["receipt"]["status"], "rejected");
    assert!(rejected.get("steps").is_none());
}

#[test]
fn inconsistent_centerlines_cannot_return_guidance_but_narrow_graphics_keep_text() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/clearance.json"
    ))
    .unwrap();
    for case in cases["negative"].as_array().unwrap() {
        let original = package("asterion-medical-center");
        let mut document = original.document().clone();
        let mut target = &mut document;
        for key in case["path"].as_array().unwrap() {
            target = if let Some(i) = key.as_u64() {
                &mut target[i as usize]
            } else {
                &mut target[key.as_str().unwrap()]
            };
        }
        *target = case["value"].clone();
        document["manifest"]["contentHash"] =
            json!(legacy_package_content_hash(&document).unwrap());
        let venue = LegacyVenuePackage::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
        let options = LegacyCompiledOptions {
            accessible_only: true,
            ..Default::default()
        };
        let result = venue
            .calculate_route(
                case["startId"].as_str().unwrap(),
                case["destinationId"].as_str().unwrap(),
                &options,
            )
            .unwrap();
        assert_eq!(
            comparable(result["displayClearance"].clone()),
            comparable(case["expected"].clone())
        );
        let valid = case["expected"]["centerlineValid"].as_bool().unwrap();
        assert_eq!(result["found"], valid, "{}", case["id"]);
        assert_eq!(
            result["receipt"]["status"],
            if valid { "routed" } else { "rejected" }
        );
        assert_eq!(result.get("steps").is_some(), valid);
        assert_eq!(result.get("path").is_some(), valid);
    }
}

#[test]
fn unsupported_connector_collation_is_explicit() {
    let original = package("asterion-medical-center");
    let mut document = original.document().clone();
    let previous = "lift-south";
    let replacement = "lift_south";
    for connector in document["verticalConnectors"].as_array_mut().unwrap() {
        if connector["id"] == previous {
            connector["id"] = json!(replacement);
        }
    }
    for key in ["nodes", "edges"] {
        for item in document["routing"][key].as_array_mut().unwrap() {
            if item["sourceId"] == previous {
                item["sourceId"] = json!(replacement);
            }
        }
    }
    document["manifest"]["contentHash"] = json!(legacy_package_content_hash(&document).unwrap());
    let venue = LegacyVenuePackage::from_json(&serde_json::to_vec(&document).unwrap()).unwrap();
    let options = LegacyCompiledOptions {
        accessible_only: true,
        ..Default::default()
    };
    assert_eq!(
        venue
            .calculate_route("poi:poi-main-entrance", "poi:poi-cardiology", &options)
            .unwrap_err(),
        LegacyRouteError::UnsupportedConnectorId
    );
}
