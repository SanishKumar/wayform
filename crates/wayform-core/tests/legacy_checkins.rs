use serde_json::{Value, json};
use wayform_core::{LegacyVenuePackage, canonical_json, legacy_package_content_hash};

fn package(venue: &str) -> LegacyVenuePackage {
    let file = format!(
        "{}/../../compat/voicegis/upstream/buildings/{venue}/compiled/building.package.json",
        env!("CARGO_MANIFEST_DIR")
    );
    LegacyVenuePackage::from_json(&std::fs::read(file).unwrap()).unwrap()
}
fn load_changed(mut value: Value) -> LegacyVenuePackage {
    value["manifest"]["contentHash"] = json!(legacy_package_content_hash(&value).unwrap());
    LegacyVenuePackage::from_json(&serde_json::to_vec(&value).unwrap()).unwrap()
}

#[test]
fn all_pinned_anchor_payloads_and_whitespace_match_the_reference() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/checkins.json"
    ))
    .unwrap();
    let mut loaded = std::collections::HashMap::new();
    for case in cases.as_array().unwrap() {
        let venue = case["venue"].as_str().unwrap();
        let pkg = loaded.entry(venue).or_insert_with(|| package(venue));
        let actual = pkg.check_in(case["payload"].as_str().unwrap()).unwrap();
        assert_eq!(
            canonical_json(&actual).unwrap(),
            canonical_json(&case["expected"]).unwrap(),
            "{}",
            case["id"]
        );
    }
}

#[test]
fn declared_non_qr_anchors_cannot_be_used_as_qr_codes() {
    let mut value = package("asterion-medical-center").document().clone();
    let payload = value["localizationAnchors"][0]["payload"]
        .as_str()
        .unwrap()
        .to_owned();
    value["localizationAnchors"][0]["kind"] = json!("apriltag");
    let venue = load_changed(value);
    assert_eq!(
        venue.check_in(&payload).unwrap(),
        json!({"ok":false,"reason":"not-a-checkin-code"})
    );
}

#[test]
fn equal_distance_candidates_use_utf16_id_order_and_prefer_the_anchor_space() {
    let mut value = package("asterion-medical-center").document().clone();
    let anchor = value["localizationAnchors"][0].clone();
    for node in value["routing"]["nodes"].as_array_mut().unwrap() {
        if node["sourceId"] == anchor["spaceId"] {
            node["position"] = json!([1000, 1000]);
        }
    }
    let nodes = value["routing"]["nodes"].as_array_mut().unwrap();
    for id in ["\u{e000}", "\u{10000}"] {
        nodes.push(json!({"id":id,"kind":"waypoint","sourceId":anchor["spaceId"],"floorId":anchor["floorId"],"position":anchor["position"]}));
    }
    let venue = load_changed(value);
    let result = venue.check_in(anchor["payload"].as_str().unwrap()).unwrap();
    assert_eq!(result["nodeId"], "\u{10000}");
    assert_eq!(result["distanceMeters"].as_f64(), Some(0.0));
}

#[test]
fn an_anchor_without_nodes_on_its_floor_cannot_establish_a_start() {
    let mut value = package("asterion-medical-center").document().clone();
    let entry_space = value["building"]["entrySpaceId"].clone();
    let ground = value["spaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == entry_space)
        .unwrap()["floorId"]
        .clone();
    let anchor = value["localizationAnchors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["floorId"] != ground)
        .unwrap()
        .clone();
    let nodes = value["routing"]["nodes"].as_array_mut().unwrap();
    nodes.retain(|n| n["floorId"] != anchor["floorId"]);
    let remaining: std::collections::HashSet<_> = nodes
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_owned())
        .collect();
    value["routing"]["edges"]
        .as_array_mut()
        .unwrap()
        .retain(|e| {
            remaining.contains(e["from"].as_str().unwrap())
                && remaining.contains(e["to"].as_str().unwrap())
        });
    let venue = load_changed(value);
    assert_eq!(
        venue.check_in(anchor["payload"].as_str().unwrap()).unwrap(),
        json!({"ok":false,"reason":"no-node-on-floor"})
    );
}
