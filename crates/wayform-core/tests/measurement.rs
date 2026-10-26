use wayform_core::{
    EvidenceError, EvidencePolicy, EvidenceRecord, EvidenceSnapshot, EvidenceSource,
    MeasurementAttribute, MeasurementError, MeasurementObservation, MeasurementRequirement,
    MeasurementValue, ReviewStatus, Suitability, assess_measurement,
};

fn snapshot() -> EvidenceSnapshot {
    EvidenceSnapshot::new(
        &[EvidenceRecord {
            id: "survey".into(),
            source: EvidenceSource::FieldSurvey,
            actor_id: "fictional-surveyor".into(),
            method: "synthetic-test".into(),
            observed_at_ms: 0,
            valid_until_ms: None,
            review_status: ReviewStatus::Reviewed,
            reviewer_id: Some("fictional-reviewer".into()),
            reviewed_at_ms: Some(0),
        }],
        &EvidencePolicy {
            accepted_sources: vec![EvidenceSource::FieldSurvey],
            require_reviewed: true,
            max_age_seconds: 0.0,
        },
        0,
    )
    .unwrap()
}

fn observation() -> MeasurementObservation {
    MeasurementObservation {
        id: "width".into(),
        attribute: MeasurementAttribute::ClearWidthMeters,
        value: MeasurementValue::Number(1.0),
        uncertainty: Some(0.02),
        unknown_reason: None,
        evidence_id: "survey".into(),
    }
}

#[test]
fn malformed_observations_cannot_become_missing_or_suitable_evidence() {
    let changes: [fn(&mut MeasurementObservation); 12] = [
        |item| item.id.clear(),
        |item| item.attribute = MeasurementAttribute::SlopePercent,
        |item| item.value = MeasurementValue::Number(f64::NAN),
        |item| item.value = MeasurementValue::Number(f64::INFINITY),
        |item| item.value = MeasurementValue::Number(-1.0),
        |item| item.value = MeasurementValue::Category("wide".into()),
        |item| item.uncertainty = Some(-1.0),
        |item| item.uncertainty = Some(f64::INFINITY),
        |item| item.uncertainty = Some(f64::NAN),
        |item| item.unknown_reason = Some("unexpected".into()),
        |item| {
            item.value = MeasurementValue::Unknown;
            item.uncertainty = None;
        },
        |item| {
            item.value = MeasurementValue::Unknown;
            item.unknown_reason = Some("not-surveyed".into());
        },
    ];
    for change in changes {
        let mut item = observation();
        change(&mut item);
        assert!(matches!(
            assess_measurement(
                &MeasurementRequirement::MinimumWidth(0.8),
                &[item],
                &snapshot()
            ),
            Err(MeasurementError::InvalidObservation(_))
        ));
    }
}

#[test]
fn invalid_limits_and_empty_or_duplicate_category_sets_are_rejected() {
    let requirements = [
        MeasurementRequirement::MinimumWidth(-1.0),
        MeasurementRequirement::MinimumWidth(f64::NAN),
        MeasurementRequirement::MaximumCrossSlope(f64::INFINITY),
        MeasurementRequirement::MaximumSteps(9_007_199_254_740_992),
        MeasurementRequirement::Slope {
            max_uphill: None,
            max_downhill: None,
        },
        MeasurementRequirement::Slope {
            max_uphill: Some(-1.0),
            max_downhill: None,
        },
        MeasurementRequirement::Slope {
            max_uphill: None,
            max_downhill: Some(f64::NAN),
        },
        MeasurementRequirement::Surfaces(vec![]),
        MeasurementRequirement::Surfaces(vec!["paved".into(), "paved".into()]),
        MeasurementRequirement::DoorOperations(vec![String::new()]),
    ];
    for requirement in requirements {
        assert_eq!(
            assess_measurement(&requirement, &[], &snapshot()),
            Err(MeasurementError::InvalidRequirement)
        );
    }
}

#[test]
fn missing_references_and_duplicate_observations_are_invalid() {
    let mut item = observation();
    item.evidence_id = "absent".into();
    assert_eq!(
        assess_measurement(
            &MeasurementRequirement::MinimumWidth(0.8),
            &[item],
            &snapshot()
        ),
        Err(MeasurementError::Evidence(EvidenceError::UnknownId(
            "absent".into()
        )))
    );
    assert_eq!(
        assess_measurement(
            &MeasurementRequirement::MinimumWidth(0.8),
            &[observation(), observation()],
            &snapshot()
        ),
        Err(MeasurementError::DuplicateObservation("width".into()))
    );
}

#[test]
fn step_counts_and_errors_must_be_safe_nonnegative_integers() {
    let mut item = observation();
    item.attribute = MeasurementAttribute::StepCount;
    for value in [-1.0, 0.5, 9_007_199_254_740_992.0] {
        item.value = MeasurementValue::Number(value);
        item.uncertainty = Some(0.0);
        assert!(matches!(
            assess_measurement(
                &MeasurementRequirement::MaximumSteps(0),
                &[item.clone()],
                &snapshot()
            ),
            Err(MeasurementError::InvalidObservation(_))
        ));
        item.value = MeasurementValue::Number(0.0);
        item.uncertainty = Some(value);
        assert!(matches!(
            assess_measurement(
                &MeasurementRequirement::MaximumSteps(0),
                &[item.clone()],
                &snapshot()
            ),
            Err(MeasurementError::InvalidObservation(_))
        ));
    }
}

#[test]
fn categories_require_nonempty_text_and_no_numeric_error() {
    let mut item = observation();
    item.attribute = MeasurementAttribute::Surface;
    for (value, uncertainty) in [
        (MeasurementValue::Number(1.0), None),
        (MeasurementValue::Category(String::new()), None),
        (MeasurementValue::Category("paved".into()), Some(0.0)),
    ] {
        item.value = value;
        item.uncertainty = uncertainty;
        assert!(matches!(
            assess_measurement(
                &MeasurementRequirement::Surfaces(vec!["paved".into()]),
                &[item.clone()],
                &snapshot()
            ),
            Err(MeasurementError::InvalidObservation(_))
        ));
    }
}

#[test]
fn tightening_width_never_qualifies_previously_unqualified_intervals() {
    let snapshot = snapshot();
    for value in 0..=40 {
        for error in 0..=10 {
            let mut item = observation();
            item.value = MeasurementValue::Number(f64::from(value) / 20.0);
            item.uncertainty = Some(f64::from(error) / 50.0);
            let loose = assess_measurement(
                &MeasurementRequirement::MinimumWidth(0.8),
                std::slice::from_ref(&item),
                &snapshot,
            )
            .unwrap();
            let strict = assess_measurement(
                &MeasurementRequirement::MinimumWidth(1.0),
                &[item],
                &snapshot,
            )
            .unwrap();
            if strict.status == Suitability::Suitable {
                assert_eq!(loose.status, Suitability::Suitable);
            }
        }
    }
}
