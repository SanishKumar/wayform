use serde_json::{Value, json};
use std::sync::Arc;
use wayform_core::{
    LegacyCompiledOptions, LegacyGuidanceError, LegacyGuidanceRead, LegacyGuidanceSession,
    LegacySessionInvalidation, LegacySessionRequest, LegacyVenuePackage,
    legacy_package_content_hash,
};

fn venue() -> Arc<LegacyVenuePackage> {
    Arc::new(LegacyVenuePackage::from_json(include_bytes!("../../../compat/voicegis/upstream/buildings/asterion-medical-center/compiled/building.package.json")).unwrap())
}
fn request() -> LegacySessionRequest {
    LegacySessionRequest {
        start_id: "poi:poi-main-entrance".into(),
        destination_id: "poi:poi-cardiology".into(),
        accessible_only: true,
        allow_restricted: false,
    }
}
fn overlay(venue: &LegacyVenuePackage) -> Value {
    json!({"schemaVersion":"0.1.0","id":"sample-conditions","buildingId":venue.building_id(),"packageHash":venue.content_hash(),"validFrom":"1970-01-01T00:00:00.000Z","validUntil":"1970-01-01T00:00:20.000Z","closures":[]})
}
fn session() -> LegacyGuidanceSession {
    let venue = venue();
    let policy = overlay(&venue);
    LegacyGuidanceSession::new(venue, request(), policy, 10_000.0, 100.0).unwrap()
}

#[test]
fn current_reads_return_the_complete_result_bound_to_the_session_policy() {
    let venue = venue();
    let policy = overlay(&venue);
    let request = request();
    let expected = venue
        .calculate_route(
            &request.start_id,
            &request.destination_id,
            &LegacyCompiledOptions {
                accessible_only: true,
                operational_overlay: Some(policy.clone()),
                evaluated_at: Some("1970-01-01T00:00:10.000Z".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let mut session = LegacyGuidanceSession::new(venue, request, policy, 10_000.0, 100.0).unwrap();
    let LegacyGuidanceRead::Ready(result) = session.read(10_001.0, 101.0).unwrap() else {
        panic!("current session must expose its result")
    };
    assert_eq!(result, &expected);
    assert_eq!(
        result["receipt"]["operationalOverlayId"],
        "sample-conditions"
    );
    assert!(!format!("{session:?}").contains("poi:"));
    assert_eq!(session.next_check_delay(19_900.0, 10_000.0), 100.0);
}

#[test]
fn expiry_during_planning_or_guidance_permanently_withdraws_the_result() {
    for (wall, monotonic) in [(20_000.0, 10_100.0), (10_000.0, 10_100.0)] {
        let mut session = session();
        assert!(matches!(
            session.read(wall, monotonic).unwrap(),
            LegacyGuidanceRead::PolicyUnavailable(_)
        ));
        assert!(matches!(
            session.read(10_000.0, 100.0).unwrap(),
            LegacyGuidanceRead::PolicyUnavailable(_)
        ));
    }
}

#[test]
fn clock_failures_cannot_restore_old_text_or_graphics() {
    for (wall, monotonic) in [
        (8_999.0, 101.0),
        (10_000.0, 99.0),
        (f64::NAN, 101.0),
        (10_000.0, f64::INFINITY),
    ] {
        let mut session = session();
        assert!(matches!(
            session.read(wall, monotonic).unwrap(),
            LegacyGuidanceRead::PolicyUnavailable(_)
        ));
        assert!(matches!(
            session.read(10_000.0, 102.0).unwrap(),
            LegacyGuidanceRead::PolicyUnavailable(_)
        ));
    }
}

#[test]
fn cancellation_and_context_changes_remove_the_entire_result() {
    for reason in [
        LegacySessionInvalidation::Cancelled,
        LegacySessionInvalidation::PolicyChanged,
        LegacySessionInvalidation::PolicyRemoved,
        LegacySessionInvalidation::RouteContextChanged,
    ] {
        let mut session = session();
        session.invalidate(reason);
        session.invalidate(LegacySessionInvalidation::Cancelled);
        assert!(
            matches!(session.read(10_001.0,101.0).unwrap(),LegacyGuidanceRead::Invalidated(value) if value==reason)
        );
    }
}

#[test]
fn invalid_policy_and_stationary_expiry_never_create_a_guidance_handle() {
    let venue = venue();
    for policy in [Value::Null, overlay(&venue)] {
        let mut request = request();
        request.destination_id = request.start_id.clone();
        assert!(matches!(
            LegacyGuidanceSession::new(venue.clone(), request, policy, 20_000.0, 100.0),
            Err(LegacyGuidanceError::PolicyUnavailable(_))
        ));
    }
}

#[test]
fn missing_paths_and_inconsistent_geometry_never_create_a_guidance_handle() {
    let venue = venue();
    let mut closed = overlay(&venue);
    closed["closures"] = json!(
        venue
            .runtime()
            .edges()
            .iter()
            .map(|e| json!({"id":e.id,"target":{"type":"edge","id":e.id},"reason":"test closure"}))
            .collect::<Vec<_>>()
    );
    let Err(LegacyGuidanceError::RouteUnavailable(result)) =
        LegacyGuidanceSession::new(venue.clone(), request(), closed, 10_000.0, 100.0)
    else {
        panic!("closed route must fail")
    };
    assert_eq!(result["receipt"]["status"], "unroutable");
    assert!(result.get("steps").is_none());
    let mut document = venue.document().clone();
    let space_id = document["pois"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "poi-cardiology")
        .unwrap()["spaceId"]
        .clone();
    let space = document["spaces"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["id"] == space_id)
        .unwrap();
    space["polygon"] = json!([[1000, 1000], [1010, 1000], [1010, 1010], [1000, 1010]]);
    document["manifest"]["contentHash"] = json!(legacy_package_content_hash(&document).unwrap());
    let changed =
        Arc::new(LegacyVenuePackage::from_json(&serde_json::to_vec(&document).unwrap()).unwrap());
    let policy = overlay(&changed);
    let Err(LegacyGuidanceError::RouteUnavailable(result)) =
        LegacyGuidanceSession::new(changed, request(), policy, 10_000.0, 100.0)
    else {
        panic!("invalid centerline must fail")
    };
    assert_eq!(result["receipt"]["status"], "rejected");
    assert!(result.get("steps").is_none());
    assert!(result.get("path").is_none());
}
