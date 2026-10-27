use serde_json::Value;
use wayform_core::{
    BundleIdentity, OperationalPolicy, PolicyBundle, PolicyContext, PolicyRequirement,
    RevisionFloor,
};

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap().to_owned()
}
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect()
}
fn identity(value: &Value) -> BundleIdentity {
    BundleIdentity {
        id: text(value, "id"),
        revision: text(value, "revision"),
        digest: text(value, "digest"),
    }
}

pub fn inputs(case: &Value) -> (PolicyBundle, PolicyContext) {
    let bundle = &case["bundle"];
    let context = &case["context"];
    (
        PolicyBundle {
            identity: identity(&bundle["identity"]),
            edge_ids: strings(&bundle["edgeIds"]),
            requirements: bundle["requirements"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| PolicyRequirement {
                    id: text(item, "id"),
                    edge_ids: strings(&item["edgeIds"]),
                })
                .collect(),
        },
        PolicyContext {
            evaluated_at_ms: context["evaluatedAtMs"].as_i64().unwrap(),
            trusted_issuer_ids: strings(&context["trustedIssuerIds"]),
            minimum_revisions: context["minimumRevisions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| RevisionFloor {
                    requirement_id: text(item, "requirementId"),
                    revision: item["revision"].as_u64().unwrap(),
                })
                .collect(),
            policies: context["policies"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| OperationalPolicy {
                    requirement_id: text(item, "requirementId"),
                    bundle: identity(&item["bundle"]),
                    revision: item["revision"].as_u64().unwrap(),
                    issuer_id: text(item, "issuerId"),
                    valid_from_ms: item["validFromMs"].as_i64().unwrap(),
                    valid_until_ms: item["validUntilMs"].as_i64().unwrap(),
                    closed_edge_ids: strings(&item["closedEdgeIds"]),
                })
                .collect(),
        },
    )
}
