use serde_json::{Value, json};
use wayform_core::{
    BundleIdentity, Edge, EdgeDecision, EdgeKind, EligibilityError, EvidenceError, Graph,
    MeasurementError, MeasurementRequirement, MeasurementValue, OperationalPolicy, PolicyBundle,
    PolicyContext, PolicyError, PolicyRequirement, PolicySnapshot, RevisionFloor, SearchOutcome,
    Suitability, assess_edge,
};

#[path = "support/eligibility.rs"]
mod fixture;

fn example(id: &str) -> Value {
    include_str!("../../../packages/mobility-contract/examples/edges-v0.1.0-draft.1.jsonl")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|case| case["id"] == id)
        .unwrap()
}

#[test]
fn evidence_and_policy_times_must_match_even_for_known_exclusions() {
    let mut case = example("restricted-and-missing");
    let now = case["evidenceInput"]["evaluatedAtMs"].as_i64().unwrap();
    case["evidenceInput"]["evaluatedAtMs"] = json!(now + 1);
    let (facts, requirements, evidence, policies) = fixture::inputs(&case);
    assert_eq!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::MismatchedEvaluationTime)
    );
}

#[test]
fn invalid_unconstrained_observations_are_not_hidden_by_a_closure() {
    let (mut facts, requirements, evidence, policies) =
        fixture::inputs(&example("closed-and-missing-width"));
    let (template, _, _, _) = fixture::inputs(&example("measured"));
    let mut observation = template.observations[0].clone();
    observation.attribute = wayform_core::MeasurementAttribute::DoorOperation;
    observation.value = MeasurementValue::Number(1.0);
    facts.observations.push(observation);
    assert!(matches!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::Measurement(
            MeasurementError::InvalidObservation(_)
        ))
    ));
    facts.observations = vec![
        template.observations[0].clone(),
        template.observations[0].clone(),
    ];
    assert!(matches!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::Measurement(
            MeasurementError::DuplicateObservation(_)
        ))
    ));
    facts.observations.truncate(1);
    facts.observations[0].evidence_id = "missing".into();
    assert!(matches!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::Evidence(EvidenceError::UnknownId(_)))
    ));
}

#[test]
fn invalid_or_duplicate_requirements_are_rejected_before_evaluation() {
    let (facts, base, evidence, policies) = fixture::inputs(&example("measured"));
    let mut requirements = base.clone();
    requirements
        .measurements
        .push(MeasurementRequirement::MinimumWidth(0.9));
    assert_eq!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::InvalidRequirements)
    );
    let mut requirements = base.clone();
    requirements
        .measurements
        .push(MeasurementRequirement::DoorOperations(vec![]));
    assert_eq!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::InvalidRequirements)
    );
    for budget in [-1.0, f64::NAN, f64::INFINITY] {
        let mut requirements = base.clone();
        requirements.max_transform_error_meters = budget;
        assert_eq!(
            assess_edge(&facts, &requirements, &evidence, &policies),
            Err(EligibilityError::InvalidRequirements)
        );
    }
}

#[test]
fn frame_transitions_require_valid_entrance_geometry_and_evidence() {
    let (base, requirements, evidence, policies) = fixture::inputs(&example("entrance"));
    let changes: [fn(&mut wayform_core::EdgeFacts); 7] = [
        |facts| facts.kind = EdgeKind::Walkway,
        |facts| facts.registration = None,
        |facts| facts.crosses_frame = false,
        |facts| facts.registration.as_mut().unwrap().controls.truncate(1),
        |facts| {
            let registration = facts.registration.as_mut().unwrap();
            registration.controls[1] = registration.controls[0].clone();
        },
        |facts| facts.registration.as_mut().unwrap().local_endpoint[0] = f64::NAN,
        |facts| facts.registration.as_mut().unwrap().uncertainty_meters = -1.0,
    ];
    for change in changes {
        let mut facts = base.clone();
        change(&mut facts);
        assert_eq!(
            assess_edge(&facts, &requirements, &evidence, &policies),
            Err(EligibilityError::InvalidRegistration)
        );
    }
    let mut facts = base;
    facts.registration.as_mut().unwrap().evidence_id = "missing".into();
    assert!(matches!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::Evidence(EvidenceError::UnknownId(_)))
    ));
}

#[test]
fn absent_policy_inventory_is_an_error_even_for_an_unrestricted_edge() {
    let (mut facts, requirements, evidence, policies) = fixture::inputs(&example("measured"));
    facts.id = "absent".into();
    assert_eq!(
        assess_edge(&facts, &requirements, &evidence, &policies),
        Err(EligibilityError::Policy(PolicyError::UnknownEdge(
            "absent".into()
        )))
    );
}

#[test]
fn search_uses_combined_decisions_and_never_promotes_an_expired_closure() {
    let (template, requirements, evidence, _) = fixture::inputs(&example("measured"));
    let now = evidence.evaluated_at_ms();
    let graph = Graph::new(
        vec!["a".into(), "b".into(), "c".into()],
        vec![
            Edge {
                id: "ab".into(),
                from: "a".into(),
                to: "b".into(),
                length_meters: 1.0,
            },
            Edge {
                id: "ac".into(),
                from: "a".into(),
                to: "c".into(),
                length_meters: 5.0,
            },
            Edge {
                id: "cb".into(),
                from: "c".into(),
                to: "b".into(),
                length_meters: 5.0,
            },
        ],
    )
    .unwrap();
    let bundle = PolicyBundle {
        identity: BundleIdentity {
            id: "synthetic".into(),
            revision: "r1".into(),
            digest: "a".repeat(64),
        },
        edge_ids: graph.edges().map(|edge| edge.id.clone()).collect(),
        requirements: vec![PolicyRequirement {
            id: "operations".into(),
            edge_ids: vec!["ab".into()],
        }],
    };
    let mut context = PolicyContext {
        evaluated_at_ms: now,
        trusted_issuer_ids: vec!["operator".into()],
        minimum_revisions: vec![RevisionFloor {
            requirement_id: "operations".into(),
            revision: 1,
        }],
        policies: vec![OperationalPolicy {
            requirement_id: "operations".into(),
            bundle: bundle.identity.clone(),
            revision: 1,
            issuer_id: "operator".into(),
            valid_from_ms: now - 1_000,
            valid_until_ms: now + 1_000,
            closed_edge_ids: vec!["ab".into()],
        }],
    };
    for expired in [false, true] {
        if expired {
            context.policies[0].valid_until_ms = now;
        }
        let policies = PolicySnapshot::new(&bundle, &context).unwrap();
        let decisions: Vec<_> = graph
            .edges()
            .map(|edge| {
                let mut facts = template.clone();
                facts.id = edge.id.clone();
                if expired && edge.id == "ac" {
                    facts.restricted = true;
                }
                let assessment = assess_edge(&facts, &requirements, &evidence, &policies).unwrap();
                if edge.id == "ab" {
                    assert_eq!(
                        assessment.status,
                        if expired {
                            Suitability::Unknown
                        } else {
                            Suitability::Unsuitable
                        }
                    );
                }
                EdgeDecision {
                    edge_id: edge.id.clone(),
                    suitability: assessment.status,
                    penalty_meters: 0.0,
                }
            })
            .collect();
        let outcome = graph.route("a", "b", &decisions).unwrap();
        if expired {
            assert_eq!(outcome, SearchOutcome::InsufficientEvidence);
        } else {
            let SearchOutcome::Routed(path) = outcome else {
                panic!("expected the suitable detour")
            };
            assert_eq!(path.edge_ids, ["ac", "cb"]);
        }
    }
}
