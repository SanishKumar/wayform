use serde_json::{Value, json};
use wayform_core::legacy_package_content_hash;
use wayform_mobile::{MobileVenue, TripStopReason, mobile_contract_version, plan_mobility};

const PACKAGE: &[u8] = include_bytes!(
    "../../../compat/voicegis/upstream/buildings/asterion-medical-center/compiled/building.package.json"
);
const START: &str = "poi:poi-main-entrance";
const END: &str = "poi:poi-cardiology";
fn decode(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}
fn policy(venue: &MobileVenue) -> Vec<u8> {
    let catalog = decode(venue.catalog().unwrap());
    serde_json::to_vec(&json!({"schemaVersion":"0.1.0","id":"test-policy","buildingId":catalog["building"]["id"],"packageHash":catalog["packageHash"],"validFrom":"1970-01-01T00:00:00.000Z","validUntil":"1970-01-01T00:00:20.000Z","closures":[]})).unwrap()
}

#[test]
fn verified_catalog_checkin_and_preview_cross_the_mobile_boundary() {
    let venue = MobileVenue::new(PACKAGE.to_vec()).unwrap();
    let catalog = decode(venue.catalog().unwrap());
    assert_eq!(mobile_contract_version(), "0.1.0");
    assert!(!catalog["destinations"].as_array().unwrap().is_empty());
    let payload = catalog["anchors"][0]["payload"].as_str().unwrap();
    assert_eq!(decode(venue.check_in(payload.into()).unwrap())["ok"], true);
    assert_eq!(
        decode(venue.check_in("foreign-code".into()).unwrap())["ok"],
        false
    );
    assert!(venue.check_in("x".repeat(4097)).is_err());
    let route = decode(venue.preview(START.into(), END.into(), true).unwrap());
    assert_eq!(route["found"], true);
    assert_eq!(route["receipt"]["packageHash"], catalog["packageHash"]);
    assert!(MobileVenue::new(b"{\"building\":{},\"building\":{}}".to_vec()).is_err());
}

#[test]
fn expiry_and_invalidation_never_return_cached_guidance() {
    let venue = MobileVenue::new(PACKAGE.to_vec()).unwrap();
    for reason in [
        None,
        Some(TripStopReason::PolicyChanged),
        Some(TripStopReason::ContextChanged),
    ] {
        let trip = venue
            .start_trip(
                START.into(),
                END.into(),
                true,
                policy(&venue),
                10_000.0,
                100.0,
            )
            .unwrap();
        assert_eq!(
            decode(trip.read(10_001.0, 101.0).unwrap())["status"],
            "ready"
        );
        assert!(trip.next_check_delay(19_900.0, 10_000.0).unwrap() <= 100.0);
        let result = if let Some(reason) = reason {
            trip.invalidate(reason).unwrap();
            decode(trip.read(10_002.0, 102.0).unwrap())
        } else {
            decode(trip.read(20_000.0, 10_100.0).unwrap())
        };
        assert_ne!(result["status"], "ready");
        assert!(result.get("route").is_none());
        assert!(
            decode(trip.read(10_002.0, 102.0).unwrap())
                .get("route")
                .is_none()
        );
    }
}

#[test]
fn incorrectly_accessible_stairs_are_withheld() {
    let mut package: Value = serde_json::from_slice(PACKAGE).unwrap();
    for connector in package["verticalConnectors"].as_array_mut().unwrap() {
        if connector["kind"] == "elevator" {
            connector["kind"] = json!("stairs");
        }
    }
    for edge in package["routing"]["edges"].as_array_mut().unwrap() {
        if edge["connectorKind"] == "elevator" {
            edge["connectorKind"] = json!("stairs");
        }
    }
    package["manifest"]["contentHash"] = json!(legacy_package_content_hash(&package).unwrap());
    let venue = MobileVenue::new(serde_json::to_vec(&package).unwrap()).unwrap();
    assert!(venue.preview(START.into(), END.into(), true).is_err());
    let trip = venue
        .start_trip(
            START.into(),
            END.into(),
            true,
            policy(&venue),
            10_000.0,
            100.0,
        )
        .unwrap();
    assert!(trip.read(10_001.0, 101.0).is_err());
    assert_eq!(
        decode(trip.read(10_002.0, 102.0).unwrap())["status"],
        "invalidated"
    );
}

#[test]
fn mobility_snapshots_preserve_unknown_evidence() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/loading-v0.1.0-draft.1.json"
    ))
    .unwrap();
    let mut base = corpus["base"].clone();
    base["bundle"]["observations"] = json!([]);
    let result = decode(
        plan_mobility(
            serde_json::to_vec(&base["bundle"]).unwrap(),
            serde_json::to_vec(&base["profile"]).unwrap(),
            serde_json::to_vec(&base["context"]).unwrap(),
            "a".into(),
            "b".into(),
        )
        .unwrap(),
    );
    assert_eq!(result["outcome"], "insufficient-evidence");
    assert!(result.get("assessment").is_none());
}
