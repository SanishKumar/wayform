use serde_json::{Value, json};
use wayform_core::{PolicySnapshot, Suitability};

#[path = "support/policy.rs"]
mod fixture;

#[test]
fn policy_outcomes_match_typescript_with_reordered_inputs() {
    let mut count = 0;
    for line in
        include_str!("../../../packages/mobility-contract/examples/policies-v0.1.0-draft.1.jsonl")
            .lines()
    {
        let case: Value = serde_json::from_str(line).unwrap();
        assert_eq!(case["synthetic"], true);
        let (mut bundle, mut context) = fixture::inputs(&case);
        for _ in 0..2 {
            let snapshot = PolicySnapshot::new(&bundle, &context).unwrap();
            let result = snapshot
                .assess_edge(case["edgeId"].as_str().unwrap())
                .unwrap();
            let actual = json!({ "status": match result.status { Suitability::Suitable => "suitable", Suitability::Unsuitable => "unsuitable", Suitability::Unknown => "unknown" }, "codes": result.reasons.iter().map(|reason| reason.issue.code()).collect::<Vec<_>>() });
            assert_eq!(actual, case["expected"], "{}", case["id"]);
            bundle.edge_ids.reverse();
            bundle.requirements.reverse();
            context.policies.reverse();
            context.minimum_revisions.reverse();
            context.trusted_issuer_ids.reverse();
            for requirement in &mut bundle.requirements {
                requirement.edge_ids.reverse();
            }
            for policy in &mut context.policies {
                policy.closed_edge_ids.reverse();
            }
        }
        count += 1;
    }
    assert_eq!(count, 23);
}
