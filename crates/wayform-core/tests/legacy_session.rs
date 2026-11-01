use std::sync::Arc;

use serde_json::{Value, json};
use wayform_core::{
    LegacyEdge, LegacyFloor, LegacyNode, LegacyPolicyEdge, LegacyPolicyError, LegacyPolicyPackage,
    LegacyRouteSession, LegacySearchError, LegacySessionError, LegacySessionInvalidation,
    LegacySessionRead, LegacySessionRequest, LegacySessionVenue, MAX_EDGES,
};

fn node(id: &str) -> LegacyNode {
    LegacyNode {
        id: id.into(),
        x: 0.0,
        y: 0.0,
        floor: LegacyFloor::Text("g".into()),
    }
}

fn edge(id: &str, from: &str, to: &str, distance: f64) -> LegacyEdge {
    LegacyEdge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        distance,
        accessible: true,
        restricted: false,
    }
}

fn package(edges: &[LegacyEdge]) -> LegacyPolicyPackage {
    LegacyPolicyPackage {
        building_id: "fixture".into(),
        package_hash: "a".repeat(64),
        edges: edges
            .iter()
            .map(|edge| LegacyPolicyEdge {
                id: edge.id.clone(),
                source_id: edge.id.clone(),
            })
            .collect(),
    }
}

fn overlay() -> Value {
    json!({
        "schemaVersion": "0.1.0", "id": "outage", "buildingId": "fixture", "packageHash": "a".repeat(64),
        "validFrom": "1970-01-01T00:00:00.000Z", "validUntil": "1970-01-01T00:00:20.000Z",
        "closures": [{"id": "outage", "target": {"type": "source", "id": "lift"}, "reason": "Maintenance"}]
    })
}

fn fixture() -> Arc<LegacySessionVenue> {
    let mut edges = vec![
        edge("lift", "a", "b", 1.0),
        edge("stairs-1", "a", "c", 1.0),
        edge("stairs-2", "c", "b", 1.0),
        edge("staff", "a", "b", 0.5),
    ];
    edges[1].accessible = false;
    edges[2].accessible = false;
    edges[3].restricted = true;
    let package = package(&edges);
    Arc::new(
        LegacySessionVenue::new(
            vec![node("a"), node("b"), node("c"), node("isolated")],
            edges,
            package,
        )
        .unwrap(),
    )
}

fn request() -> LegacySessionRequest {
    LegacySessionRequest {
        start_id: "a".into(),
        destination_id: "b".into(),
        accessible_only: false,
        allow_restricted: false,
    }
}

fn session() -> LegacyRouteSession {
    LegacyRouteSession::new(fixture(), request(), overlay(), 10_000.0, 100.0).unwrap()
}

fn ready(
    session: &mut LegacyRouteSession,
    wall: f64,
    monotonic: f64,
    path: &[&str],
    distance: f64,
) {
    let LegacySessionRead::Ready {
        path_ids,
        total_distance,
        overlay_id,
    } = session.read(wall, monotonic).unwrap()
    else {
        panic!("expected a current route")
    };
    assert_eq!(path_ids, path);
    assert_eq!(total_distance, distance);
    assert_eq!(overlay_id, "outage");
}

fn unavailable(session: &mut LegacyRouteSession, wall: f64, monotonic: f64, reason: &str) {
    let LegacySessionRead::PolicyUnavailable(freshness) = session.read(wall, monotonic).unwrap()
    else {
        panic!("expected withheld route")
    };
    assert_eq!(freshness.reason, Some(reason));
}

#[test]
fn resolved_source_closures_and_access_flags_determine_the_owned_route() {
    let venue = fixture();
    let mut normal =
        LegacyRouteSession::new(venue.clone(), request(), overlay(), 10_000.0, 100.0).unwrap();
    ready(&mut normal, 10_001.0, 101.0, &["a", "c", "b"], 2.0);
    let mut step_free = request();
    step_free.accessible_only = true;
    assert_eq!(
        LegacyRouteSession::new(venue.clone(), step_free, overlay(), 10_000.0, 100.0).unwrap_err(),
        LegacySessionError::NoRoute
    );
    let mut staff = request();
    staff.allow_restricted = true;
    let mut staff_session =
        LegacyRouteSession::new(venue, staff, overlay(), 10_000.0, 100.0).unwrap();
    ready(&mut staff_session, 10_001.0, 101.0, &["a", "b"], 0.5);
}

