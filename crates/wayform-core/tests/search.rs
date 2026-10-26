use wayform_core::{Edge, EdgeDecision, Graph, Path, SearchError, SearchOutcome, Suitability};

fn edge(id: &str, from: &str, to: &str, length: f64) -> Edge {
    Edge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        length_meters: length,
    }
}

fn decisions(edges: &[Edge]) -> Vec<EdgeDecision> {
    edges
        .iter()
        .map(|edge| EdgeDecision {
            edge_id: edge.id.clone(),
            suitability: Suitability::Suitable,
            penalty_meters: 0.0,
        })
        .collect()
}

fn graph(edges: Vec<Edge>) -> Graph {
    Graph::new(["a", "b", "c", "d"].map(String::from).to_vec(), edges).unwrap()
}

fn routed(ids: &[&str], distance: f64, cost: f64) -> SearchOutcome {
    SearchOutcome::Routed(Path {
        edge_ids: ids.iter().map(|id| (*id).into()).collect(),
        distance_meters: distance,
        cost_meters: cost,
    })
}

#[test]
fn shortest_path_respects_direction_and_keeps_parallel_arcs() {
    let edges = vec![
        edge("wide", "a", "b", 10.0),
        edge("narrow", "a", "b", 2.0),
        edge("bc", "b", "c", 3.0),
        edge("direct", "a", "c", 20.0),
    ];
    let mut assessment = decisions(&edges);
    assessment[1].suitability = Suitability::Unsuitable;
    let graph = graph(edges);
    assert_eq!(
        graph.route("a", "c", &assessment).unwrap(),
        routed(&["wide", "bc"], 13.0, 13.0)
    );
    assert_eq!(
        graph.route("c", "a", &assessment).unwrap(),
        SearchOutcome::Infeasible
    );
}

#[test]
fn uncertainty_is_diagnostic_and_never_an_eligible_route() {
    let edges = vec![edge("ab", "a", "b", 1.0), edge("bc", "b", "c", 1.0)];
    let mut assessment = decisions(&edges);
    let graph = graph(edges);
    assessment[1].suitability = Suitability::Unknown;
    assert_eq!(
        graph.route("a", "c", &assessment).unwrap(),
        SearchOutcome::InsufficientEvidence
    );
    assessment[0].suitability = Suitability::Unsuitable;
    assert_eq!(
        graph.route("a", "c", &assessment).unwrap(),
        SearchOutcome::Infeasible
    );
}

#[test]
fn an_eligible_detour_wins_over_a_shorter_unknown_path() {
    let edges = vec![
        edge("short", "a", "c", 1.0),
        edge("ab", "a", "b", 5.0),
        edge("bc", "b", "c", 5.0),
    ];
    let mut assessment = decisions(&edges);
    assessment[0].suitability = Suitability::Unknown;
    assert_eq!(
        graph(edges).route("a", "c", &assessment).unwrap(),
        routed(&["ab", "bc"], 10.0, 10.0)
    );
}

#[test]
fn preferences_rank_only_eligible_paths() {
    let edges = vec![
        edge("direct", "a", "c", 1.0),
        edge("ab", "a", "b", 3.0),
        edge("bc", "b", "c", 3.0),
    ];
    let mut assessment = decisions(&edges);
    let graph = graph(edges);
    assessment[0].penalty_meters = 10.0;
    assert_eq!(
        graph.route("a", "c", &assessment).unwrap(),
        routed(&["ab", "bc"], 6.0, 6.0)
    );
    assessment[1].suitability = Suitability::Unsuitable;
    assert_eq!(
        graph.route("a", "c", &assessment).unwrap(),
        routed(&["direct"], 1.0, 11.0)
    );
}

#[test]
fn ties_compare_complete_edge_sequences_not_last_edge_or_discovery_order() {
    let edges = vec![
        edge("z-first", "a", "b", 1.0),
        edge("a-last", "b", "d", 3.0),
        edge("a-first", "a", "c", 3.0),
        edge("z-last", "c", "d", 1.0),
    ];
    let assessment = decisions(&edges);
    assert_eq!(
        graph(edges).route("a", "d", &assessment).unwrap(),
        routed(&["a-first", "z-last"], 4.0, 4.0)
    );
}

