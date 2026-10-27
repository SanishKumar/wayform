mod support;

use std::io;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use wayform_core::{
    Document, InputDigests, LoadError, MobilitySnapshot, PathAssessment, PathError,
};

const PROTOCOL_VERSION: &str = "0.1.0";

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Operation {
    AssessPath,
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
    edge_ids: Vec<String>,
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
            &request.edge_ids,
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
