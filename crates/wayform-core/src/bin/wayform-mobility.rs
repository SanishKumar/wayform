mod support;

use std::io;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;
use wayform_core::{
    Document, InputDigests, LoadError, MobilityRouteError, MobilityRouteOutcome, MobilitySnapshot,
    PathAssessment, PathError, RouteDiagnostic, SearchError,
};

const PROTOCOL_VERSION: &str = "0.1.0";

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Operation {
    AssessPath,
    Route,
}

// A present edgeIds must be an array. Null must not collapse into an omitted
// route argument, and assess-path must retain its explicit path requirement.
fn present_path<'de, D: Deserializer<'de>>(input: D) -> Result<Option<Vec<String>>, D::Error> {
    Vec::<String>::deserialize(input).map(Some)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    protocol_version: String,
    request_id: String,
    operation: Operation,
    // Keep exact document JSON until the loader checks duplicate names, number
    // semantics and schema. Value would silently discard duplicate properties.
    bundle: Box<RawValue>,
    profile: Box<RawValue>,
    context: Box<RawValue>,
    start_id: String,
    destination_id: String,
    #[serde(default, deserialize_with = "present_path")]
    edge_ids: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    protocol_version: &'static str,
    request_id: Option<String>,
    snapshot: Option<InputDigests>,
    #[serde(flatten)]
    result: ResultBody,
}

#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
enum ResultBody {
    Assessed { assessment: Box<PathAssessment> },
    Routed { assessment: Box<PathAssessment> },
    InsufficientEvidence { diagnostic: RouteDiagnostic },
    Infeasible { diagnostic: RouteDiagnostic },
    Rejected { error: Failure },
}

#[derive(Serialize)]
struct Failure {
    code: &'static str,
    document: Option<&'static str>,
    message: String,
}

fn rejected(code: &'static str, document: Option<Document>, message: impl ToString) -> ResultBody {
    ResultBody::Rejected {
        error: Failure {
            code,
            document: document.map(|value| match value {
                Document::Bundle => "bundle",
                Document::Profile => "profile",
                Document::Context => "context",
            }),
            message: message.to_string().chars().take(512).collect(),
        },
    }
}

fn uncorrelated(code: &'static str, message: impl ToString) -> Response {
    Response {
        protocol_version: PROTOCOL_VERSION,
        request_id: None,
        snapshot: None,
        result: rejected(code, None, message),
    }
}

fn load_failure(error: LoadError) -> ResultBody {
    let (code, document) = match &error {
        LoadError::InvalidJson(document) => ("invalid-document", Some(*document)),
        LoadError::InvalidContract { document, .. } => ("invalid-contract", Some(*document)),
        LoadError::ResourceLimit => ("document-too-large", None),
        LoadError::Identity(_) => ("identity-error", None),
    };
    rejected(code, document, error)
}

fn execute(request: Request) -> Response {
    let mut response = Response {
        protocol_version: PROTOCOL_VERSION,
        request_id: Some(request.request_id),
        snapshot: None,
        result: rejected(
            "unsupported-protocol",
            None,
            "expected protocolVersion 0.1.0",
        ),
    };
    if request.protocol_version != PROTOCOL_VERSION {
        return response;
    }
    if matches!(request.operation, Operation::AssessPath) != request.edge_ids.is_some() {
        response.result = rejected(
            "invalid-operation-input",
            None,
            "assess-path requires edgeIds; route requires it to be omitted",
        );
        return response;
    }
    let snapshot = match MobilitySnapshot::from_json(
        request.bundle.get().as_bytes(),
        request.profile.get().as_bytes(),
        request.context.get().as_bytes(),
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            response.result = load_failure(error);
            return response;
        }
    };
    response.snapshot = Some(snapshot.input_digests().clone());
    response.result = match request.operation {
        Operation::AssessPath => match snapshot.assess_path(
            &request.start_id,
            &request.destination_id,
            request
                .edge_ids
                .as_deref()
                .expect("validated assess-path input"),
        ) {
            Ok(assessment) => ResultBody::Assessed {
                assessment: Box::new(assessment),
            },
            Err(error) => rejected(
                match error {
                    PathError::ResourceLimit => "path-limit-exceeded",
                    PathError::Eligibility(_) => "evaluation-failed",
                },
                None,
                error,
            ),
        },
        Operation::Route => match snapshot.route(&request.start_id, &request.destination_id) {
            Ok(MobilityRouteOutcome::Routed(assessment)) => ResultBody::Routed { assessment },
            Ok(MobilityRouteOutcome::InsufficientEvidence(diagnostic)) => {
                ResultBody::InsufficientEvidence { diagnostic }
            }
            Ok(MobilityRouteOutcome::Infeasible(diagnostic)) => {
                ResultBody::Infeasible { diagnostic }
            }
            Err(error) => rejected(
                match &error {
                    MobilityRouteError::Search(SearchError::WorkLimitExceeded) => {
                        "work-limit-exceeded"
                    }
                    MobilityRouteError::Search(SearchError::UnrepresentableCost(_)) => {
                        "unrepresentable-cost"
                    }
                    MobilityRouteError::Search(_) => "invalid-search",
                    MobilityRouteError::Path(PathError::ResourceLimit) => "path-limit-exceeded",
                    _ => "evaluation-failed",
                },
                None,
                error,
            ),
        },
    };
    response
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

fn main() -> io::Result<()> {
    support::serve(process, || {
        uncorrelated("frame-too-large", "request exceeds 16 MiB before LF")
    })
}