#[test]
fn planning_does_not_authorize_a_first_read_after_either_expiry_deadline() {
    for (wall, monotonic) in [(20_000.0, 101.0), (10_000.0, 10_100.0)] {
        let mut session = session();
        unavailable(&mut session, wall, monotonic, "overlay-expired");
        unavailable(&mut session, wall - 1.0, monotonic + 1.0, "overlay-expired");
    }
}

#[test]
fn an_active_route_is_withheld_permanently_after_expiry_or_clock_failure() {
    for (wall, monotonic, reason) in [
        (20_000.0, 1101.0, "overlay-expired"),
        (11_000.0, 10_100.0, "overlay-expired"),
        (9_999.0, 1101.0, "clock-unreliable"),
        (11_001.0, 1099.0, "clock-unreliable"),
        (f64::NAN, 1101.0, "clock-unreliable"),
        (11_001.0, f64::NAN, "clock-unreliable"),
    ] {
        let mut session = session();
        ready(&mut session, 11_000.0, 1100.0, &["a", "c", "b"], 2.0);
        unavailable(&mut session, wall, monotonic, reason);
        assert!(!matches!(
            session.read(12_000.0, 12_000.0).unwrap(),
            LegacySessionRead::Ready { .. }
        ));
    }
}

#[test]
fn explicit_invalidation_withdraws_the_route_and_preserves_the_first_reason() {
    for reason in [
        LegacySessionInvalidation::PolicyChanged,
        LegacySessionInvalidation::PolicyRemoved,
        LegacySessionInvalidation::RouteContextChanged,
        LegacySessionInvalidation::Cancelled,
    ] {
        let mut session = session();
        ready(&mut session, 10_001.0, 101.0, &["a", "c", "b"], 2.0);
        session.invalidate(reason);
        session.invalidate(LegacySessionInvalidation::Cancelled);
        for (wall, monotonic) in [
            (10_002.0, 102.0),
            (20_000.0, 10_100.0),
            (f64::NAN, f64::NAN),
        ] {
            assert_eq!(
                session.read(wall, monotonic).unwrap(),
                LegacySessionRead::Invalidated(reason)
            );
        }
    }
    let mut not_yet_read = session();
    not_yet_read.invalidate(LegacySessionInvalidation::PolicyRemoved);
    assert_eq!(
        not_yet_read.read(10_000.0, 100.0).unwrap(),
        LegacySessionRead::Invalidated(LegacySessionInvalidation::PolicyRemoved)
    );
}

#[test]
fn invalid_policy_blocks_planning_before_stationary_or_unknown_endpoint_handling() {
    let mut future = overlay();
    future["validFrom"] = json!("1970-01-01T00:00:11.000Z");
    let mut expired = overlay();
    expired["validUntil"] = json!("1970-01-01T00:00:10.000Z");
    let mut foreign = overlay();
    foreign["packageHash"] = json!("b".repeat(64));
    let mut unknown_target = overlay();
    unknown_target["closures"][0]["target"]["id"] = json!("missing");
    for policy in [Value::Null, future, expired, foreign, unknown_target] {
        for (start, destination) in [("a", "b"), ("a", "a"), ("missing", "b")] {
            let mut req = request();
            req.start_id = start.into();
            req.destination_id = destination.into();
            assert!(matches!(
                LegacyRouteSession::new(fixture(), req, policy.clone(), 10_000.0, 100.0),
                Err(LegacySessionError::PolicyUnavailable(_))
            ));
        }
    }
    for (wall, monotonic) in [(f64::NAN, 100.0), (10_000.0, f64::INFINITY)] {
        let Err(LegacySessionError::PolicyUnavailable(freshness)) =
            LegacyRouteSession::new(fixture(), request(), overlay(), wall, monotonic)
        else {
            panic!("invalid clocks created a session")
        };
        assert_eq!(freshness.reason, Some("clock-unreliable"));
    }
}