#[test]
fn unicode_ties_use_utf16_order() {
    let edges = vec![
        edge("\u{e000}", "a", "b", 1.0),
        edge("\u{10000}", "a", "b", 1.0),
    ];
    let assessment = decisions(&edges);
    assert_eq!(
        graph(edges).route("a", "b", &assessment).unwrap(),
        routed(&["\u{10000}"], 1.0, 1.0)
    );
}

#[test]
fn cycles_and_obsolete_queue_labels_do_not_change_the_shortest_route() {
    let edges = vec![
        edge("slow", "a", "b", 8.0),
        edge("ac", "a", "c", 1.0),
        edge("cb", "c", "b", 1.0),
        edge("ba", "b", "a", 1.0),
        edge("bd", "b", "d", 10.0),
    ];
    let assessment = decisions(&edges);
    assert_eq!(
        graph(edges).route("a", "d", &assessment).unwrap(),
        routed(&["ac", "cb", "bd"], 12.0, 12.0)
    );
}

#[test]
fn missing_duplicate_and_foreign_decisions_are_errors() {
    let edges = vec![edge("ab", "a", "b", 1.0)];
    let assessment = decisions(&edges);
    let graph = graph(edges);
    assert_eq!(
        graph.route("a", "b", &[]).unwrap_err(),
        SearchError::MissingDecision("ab".into())
    );
    assert_eq!(
        graph
            .route("a", "b", &[assessment[0].clone(), assessment[0].clone()])
            .unwrap_err(),
        SearchError::DuplicateDecision("ab".into())
    );
    let mut invalid = assessment.clone();
    invalid[0].edge_id = "missing".into();
    assert_eq!(
        graph.route("a", "b", &invalid).unwrap_err(),
        SearchError::UnknownEdge("missing".into())
    );
    for penalty in [f64::NAN, f64::INFINITY, -1.0] {
        let mut invalid = assessment.clone();
        invalid[0].penalty_meters = penalty;
        assert_eq!(
            graph.route("a", "b", &invalid).unwrap_err(),
            SearchError::InvalidPenalty("ab".into())
        );
    }
}

#[test]
fn stationary_routes_require_valid_endpoints_and_complete_decisions() {
    let edges = vec![edge("ab", "a", "b", 1.0)];
    let assessment = decisions(&edges);
    let graph = graph(edges);
    assert_eq!(
        graph.route("a", "a", &assessment).unwrap(),
        routed(&[], 0.0, 0.0)
    );
    assert_eq!(
        graph.route("missing", "missing", &assessment).unwrap_err(),
        SearchError::UnknownNode("missing".into())
    );
    assert_eq!(
        graph.route("a", "a", &[]).unwrap_err(),
        SearchError::MissingDecision("ab".into())
    );
}

#[test]
fn overflowing_or_nonadvancing_costs_are_rejected() {
    for (first, second) in [(1e308, 1e308), (1e20, 1.0)] {
        let edges = vec![edge("ab", "a", "b", first), edge("bc", "b", "c", second)];
        let assessment = decisions(&edges);
        assert_eq!(
            graph(edges).route("a", "c", &assessment).unwrap_err(),
            SearchError::UnrepresentableCost("bc".into())
        );
    }
}

#[test]
fn path_work_is_bounded_and_does_not_report_infeasibility_on_exhaustion() {
    let nodes: Vec<_> = (0..1100).map(|index| index.to_string()).collect();
    let edges: Vec<_> = (0..1099)
        .map(|index| {
            edge(
                &format!("e{index}"),
                &index.to_string(),
                &(index + 1).to_string(),
                1.0,
            )
        })
        .collect();
    let assessment = decisions(&edges);
    assert_eq!(
        Graph::new(nodes, edges)
            .unwrap()
            .route("0", "1099", &assessment)
            .unwrap_err(),
        SearchError::WorkLimitExceeded
    );
}

#[test]
fn updated_decisions_do_not_leak_between_queries() {
    let edges = vec![edge("lift", "a", "b", 1.0)];
    let mut assessment = decisions(&edges);
    let graph = graph(edges);
    assert!(matches!(
        graph.route("a", "b", &assessment).unwrap(),
        SearchOutcome::Routed(_)
    ));
    assessment[0].suitability = Suitability::Unsuitable;
    assert_eq!(
        graph.route("a", "b", &assessment).unwrap(),
        SearchOutcome::Infeasible
    );
    assessment[0].suitability = Suitability::Unknown;
    assert_eq!(
        graph.route("a", "b", &assessment).unwrap(),
        SearchOutcome::InsufficientEvidence
    );
}
