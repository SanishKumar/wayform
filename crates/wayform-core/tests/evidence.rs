use wayform_core::{
    EvidenceError, EvidenceIssue, EvidencePolicy, EvidenceRecord, EvidenceSnapshot, EvidenceSource,
    ReviewStatus,
};

fn record() -> EvidenceRecord {
    EvidenceRecord {
        id: "survey".into(),
        source: EvidenceSource::FieldSurvey,
        actor_id: "fictional-surveyor".into(),
        method: "synthetic-test".into(),
        observed_at_ms: 1_000,
        valid_until_ms: None,
        review_status: ReviewStatus::Reviewed,
        reviewer_id: Some("fictional-reviewer".into()),
        reviewed_at_ms: Some(1_000),
    }
}

fn policy() -> EvidencePolicy {
    EvidencePolicy {
        accepted_sources: vec![EvidenceSource::FieldSurvey],
        require_reviewed: true,
        max_age_seconds: 10.0,
    }
}

fn issue(record: &EvidenceRecord, policy: &EvidencePolicy, now: i64) -> Option<EvidenceIssue> {
    EvidenceSnapshot::new(std::slice::from_ref(record), policy, now)
        .unwrap()
        .issue(&record.id)
        .unwrap()
}

#[test]
fn observation_age_and_expiry_have_distinct_inclusive_boundaries() {
    let mut record = record();
    assert_eq!(issue(&record, &policy(), 999), Some(EvidenceIssue::Future));
    assert_eq!(issue(&record, &policy(), 1_000), None);
    assert_eq!(issue(&record, &policy(), 11_000), None);
    assert_eq!(
        issue(&record, &policy(), 11_001),
        Some(EvidenceIssue::Stale)
    );
    record.valid_until_ms = Some(5_000);
    assert_eq!(issue(&record, &policy(), 4_999), None);
    assert_eq!(issue(&record, &policy(), 5_000), Some(EvidenceIssue::Stale));
    let mut fractional = policy();
    fractional.max_age_seconds = 0.001;
    assert_eq!(issue(&record, &fractional, 1_001), None);
    assert_eq!(
        issue(&record, &fractional, 1_002),
        Some(EvidenceIssue::Stale)
    );
}

#[test]
fn review_rules_include_future_reviews_and_explicit_rejections() {
    let mut record = record();
    record.reviewed_at_ms = Some(2_000);
    assert_eq!(
        issue(&record, &policy(), 1_999),
        Some(EvidenceIssue::Future)
    );
    assert_eq!(issue(&record, &policy(), 2_000), None);
    record.review_status = ReviewStatus::Pending;
    record.reviewed_at_ms = None;
    record.reviewer_id = None;
    assert_eq!(
        issue(&record, &policy(), 2_000),
        Some(EvidenceIssue::Unreviewed)
    );
    let mut optional_review = policy();
    optional_review.require_reviewed = false;
    assert_eq!(issue(&record, &optional_review, 2_000), None);
    record.review_status = ReviewStatus::Rejected;
    assert_eq!(
        issue(&record, &optional_review, 2_000),
        Some(EvidenceIssue::Unreviewed)
    );
}

#[test]
fn issue_precedence_matches_source_then_review_then_time() {
    let mut record = record();
    record.source = EvidenceSource::Inference;
    record.review_status = ReviewStatus::Rejected;
    assert_eq!(
        issue(&record, &policy(), 0),
        Some(EvidenceIssue::UnacceptedSource)
    );
    record.source = EvidenceSource::FieldSurvey;
    assert_eq!(
        issue(&record, &policy(), 0),
        Some(EvidenceIssue::Unreviewed)
    );
    record.review_status = ReviewStatus::Reviewed;
    assert_eq!(issue(&record, &policy(), 0), Some(EvidenceIssue::Future));
}

#[test]
fn malformed_metadata_is_invalid_even_when_the_source_is_unaccepted() {
    let mutations: [fn(&mut EvidenceRecord); 10] = [
        |record| record.id.clear(),
        |record| record.actor_id.clear(),
        |record| record.method.clear(),
        |record| record.valid_until_ms = Some(record.observed_at_ms),
        |record| record.reviewed_at_ms = Some(record.observed_at_ms - 1),
        |record| record.reviewed_at_ms = None,
        |record| record.reviewer_id = None,
        |record| record.reviewer_id = Some(String::new()),
        |record| record.observed_at_ms = i64::MIN,
        |record| record.valid_until_ms = Some(i64::MAX),
    ];
    for change in mutations {
        let mut record = record();
        record.source = EvidenceSource::Inference;
        change(&mut record);
        assert!(matches!(
            EvidenceSnapshot::new(&[record], &policy(), 2_000),
            Err(EvidenceError::InvalidRecord(_))
        ));
    }
}

#[test]
fn duplicate_and_missing_evidence_references_are_errors() {
    let records = vec![record(), record()];
    assert!(matches!(
        EvidenceSnapshot::new(&records, &policy(), 2_000),
        Err(EvidenceError::DuplicateId(_))
    ));
    let snapshot = EvidenceSnapshot::new(&records[..1], &policy(), 2_000).unwrap();
    assert_eq!(
        snapshot.issue("absent"),
        Err(EvidenceError::UnknownId("absent".into()))
    );
    assert_eq!(snapshot.issue("survey"), Ok(None));
}

#[test]
fn invalid_policy_and_unrepresentable_contract_dates_are_rejected() {
    for max_age_seconds in [-1.0, f64::INFINITY, f64::NAN] {
        let mut policy = policy();
        policy.max_age_seconds = max_age_seconds;
        assert!(matches!(
            EvidenceSnapshot::new(&[], &policy, 0),
            Err(EvidenceError::InvalidPolicy)
        ));
    }
    for sources in [
        vec![],
        vec![EvidenceSource::Operator, EvidenceSource::Operator],
    ] {
        let mut policy = policy();
        policy.accepted_sources = sources;
        assert!(matches!(
            EvidenceSnapshot::new(&[], &policy, 0),
            Err(EvidenceError::InvalidPolicy)
        ));
    }
    for now in [-62_167_219_200_000, 253_402_300_799_999] {
        assert!(EvidenceSnapshot::new(&[], &policy(), now).is_ok());
    }
    for now in [-62_167_219_200_001, 253_402_300_800_000] {
        assert!(matches!(
            EvidenceSnapshot::new(&[], &policy(), now),
            Err(EvidenceError::InvalidEvaluationTime)
        ));
    }
}

#[test]
fn snapshots_are_immutable_and_do_not_reuse_previous_time_or_policy() {
    let mut records = vec![record()];
    let fresh = EvidenceSnapshot::new(&records, &policy(), 1_000).unwrap();
    records[0].source = EvidenceSource::Inference;
    let changed = EvidenceSnapshot::new(&records, &policy(), 1_000).unwrap();
    assert_eq!(fresh.issue("survey"), Ok(None));
    assert_eq!(
        changed.issue("survey"),
        Ok(Some(EvidenceIssue::UnacceptedSource))
    );
    assert_eq!(
        issue(&record(), &policy(), 11_001),
        Some(EvidenceIssue::Stale)
    );
    assert_eq!(fresh.issue("survey"), Ok(None));
}