#[test]
fn stationary_routes_remain_gated_and_nonexistent_routes_do_not_create_sessions() {
    let mut stationary = request();
    stationary.destination_id = "a".into();
    let mut session =
        LegacyRouteSession::new(fixture(), stationary, overlay(), 10_000.0, 100.0).unwrap();
    ready(&mut session, 10_001.0, 101.0, &["a"], 0.0);
    unavailable(&mut session, 20_000.0, 10_100.0, "overlay-expired");
    for (destination, error) in [
        ("missing", LegacySessionError::UnknownEndpoint),
        ("isolated", LegacySessionError::NoRoute),
    ] {
        let mut req = request();
        req.destination_id = destination.into();
        assert_eq!(
            LegacyRouteSession::new(fixture(), req, overlay(), 10_000.0, 100.0).unwrap_err(),
            error
        );
    }
}

#[test]
fn matching_inventories_are_required_and_both_projections_are_validated() {
    let edges = vec![edge("lift", "a", "b", 1.0), edge("walk", "a", "b", 2.0)];
    for ids in [
        vec!["lift"],
        vec!["lift", "walk", "extra"],
        vec!["lift", "foreign"],
        vec!["lift", "lift"],
    ] {
        let mut package = package(&edges);
        package.edges = ids
            .iter()
            .map(|id| LegacyPolicyEdge {
                id: (*id).into(),
                source_id: (*id).into(),
            })
            .collect();
        assert!(
            LegacySessionVenue::new(vec![node("a"), node("b")], edges.clone(), package).is_err()
        );
    }
    let mut bad_package = package(&edges);
    bad_package.edges[0].source_id.clear();
    assert!(matches!(
        LegacySessionVenue::new(vec![node("a"), node("b")], edges.clone(), bad_package),
        Err(LegacySessionError::Policy(
            LegacyPolicyError::InvalidPackage
        ))
    ));
    let mut bad_edges = edges.clone();
    bad_edges[0].to = "missing".into();
    assert!(matches!(
        LegacySessionVenue::new(vec![node("a"), node("b")], bad_edges, package(&edges)),
        Err(LegacySessionError::Search(LegacySearchError::InvalidGraph))
    ));
}

#[test]
fn caller_changes_and_other_sessions_cannot_reset_an_existing_lease() {
    let venue = fixture();
    let mut source = overlay();
    let mut req = request();
    let mut first =
        LegacyRouteSession::new(venue.clone(), req.clone(), source.clone(), 10_000.0, 100.0)
            .unwrap();
    let mut second =
        LegacyRouteSession::new(venue.clone(), req.clone(), source.clone(), 10_000.0, 100.0)
            .unwrap();
    source["closures"] = json!([]);
    source["validUntil"] = json!("2099-01-01T00:00:00.000Z");
    req.allow_restricted = true;
    ready(&mut first, 10_001.0, 101.0, &["a", "c", "b"], 2.0);
    unavailable(&mut first, 10_001.0, 10_100.0, "overlay-expired");
    ready(&mut second, 10_001.0, 101.0, &["a", "c", "b"], 2.0);
    second.invalidate(LegacySessionInvalidation::PolicyChanged);
    let mut replacement = LegacyRouteSession::new(venue, req, source, 10_002.0, 102.0).unwrap();
    ready(&mut replacement, 10_003.0, 103.0, &["a", "b"], 0.5);
    assert_eq!(
        second.read(10_003.0, 103.0).unwrap(),
        LegacySessionRead::Invalidated(LegacySessionInvalidation::PolicyChanged)
    );
    unavailable(&mut first, 10_002.0, 10_101.0, "overlay-expired");
}

