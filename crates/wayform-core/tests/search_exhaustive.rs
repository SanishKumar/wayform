use wayform_core::{Edge, EdgeDecision, Graph, Path, SearchOutcome, Suitability};

// Enumerate simple paths independently of the production heap/search. Positive
// weights mean cycles cannot improve an optimum or change reachability.
fn all_paths(
    edges: &[Edge],
    decisions: &[EdgeDecision],
    start: &str,
    end: &str,
    include_unknown: bool,
) -> Vec<Path> {
    fn visit(
        edges: &[Edge],
        decisions: &[EdgeDecision],
        end: &str,
        include_unknown: bool,
        visited: &mut Vec<String>,
        path: &mut Path,
        output: &mut Vec<Path>,
    ) {
        let current = visited.last().unwrap().clone();
        if current == end {
            output.push(path.clone());
            return;
        }
        for (edge, decision) in edges.iter().zip(decisions) {
            if edge.from != current
                || visited.contains(&edge.to)
                || decision.suitability == Suitability::Unsuitable
                || (!include_unknown && decision.suitability == Suitability::Unknown)
            {
                continue;
            }
            let before = path.clone();
            visited.push(edge.to.clone());
            path.edge_ids.push(edge.id.clone());
            path.distance_meters += edge.length_meters;
            path.cost_meters += edge.length_meters + decision.penalty_meters;
            visit(
                edges,
                decisions,
                end,
                include_unknown,
                visited,
                path,
                output,
            );
            visited.pop();
            *path = before;
        }
    }
    let mut result = Vec::new();
    visit(
        edges,
        decisions,
        end,
        include_unknown,
        &mut vec![start.into()],
        &mut Path {
            edge_ids: vec![],
            distance_meters: 0.0,
            cost_meters: 0.0,
        },
        &mut result,
    );
    result.sort_by(|left, right| {
        left.cost_meters
            .total_cmp(&right.cost_meters)
            .then_with(|| left.edge_ids.cmp(&right.edge_ids))
    });
    result
}

#[test]
fn all_729_three_node_eligibility_graphs_match_exhaustive_paths() {
    let nodes: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
    let edges: Vec<Edge> = [
        ("a", "b", 1.0),
        ("a", "c", 2.0),
        ("b", "a", 1.0),
        ("b", "c", 1.0),
        ("c", "a", 2.0),
        ("c", "b", 1.0),
    ]
    .into_iter()
    .map(|(from, to, length)| Edge {
        id: format!("{from}{to}"),
        from: from.into(),
        to: to.into(),
        length_meters: length,
    })
    .collect();
    let graph = Graph::new(nodes.clone(), edges.clone()).unwrap();
    let reordered = Graph::new(
        nodes.iter().rev().cloned().collect(),
        edges.iter().rev().cloned().collect(),
    )
    .unwrap();
    for pattern in 0..729 {
        let mut remaining = pattern;
        let decisions: Vec<_> = edges
            .iter()
            .map(|edge| {
                let suitability = match remaining % 3 {
                    0 => Suitability::Suitable,
                    1 => Suitability::Unknown,
                    _ => Suitability::Unsuitable,
                };
                remaining /= 3;
                EdgeDecision {
                    edge_id: edge.id.clone(),
                    suitability,
                    penalty_meters: 0.0,
                }
            })
            .collect();
        for start in &nodes {
            for end in &nodes {
                let eligible = all_paths(&edges, &decisions, start, end, false);
                let expected = if let Some(path) = eligible.first() {
                    SearchOutcome::Routed(path.clone())
                } else if !all_paths(&edges, &decisions, start, end, true).is_empty() {
                    SearchOutcome::InsufficientEvidence
                } else {
                    SearchOutcome::Infeasible
                };
                assert_eq!(
                    graph.route(start, end, &decisions).unwrap(),
                    expected,
                    "pattern {pattern}: {start} -> {end}"
                );
            }
        }
        let result = graph.route("a", "c", &decisions).unwrap();
        assert_eq!(
            reordered
                .route(
                    "a",
                    "c",
                    &decisions.iter().rev().cloned().collect::<Vec<_>>()
                )
                .unwrap(),
            result
        );
        for closed in 0..edges.len() {
            let mut stricter = decisions.clone();
            stricter[closed].suitability = Suitability::Unsuitable;
            if let SearchOutcome::Routed(after) = graph.route("a", "c", &stricter).unwrap() {
                let SearchOutcome::Routed(before) = &result else {
                    panic!("closing an edge created a route")
                };
                assert!(after.cost_meters >= before.cost_meters);
                assert!(!after.edge_ids.contains(&edges[closed].id));
            }
        }
    }
}
