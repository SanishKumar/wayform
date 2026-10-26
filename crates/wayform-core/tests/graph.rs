use wayform_core::{Edge, Graph, GraphError, MAX_NODES};

fn nodes() -> Vec<String> {
    ["a", "b", "c"].map(String::from).to_vec()
}

fn edge(id: &str, from: &str, to: &str, length: f64) -> Edge {
    Edge {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        length_meters: length,
    }
}

#[test]
fn directed_and_parallel_edges_keep_their_identity() {
    let graph = Graph::new(
        nodes(),
        vec![edge("ramp", "a", "b", 12.0), edge("stairs", "a", "b", 3.0)],
    )
    .unwrap();
    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);
    assert_eq!(
        graph
            .outgoing_edges("a")
            .unwrap()
            .map(|edge| edge.id.as_str())
            .collect::<Vec<_>>(),
        ["ramp", "stairs"]
    );
    assert_eq!(graph.outgoing_edges("b").unwrap().count(), 0);
    assert!(graph.outgoing_edges("missing").is_none());
    assert_eq!(graph.destination("ramp"), Some("b"));
}

#[test]
fn indexing_is_independent_of_collection_order() {
    let edges = vec![edge("z", "a", "b", 2.0), edge("a", "a", "c", 1.0)];
    let first = Graph::new(nodes(), edges.clone()).unwrap();
    let second = Graph::new(
        nodes().into_iter().rev().collect(),
        edges.into_iter().rev().collect(),
    )
    .unwrap();
    assert_eq!(
        first.edges().collect::<Vec<_>>(),
        second.edges().collect::<Vec<_>>()
    );
}

#[test]
fn identifiers_follow_typescript_utf16_order() {
    let graph = Graph::new(
        nodes(),
        vec![
            edge("\u{e000}", "a", "b", 1.0),
            edge("\u{10000}", "a", "b", 1.0),
        ],
    )
    .unwrap();
    assert_eq!(
        graph
            .edges()
            .map(|edge| edge.id.as_str())
            .collect::<Vec<_>>(),
        ["\u{10000}", "\u{e000}"]
    );
}

#[test]
fn malformed_topology_is_rejected() {
    assert_eq!(
        Graph::new(vec![], vec![]).unwrap_err(),
        GraphError::EmptyGraph
    );
    assert_eq!(
        Graph::new(vec!["".into()], vec![]).unwrap_err(),
        GraphError::EmptyNodeId
    );
    assert_eq!(
        Graph::new(vec!["a".into(), "a".into()], vec![]).unwrap_err(),
        GraphError::DuplicateNode("a".into())
    );
    assert_eq!(
        Graph::new(nodes(), vec![edge("", "a", "b", 1.0)]).unwrap_err(),
        GraphError::EmptyEdgeId
    );
    assert_eq!(
        Graph::new(nodes(), vec![edge("ab", "a", "a", 1.0)]).unwrap_err(),
        GraphError::SelfLoop("ab".into())
    );
    assert_eq!(
        Graph::new(nodes(), vec![edge("ab", "a", "missing", 1.0)]).unwrap_err(),
        GraphError::UnknownEndpoint {
            edge_id: "ab".into(),
            node_id: "missing".into()
        }
    );
    assert_eq!(
        Graph::new(
            nodes(),
            vec![edge("same", "a", "b", 1.0), edge("same", "b", "a", 1.0)]
        )
        .unwrap_err(),
        GraphError::DuplicateEdge("same".into())
    );
}

#[test]
fn lengths_must_be_positive_and_finite() {
    for length in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            Graph::new(nodes(), vec![edge("ab", "a", "b", length)]).unwrap_err(),
            GraphError::InvalidLength("ab".into())
        );
    }
}

#[test]
fn oversized_node_inventory_is_rejected_before_indexing() {
    assert_eq!(
        Graph::new(vec!["a".into(); MAX_NODES + 1], vec![]).unwrap_err(),
        GraphError::TooManyNodes
    );
}
