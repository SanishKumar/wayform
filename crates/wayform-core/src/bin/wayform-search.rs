use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use wayform_core::{Edge, EdgeDecision, Graph, SearchError, SearchOutcome, Suitability};

const PROTOCOL_VERSION: &str = "0.1.0";
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    protocol_version: String,
    request_id: String,
    snapshot: Snapshot,
    nodes: Vec<String>,
    edges: Vec<InputEdge>,
    decisions: Vec<InputDecision>,
    start_id: String,
    destination_id: String,
}

/// Caller-provided correlation labels, not identities verified by this binary.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Snapshot {
    bundle_digest: String,
    profile_digest: String,
    context_digest: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InputEdge {
    id: String,
    from: String,
    to: String,
    length_meters: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InputDecision {
    edge_id: String,
    suitability: InputSuitability,
    penalty_meters: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum InputSuitability {
    Suitable,
    Unsuitable,
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    protocol_version: &'static str,
    request_id: Option<String>,
    snapshot: Option<Snapshot>,
    #[serde(flatten)]
    result: ResultBody,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
enum ResultBody {
    Routed {
        #[serde(rename = "edgeIds")]
        edge_ids: Vec<String>,
        #[serde(rename = "distanceMeters")]
        distance_meters: f64,
        #[serde(rename = "costMeters")]
        cost_meters: f64,
    },
    InsufficientEvidence,
    Infeasible,
    Rejected {
        error: Failure,
    },
}

#[derive(Serialize)]
struct Failure {
    code: &'static str,
    message: String,
}

fn rejected(code: &'static str, message: impl ToString) -> ResultBody {
    ResultBody::Rejected {
        error: Failure {
            code,
            message: message.to_string(),
        },
    }
}

fn uncorrelated(code: &'static str, message: impl ToString) -> Response {
    Response {
        protocol_version: PROTOCOL_VERSION,
        request_id: None,
        snapshot: None,
        result: rejected(code, message),
    }
}

fn execute(request: Request) -> Response {
    let result = if request.protocol_version != PROTOCOL_VERSION {
        rejected("unsupported-protocol", "expected protocolVersion 0.1.0")
    } else if [
        &request.snapshot.bundle_digest,
        &request.snapshot.profile_digest,
        &request.snapshot.context_digest,
    ]
    .iter()
    .any(|digest| {
        digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        rejected(
            "invalid-snapshot",
            "digests require 64 lowercase hex digits",
        )
    } else {
        let graph = Graph::new(
            request.nodes,
            request
                .edges
                .into_iter()
                .map(|edge| Edge {
                    id: edge.id,
                    from: edge.from,
                    to: edge.to,
                    length_meters: edge.length_meters,
                })
                .collect(),
        );
        match graph {
            Err(error) => rejected("invalid-graph", error),
            Ok(graph) => {
                let decisions = request
                    .decisions
                    .into_iter()
                    .map(|decision| EdgeDecision {
                        edge_id: decision.edge_id,
                        suitability: match decision.suitability {
                            InputSuitability::Suitable => Suitability::Suitable,
                            InputSuitability::Unsuitable => Suitability::Unsuitable,
                            InputSuitability::Unknown => Suitability::Unknown,
                        },
                        penalty_meters: decision.penalty_meters,
                    })
                    .collect::<Vec<_>>();
                match graph.route(&request.start_id, &request.destination_id, &decisions) {
                    Ok(SearchOutcome::Routed(path)) => ResultBody::Routed {
                        edge_ids: path.edge_ids,
                        distance_meters: path.distance_meters,
                        cost_meters: path.cost_meters,
                    },
                    Ok(SearchOutcome::InsufficientEvidence) => ResultBody::InsufficientEvidence,
                    Ok(SearchOutcome::Infeasible) => ResultBody::Infeasible,
                    Err(error) => {
                        let code = match error {
                            SearchError::WorkLimitExceeded => "work-limit-exceeded",
                            SearchError::UnrepresentableCost(_) => "unrepresentable-cost",
                            _ => "invalid-search",
                        };
                        rejected(code, error)
                    }
                }
            }
        }
    };
    Response {
        protocol_version: PROTOCOL_VERSION,
        request_id: Some(request.request_id),
        snapshot: Some(request.snapshot),
        result,
    }
}

fn process(frame: &[u8]) -> Response {
    match serde_json::from_slice::<Request>(frame) {
        Err(error) => uncorrelated("invalid-request", error),
        Ok(request) if request.request_id.is_empty() || request.request_id.len() > 256 => {
            uncorrelated(
                "invalid-request-id",
                "requestId requires 1 to 256 UTF-8 bytes",
            )
        }
        Ok(request) => execute(request),
    }
}

enum Frame {
    Json(Vec<u8>),
    Oversized,
}

/// Drain oversized frames through LF/EOF without retaining their remaining bytes.
fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Frame>> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(if oversized {
                Some(Frame::Oversized)
            } else if bytes.is_empty() {
                None
            } else {
                Some(Frame::Json(bytes))
            });
        }
        let newline = available.iter().position(|&byte| byte == b'\n');
        let count = newline.unwrap_or(available.len());
        if !oversized {
            if count > MAX_FRAME_BYTES - bytes.len() {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..count]);
            }
        }
        reader.consume(count + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Some(if oversized {
                Frame::Oversized
            } else {
                Frame::Json(bytes)
            }));
        }
    }
}

fn main() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    while let Some(frame) = read_frame(&mut input)? {
        let response = match frame {
            Frame::Json(bytes) => process(&bytes),
            Frame::Oversized => uncorrelated("frame-too-large", "request exceeds 16 MiB before LF"),
        };
        serde_json::to_writer(&mut output, &response).map_err(io::Error::other)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
