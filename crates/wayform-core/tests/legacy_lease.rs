use std::sync::Arc;

use serde_json::{Value, json};
use wayform_core::{
    LEGACY_SOURCE_COMMIT, LegacyFreshnessStatus, LegacyOperationalLease, LegacyPolicyEdge,
    LegacyPolicyError, LegacyPolicyIndex, LegacyPolicyPackage, MAX_EDGES,
};

fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../compat/voicegis/expected/operational-leases.json"
    ))
    .unwrap()
}

fn clock(value: &Value) -> f64 {
    if let Some(value) = value.as_f64() {
        return value;
    }
    match value.as_str().unwrap() {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        other => panic!("unknown fixture clock: {other}"),
    }
}

#[test]
fn clock_traces_match_pinned_lease_states_and_delay_values() {
    let corpus = corpus();
    assert_eq!(corpus["sourceCommit"], LEGACY_SOURCE_COMMIT);
    let (mut cases, mut events) = (0, 0);
    for group in corpus["groups"].as_array().unwrap() {
        let index = Arc::new(
            LegacyPolicyIndex::new(serde_json::from_value(group["package"].clone()).unwrap())
                .unwrap(),
        );
        for case in group["cases"].as_array().unwrap() {
            let mut lease = LegacyOperationalLease::new(
                index.clone(),
                case["overlay"].clone(),
                clock(&case["initial"]["wallMs"]),
                clock(&case["initial"]["monotonicMs"]),
            )
            .unwrap();
            let actual_events = case["events"].as_array().unwrap();
            let expected = case["expected"].as_array().unwrap();
            assert_eq!(actual_events.len(), expected.len());
            let mut latched = false;
            for (event, expected) in actual_events.iter().zip(expected) {
                let wall = clock(&event["wallMs"]);
                let monotonic = clock(&event["monotonicMs"]);
                match event["operation"].as_str().unwrap() {
                    "read" => {
                        let actual = lease.read(wall, monotonic).unwrap();
                        if latched {
                            assert_eq!(actual.status, LegacyFreshnessStatus::Unavailable);
                        }
                        if matches!(actual.reason, Some("clock-unreliable" | "overlay-expired")) {
                            latched = true;
                        }
                        assert_eq!(
                            serde_json::to_value(actual).unwrap(),
                            *expected,
                            "{}/{} event {event}",
                            group["id"],
                            case["id"]
                        );
                    }
                    "delay" => {
                        let actual = lease.delay(wall, monotonic);
                        assert!(actual.is_finite() && (1.0..=1000.0).contains(&actual));
                        assert_eq!(
                            actual,
                            expected.as_f64().unwrap(),
                            "{}/{} event {event}",
                            group["id"],
                            case["id"]
                        );
                    }
                    other => panic!("unknown fixture operation: {other}"),
                }
                events += 1;
            }
            cases += 1;
        }
    }
    assert_eq!((cases, events), (105, 332));
}

fn fixture() -> (Arc<LegacyPolicyIndex>, Value, f64) {
    let corpus = corpus();
    let group = &corpus["groups"][0];
    let case = &group["cases"][0];
    (
        Arc::new(
            LegacyPolicyIndex::new(serde_json::from_value(group["package"].clone()).unwrap())
                .unwrap(),
        ),
        case["overlay"].clone(),
        clock(&case["initial"]["wallMs"]),
    )
}

#[test]
fn leases_own_overlay_data_and_do_not_share_clock_history() {
    let (index, mut source, wall) = fixture();
    let mut first =
        LegacyOperationalLease::new(index.clone(), source.clone(), wall, 100.0).unwrap();
    let mut second = LegacyOperationalLease::new(index, source.clone(), wall, 100.0).unwrap();
    source["validUntil"] = json!("2099-01-01T00:00:00.000Z");
    source["id"] = json!("changed");
    source["closures"] = json!([]);
    assert_eq!(
        first.read(wall, 100.0).unwrap().status,
        LegacyFreshnessStatus::Current
    );
    let expired = first.read(wall, 5100.0).unwrap();
    assert_eq!(expired.reason, Some("overlay-expired"));
    assert_eq!(expired.overlay_id, "short-outage");
    assert_eq!(
        first.read(wall + 1.0, 5101.0).unwrap().status,
        LegacyFreshnessStatus::Unavailable
    );
    assert_eq!(
        second.read(wall + 1.0, 101.0).unwrap().status,
        LegacyFreshnessStatus::Current
    );
}

#[test]
fn unsupported_clock_ranges_and_timestamp_forms_cannot_create_current_leases() {
    let (index, overlay, wall) = fixture();
    for value in [
        f64::MAX,
        -f64::MAX,
        -62_167_219_200_001.0,
        253_402_300_800_000.0,
    ] {
        let mut initial =
            LegacyOperationalLease::new(index.clone(), overlay.clone(), value, 0.0).unwrap();
        assert_eq!(
            initial.read(wall, 1.0).unwrap().reason,
            Some("clock-unreliable")
        );
        let mut later =
            LegacyOperationalLease::new(index.clone(), overlay.clone(), wall, 0.0).unwrap();
        assert_eq!(
            later.read(value, 1.0).unwrap().reason,
            Some("clock-unreliable")
        );
        assert_eq!(
            later.read(wall, 2.0).unwrap().reason,
            Some("clock-unreliable")
        );
    }
    for until in [
        "2026-07-22T12:00:05Z",
        "2026-07-22T12:00:05.000+00:00",
        "2026-02-30T12:00:05.000Z",
    ] {
        let mut changed = overlay.clone();
        changed["validUntil"] = json!(until);
        let mut lease = LegacyOperationalLease::new(index.clone(), changed, wall, 0.0).unwrap();
        assert_eq!(lease.read(wall, 0.0).unwrap().reason, Some("invalid-time"));
    }
}

#[test]
fn resource_failures_are_not_treated_as_policy_absence_or_retried_after_clock_changes() {
    let (_, mut overlay, wall) = fixture();
    let index = Arc::new(
        LegacyPolicyIndex::new(LegacyPolicyPackage {
            building_id: "fixture".into(),
            package_hash: "a".repeat(64),
            edges: (0..2000)
                .map(|i| LegacyPolicyEdge {
                    id: format!("edge-{i}"),
                    source_id: "lift".into(),
                })
                .collect(),
        })
        .unwrap(),
    );
    overlay["closures"] = json!((0..501).map(|i| json!({"id": format!("closure-{i}"), "target": {"type": "source", "id": "lift"}, "reason": "Fixture"})).collect::<Vec<_>>());
    let mut lease =
        LegacyOperationalLease::new(index.clone(), overlay.clone(), wall, 100.0).unwrap();
    for (offset, monotonic) in [
        (0.0, 100.0),
        (1000.0, 1100.0),
        (5000.0, 5100.0),
        (5001.0, 1.0),
    ] {
        assert_eq!(
            lease.read(wall + offset, monotonic),
            Err(LegacyPolicyError::WorkLimitExceeded)
        );
    }
    overlay["closures"] = Value::Array(vec![Value::Null; MAX_EDGES + 1]);
    assert!(matches!(
        LegacyOperationalLease::new(index, overlay, wall, 100.0),
        Err(LegacyPolicyError::WorkLimitExceeded)
    ));
}
