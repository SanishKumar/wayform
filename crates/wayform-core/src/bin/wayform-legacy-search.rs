mod support;

use std::collections::HashSet;
use std::io;

use serde::{Deserialize, Serialize};
use wayform_core::{
    LEGACY_SOURCE_COMMIT, LegacyEdge, LegacyGraph, LegacyNode, LegacyRouteOptions,
    LegacySearchError, LegacySearchOutcome,
};

const PROTOCOL_VERSION: &str = "0.1.0";
const MAX_QUERIES: usize = 10_000;
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    protocol_version: String,
    source_commit: String,
    request_id: String,
    nodes: Vec<LegacyNode>,
    edges: Vec<LegacyEdge>,
    queries: Vec<Query>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Query {
    query_id: String,
    start_id: String,
    destination_id: String,
    options: LegacyRouteOptions,
}

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
    #[serde(flatten)]
    result: QueryResult,
}

#[derive(Serialize)]
#[serde(untagged)]
enum QueryResult {
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

fn response(request_id: Option<String>, result: BatchResult) -> Response {
    Response {
        protocol_version: PROTOCOL_VERSION,
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
    if request.protocol_version != PROTOCOL_VERSION {
        return rejected("unsupported-protocol", "expected protocolVersion 0.1.0");
    }
    if request.source_commit != LEGACY_SOURCE_COMMIT {
        return rejected("unsupported-source", "unsupported reference source commit");
    }
    if request.queries.is_empty() || request.queries.len() > MAX_QUERIES {
        return rejected("invalid-batch", "expected between 1 and 10000 queries");
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
        Some(request.request_id),
        BatchResult::Completed {
            results: Vec::new(),
        },
    );
    let mut response_bytes = match serde_json::to_vec(&empty) {
        Ok(bytes) => bytes.len(),
        Err(error) => return rejected("encoding-failed", error),
    };
    let graph = match LegacyGraph::new(request.nodes, request.edges) {
        Ok(graph) => graph,
        Err(error) => return rejected(error_code(&error), error),
    };
    let mut results = Vec::with_capacity(request.queries.len());
    for query in request.queries {
        let result = match graph.route(&query.start_id, &query.destination_id, &query.options) {
            Ok(value) => QueryResult::Search(value),
            Err(error) => QueryResult::Rejected {
                outcome: "rejected",
                error: failure(error_code(&error), error),
            },
        };
        let item = QueryResponse {
            query_id: query.query_id,
            result,
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

fn process(bytes: &[u8]) -> Response {
    let request: Request = match serde_json::from_slice(bytes) {
        Ok(request) => request,
        Err(error) => return response(None, rejected("invalid-request", error)),
    };
    if request.request_id.is_empty() || request.request_id.len() > 256 {
        return response(
            None,
            rejected(
                "invalid-request-id",
                "requestId requires 1 to 256 UTF-8 bytes",
            ),
        );
    }
    let request_id = request.request_id.clone();
    response(Some(request_id), execute(request))
}

fn main() -> io::Result<()> {
    support::serve(process, || {
        response(None, rejected("frame-too-large", "input exceeds 16 MiB"))
    })
}
