use wayform_core::{
    BundleIdentity, OperationalPolicy, PolicyBundle, PolicyContext, PolicyError, PolicyIssue,
    PolicyRequirement, PolicySnapshot, RevisionFloor, Suitability,
};

fn fixture() -> (PolicyBundle, PolicyContext) {
    let bundle = PolicyBundle {
        identity: BundleIdentity {
            id: "synthetic".into(),
            revision: "r1".into(),
            digest: "a".repeat(64),
        },
        edge_ids: vec!["ab".into(), "ba".into()],
        requirements: vec![PolicyRequirement {
            id: "lift".into(),
            edge_ids: vec!["ab".into()],
        }],
    };
    let context = PolicyContext {
        evaluated_at_ms: 500,
        trusted_issuer_ids: vec!["operator".into()],
        minimum_revisions: vec![RevisionFloor {
            requirement_id: "lift".into(),
            revision: 3,
        }],
        policies: vec![OperationalPolicy {
            requirement_id: "lift".into(),
            bundle: bundle.identity.clone(),
            revision: 3,
            issuer_id: "operator".into(),
            valid_from_ms: 0,
            valid_until_ms: 1_000,
            closed_edge_ids: vec!["ab".into()],
        }],
    };
    (bundle, context)
}

#[test]
fn closures_expire_to_unknown_and_reopening_requires_a_current_snapshot() {
    let (bundle, mut context) = fixture();
    let closed = PolicySnapshot::new(&bundle, &context).unwrap();
    assert_eq!(
        closed.assess_edge("ab").unwrap().status,
        Suitability::Unsuitable
    );
    assert_eq!(
        closed.assess_edge("ba").unwrap().status,
        Suitability::Suitable
    );
    for (now, expected) in [
        (-1, PolicyIssue::NotCurrent),
        (0, PolicyIssue::Closed),
        (999, PolicyIssue::Closed),
        (1000, PolicyIssue::NotCurrent),
    ] {
        context.evaluated_at_ms = now;
        assert_eq!(
            PolicySnapshot::new(&bundle, &context)
                .unwrap()
                .assess_edge("ab")
                .unwrap()
                .reasons[0]
                .issue,
            expected
        );
    }
    context.evaluated_at_ms = 500;
    context.policies[0].closed_edge_ids.clear();
    context.policies[0].revision = 4;
    context.minimum_revisions[0].revision = 4;
    assert_eq!(
        PolicySnapshot::new(&bundle, &context)
            .unwrap()
            .assess_edge("ab")
            .unwrap()
            .status,
        Suitability::Suitable
    );
    assert_eq!(
        closed.assess_edge("ab").unwrap().status,
        Suitability::Unsuitable
    );
    context.policies[0].revision = 3;
    assert_eq!(
        PolicySnapshot::new(&bundle, &context)
            .unwrap()
            .assess_edge("ab")
            .unwrap()
            .reasons[0]
            .issue,
        PolicyIssue::UntrustedRevision
    );
}

#[test]
fn unknown_and_closed_requirements_retain_both_reasons() {
    let (mut bundle, context) = fixture();
    bundle.requirements.push(PolicyRequirement {
        id: "door".into(),
        edge_ids: vec!["ab".into()],
    });
    let snapshot = PolicySnapshot::new(&bundle, &context).unwrap();
    let result = snapshot.assess_edge("ab").unwrap();
    assert_eq!(result.status, Suitability::Unsuitable);
    assert_eq!(
        result
            .reasons
            .iter()
            .map(|reason| reason.issue)
            .collect::<Vec<_>>(),
        [PolicyIssue::Missing, PolicyIssue::Closed]
    );
    assert_eq!(
        snapshot.assess_edge("absent"),
        Err(PolicyError::UnknownEdge("absent".into()))
    );
}

#[test]
fn malformed_bundle_identity_inventory_and_requirements_are_rejected() {
    let changes: [fn(&mut PolicyBundle); 8] = [
        |bundle| bundle.identity.id.clear(),
        |bundle| bundle.identity.digest = "A".repeat(64),
        |bundle| bundle.edge_ids.push("ab".into()),
        |bundle| bundle.edge_ids.push(String::new()),
        |bundle| bundle.requirements.push(bundle.requirements[0].clone()),
        |bundle| bundle.requirements[0].edge_ids.clear(),
        |bundle| bundle.requirements[0].edge_ids.push("missing".into()),
        |bundle| bundle.requirements[0].edge_ids.push("ab".into()),
    ];
    for change in changes {
        let (mut bundle, context) = fixture();
        change(&mut bundle);
        assert!(PolicySnapshot::new(&bundle, &context).is_err());
    }
}

#[test]
fn malformed_policies_are_invalid_even_when_expired_or_untrusted() {
    let changes: [fn(&mut PolicyContext); 10] = [
        |context| context.policies.push(context.policies[0].clone()),
        |context| context.policies[0].requirement_id = "foreign".into(),
        |context| context.policies[0].closed_edge_ids = vec!["ba".into()],
        |context| context.policies[0].closed_edge_ids.push("ab".into()),
        |context| context.policies[0].bundle.digest = "bad".into(),
        |context| context.policies[0].issuer_id.clear(),
        |context| context.policies[0].revision = 9_007_199_254_740_992,
        |context| context.policies[0].valid_until_ms = 0,
        |context| context.policies[0].valid_from_ms = i64::MIN,
        |context| context.policies[0].valid_until_ms = i64::MAX,
    ];
    for change in changes {
        let (bundle, mut context) = fixture();
        context.evaluated_at_ms = 5_000;
        context.trusted_issuer_ids.clear();
        change(&mut context);
        assert!(matches!(
            PolicySnapshot::new(&bundle, &context),
            Err(PolicyError::InvalidPolicy(_))
        ));
    }
}

#[test]
fn context_and_revision_floors_must_be_well_formed() {
    let changes: [fn(&mut PolicyContext); 6] = [
        |context| context.evaluated_at_ms = i64::MAX,
        |context| context.trusted_issuer_ids.push("operator".into()),
        |context| context.trusted_issuer_ids.push(String::new()),
        |context| {
            context
                .minimum_revisions
                .push(context.minimum_revisions[0].clone())
        },
        |context| context.minimum_revisions[0].requirement_id = "foreign".into(),
        |context| context.minimum_revisions[0].revision = 9_007_199_254_740_992,
    ];
    for change in changes {
        let (bundle, mut context) = fixture();
        change(&mut context);
        assert!(PolicySnapshot::new(&bundle, &context).is_err());
    }
}

#[test]
fn missing_floors_differ_from_an_explicit_zero_floor() {
    let (bundle, mut context) = fixture();
    context.minimum_revisions.clear();
    assert_eq!(
        PolicySnapshot::new(&bundle, &context)
            .unwrap()
            .assess_edge("ab")
            .unwrap()
            .reasons[0]
            .issue,
        PolicyIssue::UntrustedRevision
    );
    context.minimum_revisions.push(RevisionFloor {
        requirement_id: "lift".into(),
        revision: 0,
    });
    context.policies[0].revision = 0;
    assert_eq!(
        PolicySnapshot::new(&bundle, &context)
            .unwrap()
            .assess_edge("ab")
            .unwrap()
            .reasons[0]
            .issue,
        PolicyIssue::Closed
    );
}
