use serde_json::{Value, json};
use wayform_core::{MeasurementAttribute, Suitability, assess_edge};

#[path = "support/eligibility.rs"]
mod fixture;

#[test]
fn combined_edge_decisions_match_typescript_with_reordered_inputs() {
    let mut count = 0;
    for line in
        include_str!("../../../packages/mobility-contract/examples/edges-v0.1.0-draft.1.jsonl")
            .lines()
    {
        let mut case: Value = serde_json::from_str(line).unwrap();
        assert_eq!(case["synthetic"], true);
        for permuted in [false, true] {
            let (facts, mut requirements, evidence, policies) = fixture::inputs(&case);
            if permuted {
                requirements.measurements.reverse();
            }
            let assessment = assess_edge(&facts, &requirements, &evidence, &policies).unwrap();
            assert_eq!(assessment.edge_id, facts.id);
            let actual = json!({
                "status": match assessment.status { Suitability::Suitable => "suitable", Suitability::Unsuitable => "unsuitable", Suitability::Unknown => "unknown" },
                "reasons": assessment.reasons.iter().map(|reason| json!({"code": reason.code.code(), "edgeId": assessment.edge_id, "attribute": reason.attribute.map(MeasurementAttribute::name), "evidenceIds": reason.evidence_ids})).collect::<Vec<_>>(),
                "evidenceIds": assessment.evidence_ids,
            });
            assert_eq!(actual, case["expected"], "{}", case["id"]);
            case["facts"]["observations"]
                .as_array_mut()
                .unwrap()
                .reverse();
            case["evidenceInput"]["records"]
                .as_array_mut()
                .unwrap()
                .reverse();
            case["policyInput"]["bundle"]["requirements"]
                .as_array_mut()
                .unwrap()
                .reverse();
            case["policyInput"]["context"]["policies"]
                .as_array_mut()
                .unwrap()
                .reverse();
            if !case["facts"]["registration"].is_null() {
                case["facts"]["registration"]["controls"]
                    .as_array_mut()
                    .unwrap()
                    .reverse();
            }
        }
        count += 1;
    }
    assert_eq!(count, 26);
}
