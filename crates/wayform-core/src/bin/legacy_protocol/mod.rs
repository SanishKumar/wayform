use serde::Deserialize;
use serde_json::Value;
use wayform_core::{LegacyEdge, LegacyNode, LegacyPolicyPackage, LegacyRouteOptions};

pub const SEARCH_VERSION: &str = "0.1.0";
pub const POLICY_VERSION: &str = "0.2.0";

pub struct Request {
    pub version: &'static str,
    pub source_commit: String,
    pub request_id: String,
    pub nodes: Vec<LegacyNode>,
    pub edges: Vec<LegacyEdge>,
    pub package: Option<LegacyPolicyPackage>,
    pub queries: Vec<Query>,
}

pub struct Query {
    pub query_id: String,
    pub start_id: String,
    pub destination_id: String,
    pub options: LegacyRouteOptions,
    pub policy: Policy,
}

#[derive(Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Policy {
    None,
    Overlay {
        overlay: Value,
        #[serde(rename = "evaluatedAt")]
        evaluated_at: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchRequest {
    protocol_version: String,
    source_commit: String,
    request_id: String,
    nodes: Vec<LegacyNode>,
    edges: Vec<LegacyEdge>,
    queries: Vec<SearchQuery>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SearchQuery {
    query_id: String,
    start_id: String,
    destination_id: String,
    options: LegacyRouteOptions,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyRequest {
    protocol_version: String,
    source_commit: String,
    request_id: String,
    nodes: Vec<LegacyNode>,
    edges: Vec<LegacyEdge>,
    package: LegacyPolicyPackage,
    queries: Vec<PolicyQuery>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyQuery {
    query_id: String,
    start_id: String,
    destination_id: String,
    options: AccessOptions,
    policy: Policy,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AccessOptions {
    accessible_only: bool,
    allow_restricted: bool,
}

fn object_array(value: &Value, name: &str) -> bool {
    value
        .get(name)
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().all(Value::is_object))
}

// Derived struct deserializers can accept positional arrays. Both versions
// require JSON objects for protocol records, including query options.
pub fn parse(value: Value, version: &'static str) -> Result<Request, String> {
    if !value.is_object()
        || !object_array(&value, "nodes")
        || !object_array(&value, "edges")
        || !object_array(&value, "queries")
        || !value["queries"]
            .as_array()
            .is_some_and(|queries| queries.iter().all(|query| query["options"].is_object()))
    {
        return Err("protocol records must be JSON objects".into());
    }
    if version == SEARCH_VERSION {
        let wire: SearchRequest =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        if wire.protocol_version != version {
            return Err("protocol version mismatch".into());
        }
        return Ok(Request {
            version,
            source_commit: wire.source_commit,
            request_id: wire.request_id,
            nodes: wire.nodes,
            edges: wire.edges,
            package: None,
            queries: wire
                .queries
                .into_iter()
                .map(|query| Query {
                    query_id: query.query_id,
                    start_id: query.start_id,
                    destination_id: query.destination_id,
                    options: query.options,
                    policy: Policy::None,
                })
                .collect(),
        });
    }
    if !value["package"].is_object()
        || !object_array(&value["package"], "edges")
        || !value["queries"].as_array().is_some_and(|queries| {
            queries.iter().all(|query| {
                let policy = &query["policy"];
                // Internally tagged unit variants can ignore payload fields.
                // Explicit absence must not discard an attached overlay.
                let Some(fields) = policy.as_object() else {
                    return false;
                };
                match policy.get("mode").and_then(Value::as_str) {
                    Some("none") => fields.len() == 1,
                    Some("overlay") => {
                        fields.len() == 3
                            && fields.contains_key("overlay")
                            && fields.contains_key("evaluatedAt")
                    }
                    _ => false,
                }
            })
        })
    {
        return Err("policy package and explicit query policy records are required".into());
    }
    let wire: PolicyRequest = serde_json::from_value(value).map_err(|error| error.to_string())?;
    if wire.protocol_version != version {
        return Err("protocol version mismatch".into());
    }
    Ok(Request {
        version,
        source_commit: wire.source_commit,
        request_id: wire.request_id,
        nodes: wire.nodes,
        edges: wire.edges,
        package: Some(wire.package),
        queries: wire
            .queries
            .into_iter()
            .map(|query| Query {
                query_id: query.query_id,
                start_id: query.start_id,
                destination_id: query.destination_id,
                options: LegacyRouteOptions {
                    accessible_only: query.options.accessible_only,
                    allow_restricted: query.options.allow_restricted,
                    closed_edge_ids: Vec::new(),
                },
                policy: query.policy,
            })
            .collect(),
    })
}
