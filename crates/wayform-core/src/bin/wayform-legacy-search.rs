mod legacy_protocol;
mod support;

use std::collections::HashSet;
use std::io;

use legacy_protocol::{
    COMPLETE_VERSION, POLICY_VERSION, Policy, Query, Request, SEARCH_VERSION, VENUE_VERSION,
};
use serde::Serialize;
use wayform_core::{
    LEGACY_SOURCE_COMMIT, LegacyCompiledOptions, LegacyGraph, LegacyOverlayResolution,
    LegacyPolicyError, LegacyPolicyIndex, LegacyRouteError, LegacySearchError, LegacySearchOutcome,
    LegacyVenuePackage, parse_bounded_json,
};

const MAX_QUERIES: usize = 10_000;
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    protocol_version: &'static str,
    source_commit: &'static str,
    request_id: Option<String>,
    #[serde(flatten)]
    result: BatchResult,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
enum BatchResult {
    Completed { results: Vec<QueryResponse> },
    Rejected { error: Failure },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryResponse {
    query_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    overlay_resolution: Option<LegacyOverlayResolution>,
    #[serde(flatten)]
    result: QueryResult,
}

#[derive(Serialize)]
#[serde(untagged)]
enum QueryResult {
    Complete(serde_json::Value),
    Search(LegacySearchOutcome),
    Rejected {
        outcome: &'static str,
        error: Failure,
    },
}

#[derive(Serialize)]
struct Failure {
    code: &'static str,
    message: String,
}

fn failure(code: &'static str, message: impl ToString) -> Failure {
    Failure {
        code,
        message: message.to_string().chars().take(512).collect(),
    }
}

fn rejected(code: &'static str, message: impl ToString) -> BatchResult {
    BatchResult::Rejected {
        error: failure(code, message),
    }
}

fn response(version: &'static str, request_id: Option<String>, result: BatchResult) -> Response {
    Response {
        protocol_version: version,
        source_commit: LEGACY_SOURCE_COMMIT,
        request_id,
        result,
    }
}

fn error_code(error: &LegacySearchError) -> &'static str {
    match error {
        LegacySearchError::InvalidGraph => "invalid-graph",
        LegacySearchError::InvalidClosure(_) => "invalid-closure",
        LegacySearchError::UnrepresentableCost => "unrepresentable-cost",
        LegacySearchError::WorkLimitExceeded => "work-limit-exceeded",
    }
}

fn execute(request: Request) -> BatchResult {
    if request.source_commit != LEGACY_SOURCE_COMMIT {
        return rejected("unsupported-source", "unsupported reference source commit");
    }
    let max_queries = if request.version == COMPLETE_VERSION {
        100
    } else {
        MAX_QUERIES
    };
    if request.queries.is_empty() || request.queries.len() > max_queries {
        return rejected(
            "invalid-batch",
            format!("expected between 1 and {max_queries} queries"),
        );
    }
    let mut ids = HashSet::new();
    if request.queries.iter().any(|query| {
        query.query_id.is_empty()
            || query.query_id.len() > 512
            || !ids.insert(query.query_id.as_str())
    }) {
        return rejected(
            "invalid-batch",
            "query IDs must be unique and contain 1 to 512 UTF-8 bytes",
        );
    }
    // Account for the actual serialized envelope, including escaped request IDs.
    let empty = response(
        request.version,
        Some(request.request_id),
        BatchResult::Completed {
            results: Vec::new(),
        },
    );
    let mut response_bytes = match serde_json::to_vec(&empty) {
        Ok(bytes) => bytes.len(),
        Err(error) => return rejected("encoding-failed", error),
    };
    let policy_index = if let Some(package) = request.package {
        let graph_ids: HashSet<_> = request.edges.iter().map(|edge| edge.id.as_str()).collect();
        if request.verified_graph.is_none()
            && (package.edges.len() != graph_ids.len()
                || package
                    .edges
                    .iter()
                    .any(|edge| !graph_ids.contains(edge.id.as_str())))
        {
            return rejected(
                "invalid-policy-package",
                "policy and search edge inventories must match exactly",
            );
        }
        match LegacyPolicyIndex::new(package) {
            Ok(index) => Some(index),
            Err(error) => return rejected("invalid-policy-package", error),
        }
    } else {
        None
    };
    let graph = if request.verified_venue.is_some() {
        None
    } else {
        Some(
            match request
                .verified_graph
                .map_or_else(|| LegacyGraph::new(request.nodes, request.edges), Ok)
            {
                Ok(graph) => graph,
                Err(error) => return rejected(error_code(&error), error),
            },
        )
    };
    let mut results = Vec::with_capacity(request.queries.len());
    for query in request.queries {
        let item = if let Some(venue) = &request.verified_venue {
            execute_complete_query(venue, query)
        } else {
            execute_query(graph.as_ref().unwrap(), policy_index.as_ref(), query)
        };
        let size = match serde_json::to_vec(&item) {
            Ok(bytes) => bytes.len(),
            Err(error) => return rejected("encoding-failed", error),
        };
        response_bytes += size + usize::from(!results.is_empty());
        if response_bytes > MAX_RESPONSE_BYTES {
            return rejected(
                "response-too-large",
                "batch response exceeds 16 MiB; send fewer queries",
            );
        }
        results.push(item);
    }
    BatchResult::Completed { results }
}

