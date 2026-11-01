use serde_json::{Value, json};
use wayform_core::{IdentityError, legacy_package_content_hash, parse_bounded_json};

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
