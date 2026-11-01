use serde_json::{Value, json};
use wayform_core::{
    IdentityError, LegacyPackageError, LegacyRouteOptions, LegacySearchOutcome, LegacyVenuePackage,
    legacy_package_content_hash, parse_bounded_json,
};

const PACKAGES: [&[u8]; 4] = [
    include_bytes!(
        "../../../compat/voicegis/upstream/buildings/reference-medical-centre/compiled/building.package.json"
    ),
    include_bytes!(
        "../../../compat/voicegis/upstream/buildings/asterion-medical-center/compiled/building.package.json"
    ),
    include_bytes!(
        "../../../compat/voicegis/upstream/buildings/harbor-exchange/compiled/building.package.json"
    ),
    include_bytes!(
        "../../../compat/voicegis/upstream/buildings/meridian-park-campus/compiled/building.package.json"
    ),
];

#[test]
fn native_content_hashes_match_every_pinned_package() {
    for bytes in PACKAGES {
        let mut value = parse_bounded_json(bytes).unwrap();
        let expected = value["manifest"]["contentHash"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(legacy_package_content_hash(&value).unwrap(), expected);
        value["manifest"] = json!({"ignored": "any root manifest"});
        assert_eq!(legacy_package_content_hash(&value).unwrap(), expected);
        value["building"]["name"] = json!("Changed building");
        assert_ne!(legacy_package_content_hash(&value).unwrap(), expected);
    }
}

#[test]
fn numeric_string_and_key_ordering_matches_the_reference() {
    assert_eq!(
        legacy_package_content_hash(&serde_json::from_str::<Value>(r#"{"zero":-0.0}"#).unwrap()),
        legacy_package_content_hash(&json!({"zero":0}))
    );
    let cases: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/package-identities.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        assert_eq!(
            legacy_package_content_hash(&case["value"]).unwrap(),
            case["hash"].as_str().unwrap(),
            "{}",
            case["id"]
        );
    }
}

#[test]
fn unsupported_keys_and_excessive_identity_work_are_rejected() {
    for value in [
        Value::Null,
        json!([]),
        json!({"é": 1}),
        json!({"a_b": 2}),
        json!({"0": 3}),
    ] {
        assert_eq!(
            legacy_package_content_hash(&value),
            Err(IdentityError::InvalidStructure)
        );
    }
    let mut deep = json!(null);
    for _ in 0..66 {
        deep = json!({"child": deep});
    }
    assert_eq!(
        legacy_package_content_hash(&deep),
        Err(IdentityError::ResourceLimit)
    );
    let huge = json!({"label": "x".repeat(16 * 1024 * 1024)});
    assert_eq!(
        legacy_package_content_hash(&huge),
        Err(IdentityError::ResourceLimit)
    );
}

fn rehash(value: &mut Value) {
    value["manifest"]["contentHash"] = json!(legacy_package_content_hash(value).unwrap());
}

#[test]
fn loaded_packages_preserve_original_representative_paths_and_own_their_data() {
    let examples: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/representative-routes.json"
    ))
    .unwrap();
    let mut compared = 0;
    for bytes in PACKAGES {
        let mut source = bytes.to_vec();
        let venue = LegacyVenuePackage::from_json(&source).unwrap();
        source.fill(b' ');
        assert_eq!(venue.content_hash(), venue.policy_package().package_hash);
        venue.session_venue().unwrap();
        for (key, example) in examples.as_object().unwrap() {
            if !key.starts_with(&format!("{}/", venue.building_id())) {
                continue;
            }
            let options = LegacyRouteOptions {
                accessible_only: key.ends_with("/wheelchair"),
                ..Default::default()
            };
            let result = venue
                .route(
                    example["startId"].as_str().unwrap(),
                    example["destinationId"].as_str().unwrap(),
                    &options,
                )
                .unwrap();
            let LegacySearchOutcome::Routed {
                path_ids,
                total_distance,
            } = result
            else {
                panic!("{key}")
            };
            assert_eq!(json!(path_ids), example["result"]["pathIds"]);
            assert_eq!(
                total_distance,
                example["result"]["totalDistance"].as_f64().unwrap()
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 8);
}

#[test]
fn tampering_and_duplicate_json_keys_cannot_construct_a_verified_package() {
    let mut changed = parse_bounded_json(PACKAGES[0]).unwrap();
    changed["building"]["name"] = json!("Unpublished change");
    assert_eq!(
        LegacyVenuePackage::from_json(&serde_json::to_vec(&changed).unwrap()).unwrap_err(),
        LegacyPackageError::HashMismatch
    );
    let text = String::from_utf8(PACKAGES[0].to_vec()).unwrap();
    let duplicate = text.replacen(
        "\"packageVersion\":",
        "\"packageVersion\":\"0.2.0\",\"packageVersion\":",
        1,
    );
    assert!(matches!(
        LegacyVenuePackage::from_json(duplicate.as_bytes()),
        Err(LegacyPackageError::Json(_))
    ));
    for value in [
        json!(null),
        json!([]),
        json!({}),
        json!({"packageVersion":"9.0"}),
    ] {
        assert!(LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn a_valid_hash_does_not_override_invalid_shape_flags_or_references() {
    let original = parse_bounded_json(PACKAGES[0]).unwrap();
    let mutations = [
        ("/compilerVersion", json!("99")),
        ("/building/units", json!("feet")),
        ("/building/coordinateSystem/origin", json!([0, 0])),
        ("/floors/0/outline", json!([[0, 0], [1, 1]])),
        ("/floors/0/clearHeight", json!(0)),
        ("/spaces/0/accessible", json!(null)),
        ("/spaces/0/floorId", json!("missing")),
        ("/portals/0/connects", json!(["missing", "also-missing"])),
        ("/portals/0/width", json!(-1)),
        ("/verticalConnectors/0/stops/0", json!([])),
        ("/pois/0/public", json!("true")),
        ("/localizationAnchors/0/position", json!([0])),
        ("/routing/nodes/0/position", json!([null, 0])),
        ("/routing/nodes/0/sourceId", json!("missing")),
        ("/routing/nodes/0/kind", json!("unknown")),
        ("/routing/edges/0/accessible", json!(null)),
        ("/routing/edges/0/restricted", json!(null)),
        ("/routing/edges/0/from", json!("missing")),
        ("/routing/edges/0/distanceMeters", json!(0)),
        ("/routing/edges/0/floorIds", json!([])),
        ("/routing/edges/0/sourceId", json!("missing")),
        ("/manifest/hashAlgorithm", json!("sha1")),
    ];
    for (path, replacement) in mutations {
        let mut value = original.clone();
        *value.pointer_mut(path).unwrap() = replacement;
        rehash(&mut value);
        assert!(
            LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{path}"
        );
    }
    for key in ["accessible", "restricted"] {
        let mut value = original.clone();
        value["routing"]["edges"][0]
            .as_object_mut()
            .unwrap()
            .remove(key);
        rehash(&mut value);
        assert!(
            LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).is_err(),
            "missing {key}"
        );
    }
}

#[test]
fn duplicate_ids_payloads_and_unroutable_entry_points_are_rejected() {
    let original = parse_bounded_json(PACKAGES[0]).unwrap();
    for path in [
        "/floors",
        "/spaces",
        "/portals",
        "/verticalConnectors",
        "/pois",
        "/localizationAnchors",
        "/routing/nodes",
        "/routing/edges",
    ] {
        let mut value = original.clone();
        let items = value.pointer_mut(path).unwrap().as_array_mut().unwrap();
        items.push(items[0].clone());
        rehash(&mut value);
        assert!(
            LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{path}"
        );
    }
    let mut value = original.clone();
    let mut anchor = value["localizationAnchors"][0].clone();
    anchor["id"] = json!("another-anchor");
    value["localizationAnchors"]
        .as_array_mut()
        .unwrap()
        .push(anchor);
    rehash(&mut value);
    assert!(LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let mut value = original;
    for poi in value["pois"].as_array_mut().unwrap() {
        poi["public"] = json!(false);
    }
    rehash(&mut value);
    assert!(LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}
