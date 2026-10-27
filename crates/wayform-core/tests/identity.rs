use serde_json::{Value, json};
use wayform_core::{IdentityError, bundle_digest, canonical_json, input_digests};

#[test]
fn portable_receipt_digests_match_the_original_typescript_examples() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json"
    ))
    .unwrap();
    for example in document["examples"].as_array().unwrap() {
        let actual =
            input_digests(&example["bundle"], &example["profile"], &example["context"]).unwrap();
        let receipt = &example["expected"]["receipt"];
        assert_eq!(actual.bundle_digest, receipt["bundleDigest"]);
        assert_eq!(actual.profile_digest, receipt["profileDigest"]);
        assert_eq!(actual.context_digest, receipt["contextDigest"]);
    }
}

#[test]
fn numeric_unicode_and_collection_cases_match_typescript() {
    let mut count = 0;
    for line in
        include_str!("../../../packages/mobility-contract/examples/identities-v0.1.0-draft.1.jsonl")
            .lines()
    {
        let mut case: Value = serde_json::from_str(line).unwrap();
        for _ in 0..2 {
            let actual =
                input_digests(&case["bundle"], &case["profile"], &case["context"]).unwrap();
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                case["expected"],
                "{}",
                case["id"]
            );
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
            case["profile"]["preferences"]["avoidKinds"]
                .as_array_mut()
                .unwrap()
                .reverse();
            case["profile"]["evidencePolicy"]["acceptedSources"]
                .as_array_mut()
                .unwrap()
                .reverse();
            case["context"]["trustedIssuerIds"]
                .as_array_mut()
                .unwrap()
                .reverse();
        }
        count += 1;
    }
    assert_eq!(count, 16);
}

#[test]
fn ecmascript_property_order_and_negative_zero_are_preserved() {
    let value: Value =
        serde_json::from_str(r#"{"10":10,"2":2,"01":1,"zero":-0,"integer":1.0}"#).unwrap();
    assert_eq!(
        canonical_json(&value).unwrap(),
        r#"{"2":2,"10":10,"01":1,"integer":1,"zero":0}"#
    );
    assert_eq!(canonical_json(&json!(1e21)).unwrap(), "1e+21");
    assert_eq!(canonical_json(&json!(1e-6)).unwrap(), "0.000001");
}

#[test]
fn malformed_identity_collections_and_excessive_depth_are_errors() {
    assert_eq!(
        bundle_digest(&json!({})),
        Err(IdentityError::InvalidStructure)
    );
    let mut value = Value::Null;
    for _ in 0..66 {
        value = Value::Array(vec![value]);
    }
    assert_eq!(canonical_json(&value), Err(IdentityError::ResourceLimit));
}
