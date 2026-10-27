use serde_json::{Value, json};
use wayform_core::{Document, LoadError, MAX_DOCUMENT_BYTES, MobilitySnapshot, canonical_json};

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/loading-v0.1.0-draft.1.json"
    ))
    .unwrap()
}
fn load(data: &Value) -> Result<MobilitySnapshot, LoadError> {
    MobilitySnapshot::from_json(
        &serde_json::to_vec(&data["bundle"]).unwrap(),
        &serde_json::to_vec(&data["profile"]).unwrap(),
        &serde_json::to_vec(&data["context"]).unwrap(),
    )
}
fn edited(base: &Value, edits: &Value) -> Value {
    let mut data = base.clone();
    for edit in edits.as_array().unwrap() {
        let path = edit["path"].as_array().unwrap();
        let mut parent = &mut data;
        for key in &path[..path.len() - 1] {
            parent = if let Some(index) = key.as_u64() {
                &mut parent[index as usize]
            } else {
                &mut parent[key.as_str().unwrap()]
            };
        }
        let key = path.last().unwrap();
        if edit["op"] == "remove" {
            parent
                .as_object_mut()
                .unwrap()
                .remove(key.as_str().unwrap());
        } else if let Some(index) = key.as_u64() {
            parent[index as usize] = edit["value"].clone();
        } else {
            parent[key.as_str().unwrap()] = edit["value"].clone();
        }
    }
    data
}

#[test]
fn full_document_acceptance_matches_typescript() {
    let corpus = corpus();
    let cases = corpus["cases"].as_array().unwrap();
    assert!(cases.len() > 200);
    for case in cases {
        let result = load(&edited(&corpus["base"], &case["edits"]));
        assert_eq!(
            result.is_ok(),
            case["valid"].as_bool().unwrap(),
            "{}: {result:?}",
            case["id"]
        );
        if let Ok(snapshot) = result {
            assert_eq!(
                serde_json::to_value(snapshot.input_digests()).unwrap(),
                case["expected"]["digests"],
                "{}",
                case["id"]
            );
            let edge = snapshot.assess_edge("ab").unwrap();
            assert_eq!(
                format!("{:?}", edge.status).to_lowercase(),
                case["expected"]["assessment"]["status"]
            );
            let reasons: Vec<_> = edge.reasons.iter().map(|reason| json!({
                "edgeId": edge.edge_id, "code": reason.code.code(),
                "attribute": reason.attribute.map(|value| value.name()), "evidenceIds": reason.evidence_ids,
            })).collect();
            assert_eq!(
                json!(reasons),
                case["expected"]["assessment"]["reasons"],
                "{}",
                case["id"]
            );
            assert_eq!(
                json!(edge.evidence_ids),
                case["expected"]["assessment"]["evidenceIds"]
            );
            assert_eq!(snapshot.decisions().unwrap().len(), 1);
            assert_eq!(
                canonical_json(
                    &serde_json::to_value(snapshot.assess_path("a", "b", &["ab".into()]).unwrap())
                        .unwrap()
                )
                .unwrap(),
                canonical_json(&case["expected"]["assessment"]).unwrap(),
                "{}",
                case["id"]
            );
        }
    }
}

#[test]
fn existing_portable_and_identity_inputs_load_without_adapters() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json"
    ))
    .unwrap();
    for case in document["examples"].as_array().unwrap() {
        load(case).unwrap();
    }
    for line in
        include_str!("../../../packages/mobility-contract/examples/identities-v0.1.0-draft.1.jsonl")
            .lines()
    {
        let case: Value = serde_json::from_str(line).unwrap();
        let snapshot = load(&case).unwrap();
        assert_eq!(
            serde_json::to_value(snapshot.input_digests()).unwrap(),
            case["expected"],
            "{}",
            case["id"]
        );
    }
}

#[test]
fn native_gate_rejects_ambiguous_json_and_bounds_input() {
    let data = &corpus()["base"];
    let bundle = serde_json::to_vec(&data["bundle"]).unwrap();
    let profile = serde_json::to_vec(&data["profile"]).unwrap();
    let context = serde_json::to_vec(&data["context"]).unwrap();
    for invalid in [
        r#"{"id":"one","id":"two"}"#,
        r#"{"nested":{"id":"one","\u0069d":"two"}}"#,
        r#"{"id":"\ud800"}"#,
        r#"{"number":1e400}"#,
        r#"{} {}"#,
    ] {
        assert!(
            matches!(
                MobilitySnapshot::from_json(invalid.as_bytes(), &profile, &context),
                Err(LoadError::InvalidJson(Document::Bundle))
            ),
            "{invalid}"
        );
    }
    assert!(matches!(
        MobilitySnapshot::from_json(&vec![b' '; MAX_DOCUMENT_BYTES + 1], &profile, &context),
        Err(LoadError::ResourceLimit)
    ));
    for invalid in [b"null".as_slice(), b"[]", b"42"] {
        assert!(MobilitySnapshot::from_json(&bundle, invalid, &context).is_err());
    }
    let nested = format!("{}0{}", "[".repeat(150), "]".repeat(150));
    assert!(MobilitySnapshot::from_json(nested.as_bytes(), &profile, &context).is_err());
    // Schema integers accept equivalent JSON number spellings, as JavaScript does.
    let context_decimal = String::from_utf8(context)
        .unwrap()
        .replace("\"revision\":3", "\"revision\":3.0")
        .replace("\"operations\":3", "\"operations\":3e0");
    assert!(MobilitySnapshot::from_json(&bundle, &profile, context_decimal.as_bytes()).is_ok());
}

#[test]
fn malformed_unused_records_still_reject_the_entire_snapshot() {
    let mut data = corpus()["base"].clone();
    let mut unused = data["bundle"]["edges"][0].clone();
    unused["id"] = json!("unused");
    unused["from"] = json!("c");
    unused["to"] = json!("a");
    unused["transformId"] = Value::Null;
    data["bundle"]["edges"].as_array_mut().unwrap().push(unused);
    let mut observation = data["bundle"]["observations"][0].clone();
    observation["id"] = json!("unused-measurement");
    observation["edgeId"] = json!("unused");
    observation["uncertainty"] = json!(-1);
    data["bundle"]["observations"]
        .as_array_mut()
        .unwrap()
        .push(observation);
    assert!(load(&data).is_err());
}
