use serde_json::{Value, json};
use wayform_core::{
    LegacyEdge, LegacyFloor, LegacyGraph, LegacyNode, LegacyRouteOptions, LegacySearchError,
    canonical_json,
};

#[test]
fn legacy_queue_paths_and_heuristic_scales_match_the_pinned_reference() {
    let mut count = 0;
    for line in include_str!("../../../compat/voicegis/expected/search-kernel.jsonl").lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        let graph = LegacyGraph::new(
            serde_json::from_value(case["nodes"].clone()).unwrap(),
            serde_json::from_value(case["edges"].clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            graph.heuristic_scale(),
            case["heuristicScale"].as_f64().unwrap(),
            "{}",
            case["id"]
        );
        let options = serde_json::from_value(case["request"]["options"].clone()).unwrap();
        let result = graph
            .route(
                case["request"]["startId"].as_str().unwrap(),
                case["request"]["destinationId"].as_str().unwrap(),
                &options,
            )
            .unwrap();
        assert_eq!(
            canonical_json(&serde_json::to_value(result).unwrap()).unwrap(),
            canonical_json(&case["expected"]).unwrap(),
            "{}",
            case["id"]
        );
        count += 1;
    }
    assert_eq!(count, 26);
}

fn node(id: &str, x: f64) -> LegacyNode {
    LegacyNode {
        id: id.into(),
        x,
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

#[test]
fn invalid_projected_inputs_do_not_inherit_low_level_fallbacks() {
    assert!(LegacyGraph::new(vec![], vec![]).is_err());
    assert!(LegacyGraph::new(vec![node("a", 0.0), node("a", 1.0)], vec![]).is_err());
    assert!(LegacyGraph::new(vec![node("a", f64::NAN)], vec![]).is_err());
    for distance in [-1.0, 0.0, f64::INFINITY, f64::NAN] {
        assert!(
            LegacyGraph::new(
                vec![node("a", 0.0), node("b", 1.0)],
                vec![edge("ab", "a", "b", distance)]
            )
            .is_err()
        );
    }
    assert!(LegacyGraph::new(vec![node("a", 0.0)], vec![edge("ab", "a", "absent", 1.0)]).is_err());
    assert!(
        LegacyGraph::new(
            vec![node("a", 0.0), node("b", 1.0)],
            vec![edge("ab", "a", "b", 1.0), edge("ab", "b", "a", 1.0)]
        )
        .is_err()
    );
    let missing_flag = json!({"id":"ab", "from":"a", "to":"b", "distance":1, "restricted":false});
    assert!(serde_json::from_value::<LegacyEdge>(missing_flag).is_err());
    let graph = LegacyGraph::new(vec![node("a", 0.0)], vec![]).unwrap();
    assert_eq!(
        graph.route(
            "a",
            "a",
            &LegacyRouteOptions {
                closed_edge_ids: vec!["absent".into()],
                ..Default::default()
            }
        ),
        Err(LegacySearchError::InvalidClosure("absent".into()))
    );
}

#[test]
fn numeric_failures_are_explicit_errors_instead_of_no_route_claims() {
    let graph = LegacyGraph::new(
        vec![node("a", 0.0), node("b", 0.0), node("c", 0.0)],
        vec![
            edge("ab", "a", "b", 2.0_f64.powi(54)),
            edge("bc", "b", "c", 1.0),
        ],
    )
    .unwrap();
    assert_eq!(
        graph.route("a", "c", &Default::default()),
        Err(LegacySearchError::UnrepresentableCost)
    );
    assert!(matches!(
        LegacyGraph::new(
            vec![node("a", -f64::MAX), node("b", f64::MAX)],
            vec![edge("ab", "a", "b", 1.0)]
        ),
        Err(LegacySearchError::UnrepresentableCost)
    ));
}