#[test]
fn a_failed_policy_replacement_does_not_restore_the_old_route() {
    let mut old = session();
    ready(&mut old, 10_001.0, 101.0, &["a", "c", "b"], 2.0);
    old.invalidate(LegacySessionInvalidation::PolicyChanged);
    let mut replacement = overlay();
    replacement["closures"][0]["target"]["id"] = json!("missing");
    assert!(matches!(
        LegacyRouteSession::new(fixture(), request(), replacement, 10_002.0, 102.0),
        Err(LegacySessionError::PolicyUnavailable(_))
    ));
    assert_eq!(
        old.read(10_003.0, 103.0).unwrap(),
        LegacySessionRead::Invalidated(LegacySessionInvalidation::PolicyChanged)
    );
}

#[test]
fn resource_and_numeric_failures_never_create_a_partially_usable_session() {
    let mut huge = overlay();
    huge["closures"] = Value::Array(vec![Value::Null; MAX_EDGES + 1]);
    assert_eq!(
        LegacyRouteSession::new(fixture(), request(), huge, 10_000.0, 100.0).unwrap_err(),
        LegacySessionError::Policy(LegacyPolicyError::WorkLimitExceeded)
    );
    let edges = vec![
        edge("ac", "a", "c", f64::MAX),
        edge("cb", "c", "b", f64::MAX),
    ];
    let venue = Arc::new(
        LegacySessionVenue::new(
            vec![node("a"), node("b"), node("c")],
            edges.clone(),
            package(&edges),
        )
        .unwrap(),
    );
    let mut policy = overlay();
    policy["closures"] = json!([]);
    assert_eq!(
        LegacyRouteSession::new(venue, request(), policy, 10_000.0, 100.0).unwrap_err(),
        LegacySessionError::Search(LegacySearchError::UnrepresentableCost)
    );
}

#[test]
fn session_routes_preserve_the_frozen_search_corpus() {
    let mut count = 0;
    for line in include_str!("../../../compat/voicegis/expected/search-kernel.jsonl").lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        let edges: Vec<LegacyEdge> = serde_json::from_value(case["edges"].clone()).unwrap();
        let venue = Arc::new(
            LegacySessionVenue::new(
                serde_json::from_value(case["nodes"].clone()).unwrap(),
                edges.clone(),
                package(&edges),
            )
            .unwrap(),
        );
        let options = &case["request"]["options"];
        let request = LegacySessionRequest {
            start_id: case["request"]["startId"].as_str().unwrap().into(),
            destination_id: case["request"]["destinationId"].as_str().unwrap().into(),
            accessible_only: options["accessibleOnly"].as_bool().unwrap(),
            allow_restricted: options["allowRestricted"].as_bool().unwrap(),
        };
        let mut policy = overlay();
        let closures: Vec<_> = options["closedEdgeIds"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, id)| {
                json!({
                    "id": format!("closure-{index}"),
                    "target": {"type": "edge", "id": id},
                    "reason": "Fixture"
                })
            })
            .collect();
        policy["closures"] = json!(closures);
        let actual = match LegacyRouteSession::new(venue, request, policy, 10_000.0, 100.0) {
            Ok(mut session) => {
                let LegacySessionRead::Ready {
                    path_ids,
                    total_distance,
                    ..
                } = session.read(10_001.0, 101.0).unwrap()
                else {
                    panic!("{}", case["id"])
                };
                json!({"outcome": "routed", "pathIds": path_ids, "totalDistance": total_distance})
            }
            Err(LegacySessionError::NoRoute) => json!({"outcome": "no-route"}),
            Err(LegacySessionError::UnknownEndpoint) => json!({"outcome": "unknown-endpoint"}),
            Err(error) => panic!("{}: {error}", case["id"]),
        };
        assert_eq!(
            wayform_core::canonical_json(&actual).unwrap(),
            wayform_core::canonical_json(&case["expected"]).unwrap(),
            "{}",
            case["id"]
        );
        count += 1;
    }
    assert_eq!(count, 26);
}