fn query_rejection(code: &'static str, message: impl ToString) -> QueryResult {
    QueryResult::Rejected {
        outcome: "rejected",
        error: failure(code, message),
    }
}

fn execute_complete_query(venue: &LegacyVenuePackage, query: Query) -> QueryResponse {
    let (operational_overlay, evaluated_at) = match query.policy {
        Policy::None => (None, None),
        Policy::Overlay {
            overlay,
            evaluated_at,
        } => (Some(overlay), Some(evaluated_at)),
    };
    let options = LegacyCompiledOptions {
        accessible_only: query.options.accessible_only,
        allow_restricted: query.options.allow_restricted,
        operational_overlay,
        evaluated_at,
    };
    let result = match venue.calculate_route(&query.start_id, &query.destination_id, &options) {
        Ok(route) => QueryResult::Complete(route),
        Err(error) => {
            let code = match &error {
                LegacyRouteError::Search(error) => error_code(error),
                LegacyRouteError::Policy(LegacyPolicyError::WorkLimitExceeded) => {
                    "work-limit-exceeded"
                }
                LegacyRouteError::Policy(LegacyPolicyError::InvalidPackage) => {
                    "invalid-policy-package"
                }
                LegacyRouteError::UnsupportedConnectorId => "unsupported-connector-id",
            };
            query_rejection(code, error)
        }
    };
    QueryResponse {
        query_id: query.query_id,
        overlay_resolution: None,
        result,
    }
}

fn execute_query(
    graph: &LegacyGraph,
    index: Option<&LegacyPolicyIndex>,
    mut query: Query,
) -> QueryResponse {
    let mut item = QueryResponse {
        query_id: query.query_id,
        overlay_resolution: None,
        result: query_rejection(
            "invalid-policy-package",
            "overlay requires a policy package",
        ),
    };
    if let Policy::Overlay {
        overlay,
        evaluated_at,
    } = query.policy
    {
        let Some(index) = index else {
            return item;
        };
        match index.resolve(&overlay, &evaluated_at) {
            Ok(resolution) => {
                let valid = resolution.valid;
                query.options.closed_edge_ids = resolution.closed_edge_ids.clone();
                item.overlay_resolution = Some(resolution);
                if !valid {
                    item.result = query_rejection(
                        "invalid-overlay",
                        "operational overlay rejected; search was not run",
                    );
                    return item;
                }
            }
            Err(error) => {
                let code = match error {
                    LegacyPolicyError::WorkLimitExceeded => "work-limit-exceeded",
                    LegacyPolicyError::InvalidPackage => "invalid-policy-package",
                };
                item.result = query_rejection(code, error);
                return item;
            }
        }
    }
    item.result = match graph.route(&query.start_id, &query.destination_id, &query.options) {
        Ok(value) => QueryResult::Search(value),
        Err(error) => query_rejection(error_code(&error), error),
    };
    item
}

fn process(bytes: &[u8]) -> Response {
    let value = match parse_bounded_json(bytes) {
        Ok(value) => value,
        Err(error) => return response(SEARCH_VERSION, None, rejected("invalid-request", error)),
    };
    let version = match value
        .get("protocolVersion")
        .and_then(|value| value.as_str())
    {
        Some(SEARCH_VERSION) => SEARCH_VERSION,
        Some(POLICY_VERSION) => POLICY_VERSION,
        Some(VENUE_VERSION) => VENUE_VERSION,
        Some(COMPLETE_VERSION) => COMPLETE_VERSION,
        Some(_) => {
            return response(
                SEARCH_VERSION,
                value
                    .get("requestId")
                    .and_then(|id| id.as_str())
                    .filter(|id| !id.is_empty() && id.len() <= 256)
                    .map(str::to_owned),
                rejected(
                    "unsupported-protocol",
                    "expected protocolVersion 0.1.0, 0.2.0, 0.3.0 or 0.4.0",
                ),
            );
        }
        None => {
            return response(
                SEARCH_VERSION,
                None,
                rejected("invalid-request", "protocolVersion must be a string"),
            );
        }
    };
    let request = match legacy_protocol::parse(value, version) {
        Ok(request) => request,
        Err(error) => return response(version, None, rejected("invalid-request", error)),
    };
    if request.request_id.is_empty() || request.request_id.len() > 256 {
        return response(
            version,
            None,
            rejected(
                "invalid-request-id",
                "requestId requires 1 to 256 UTF-8 bytes",
            ),
        );
    }
    let request_id = request.request_id.clone();
    response(version, Some(request_id), execute(request))
}

fn main() -> io::Result<()> {
    support::serve(process, || {
        response(
            SEARCH_VERSION,
            None,
            rejected("frame-too-large", "input exceeds 16 MiB"),
        )
    })
}
