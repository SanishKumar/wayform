use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use wayform_core::MAX_PATH_EDGES;

fn request() -> Value {
    let document: Value = serde_json::from_str(include_str!(
        "../../../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json"
    ))
    .unwrap();
    let item = &document["examples"][0];
    json!({
        "protocolVersion": "0.1.0", "requestId": "example", "operation": "assess-path",
        "bundle": item["bundle"], "profile": item["profile"], "context": item["context"],
        "startId": "a", "destinationId": "b", "edgeIds": ["ab"],
    })
}

fn run(input: Vec<u8>) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wayform-mobility"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(&input).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn assessments_compute_identities_and_keep_each_request_isolated() {
    let valid = request();
    let mut unknown = valid.clone();
    unknown["bundle"]["observations"] = json!([]);
    let mut narrow = valid.clone();
    narrow["bundle"]["observations"][0]["value"] = json!(0.5);
    let mut reverse = valid.clone();
    reverse["startId"] = json!("b");
    reverse["destinationId"] = json!("a");
    let mut profile = valid.clone();
    profile["profile"]["hard"]["minClearWidthMeters"] = json!(2);
    let responses = run([&valid, &unknown, &narrow, &reverse, &profile, &valid]
        .map(Value::to_string)
        .join("\r\n")
        .into_bytes());
    assert_eq!(responses.len(), 6);
    assert_eq!(responses[0], responses[5]);
    for (index, status) in ["suitable", "unknown", "unsuitable", "invalid", "unsuitable"]
        .iter()
        .enumerate()
    {
        assert_eq!(responses[index]["outcome"], "assessed");
        assert_eq!(responses[index]["assessment"]["status"], *status);
        assert_eq!(responses[index]["requestId"], "example");
        for key in ["bundleDigest", "profileDigest", "contextDigest"] {
            assert_eq!(
                responses[index]["snapshot"][key],
                responses[index]["assessment"]["receipt"][key]
            );
        }
    }
    assert_ne!(
        responses[0]["snapshot"]["bundleDigest"],
        responses[1]["snapshot"]["bundleDigest"]
    );
    assert_ne!(
        responses[0]["snapshot"]["profileDigest"],
        responses[4]["snapshot"]["profileDigest"]
    );
}

#[test]
fn malformed_envelopes_and_documents_recover_without_claiming_an_identity() {
    let valid = request();
    let mut lines = vec!["{broken".to_owned(), String::new(), "null".to_owned()];
    for (key, value) in [
        ("snapshot", json!({"bundleDigest": "a".repeat(64)})),
        ("decisions", json!([])),
        ("edgeIds", Value::Null),
        ("edgeIds", json!([1])),
        ("operation", json!("invented-operation")),
    ] {
        let mut input = valid.clone();
        input[key] = value;
        lines.push(input.to_string());
    }
    lines.push(
        valid
            .to_string()
            .replacen('{', "{\"requestId\":\"example\",", 1),
    );
    for document in ["bundle", "profile", "context"] {
        lines.push(valid.to_string().replacen(
            &format!("\"{document}\":{{"),
            &format!("\"{document}\":{{\"duplicate\":1,\"dupli\\u0063ate\":2,"),
            1,
        ));
    }
    let mut malformed = valid.clone();
    malformed["bundle"]["nodes"][0]["frameId"] = json!("missing");
    lines.push(malformed.to_string());
    lines.push(valid.to_string());
    let responses = run(lines.join("\n").into_bytes());
    assert_eq!(responses.len(), lines.len());
    for response in &responses[..9] {
        assert_eq!(response["error"]["code"], "invalid-request");
        assert!(response["requestId"].is_null());
        assert!(response["snapshot"].is_null());
    }
    for (response, document) in responses[9..12]
        .iter()
        .zip(["bundle", "profile", "context"])
    {
        assert_eq!(response["error"]["code"], "invalid-document");
        assert_eq!(response["error"]["document"], document);
        assert_eq!(response["requestId"], "example");
        assert!(response["snapshot"].is_null());
    }
    assert_eq!(responses[12]["error"]["code"], "invalid-contract");
    assert_eq!(responses[12]["error"]["document"], "bundle");
    assert!(responses[12]["snapshot"].is_null());
    assert_eq!(responses[13]["assessment"]["status"], "suitable");
}

#[test]
fn request_ids_versions_and_path_limits_have_stable_errors() {
    let valid = request();
    let mutations = [
        ("protocolVersion", json!("future"), "unsupported-protocol"),
        ("requestId", json!(""), "invalid-request-id"),
        ("requestId", json!("é".repeat(129)), "invalid-request-id"),
        (
            "edgeIds",
            json!(vec!["ab"; MAX_PATH_EDGES + 1]),
            "path-limit-exceeded",
        ),
    ];
    let lines: Vec<_> = mutations
        .iter()
        .map(|(key, value, _)| {
            let mut input = valid.clone();
            input[*key] = value.clone();
            input.to_string()
        })
        .collect();
    let responses = run(lines.join("\n").into_bytes());
    for (response, (_, _, code)) in responses.iter().zip(mutations) {
        assert_eq!(response["error"]["code"], code);
    }
    assert!(responses[0]["snapshot"].is_null());
    assert!(responses[3]["snapshot"]["bundleDigest"].is_string());
    let mut maximum_id = valid;
    maximum_id["requestId"] = json!("é".repeat(128));
    assert_eq!(
        run(maximum_id.to_string().into_bytes())[0]["outcome"],
        "assessed"
    );
}

#[test]
fn frame_boundaries_invalid_utf8_and_trailing_eof_preserve_the_stream() {
    const MAX_FRAME: usize = 16 * 1024 * 1024;
    let valid = request().to_string();
    let mut input = valid.as_bytes().to_vec();
    input.resize(MAX_FRAME, b' ');
    input.push(b'\n');
    input.extend(vec![b' '; MAX_FRAME + 1]);
    input.extend_from_slice(b"\n\xff\n");
    input.extend_from_slice(valid.as_bytes());
    let responses = run(input);
    assert_eq!(responses.len(), 4);
    assert_eq!(responses[0]["outcome"], "assessed");
    assert_eq!(responses[1]["error"]["code"], "frame-too-large");
    assert_eq!(responses[2]["error"]["code"], "invalid-request");
    assert_eq!(responses[0], responses[3]);
    assert!(run(Vec::new()).is_empty());
}

#[test]
fn route_requests_compute_decisions_and_keep_failure_paths_private() {
    let mut valid = request();
    valid["operation"] = json!("route");
    valid.as_object_mut().unwrap().remove("edgeIds");
    let mut unknown = valid.clone();
    unknown["bundle"]["observations"] = json!([]);
    let mut unsuitable = valid.clone();
    unsuitable["bundle"]["observations"][0]["value"] = json!(0.5);
    let mut reversed = valid.clone();
    reversed["startId"] = json!("b");
    reversed["destinationId"] = json!("a");
    let mut stationary = valid.clone();
    stationary["destinationId"] = json!("a");
    let responses = run([
        &valid,
        &unknown,
        &unsuitable,
        &reversed,
        &stationary,
        &valid,
    ]
    .map(Value::to_string)
    .join("\n")
    .into_bytes());
    assert_eq!(responses.len(), 6);
    assert_eq!(responses[0], responses[5]);
    assert_eq!(responses[0]["outcome"], "routed");
    assert_eq!(responses[0]["assessment"]["status"], "suitable");
    assert_eq!(
        responses[0]["assessment"]["receipt"]["edgeIds"],
        json!(["ab"])
    );
    for (index, outcome) in [
        (1, "insufficient-evidence"),
        (2, "infeasible"),
        (3, "infeasible"),
    ] {
        assert_eq!(responses[index]["outcome"], outcome);
        assert!(responses[index].get("assessment").is_none());
        assert!(responses[index].get("edgeIds").is_none());
        assert!(responses[index]["diagnostic"].get("edgeIds").is_none());
        assert!(responses[index]["snapshot"].is_object());
    }
    assert_eq!(
        responses[1]["diagnostic"]["reasonCodes"],
        json!(["missing-measurement"])
    );
    assert_eq!(responses[4]["assessment"]["receipt"]["edgeIds"], json!([]));
}

#[test]
fn operations_reject_missing_or_extraneous_paths_before_loading() {
    let mut missing = request();
    missing.as_object_mut().unwrap().remove("edgeIds");
    let mut extra = request();
    extra["operation"] = json!("route");
    let mut null = extra.clone();
    null["edgeIds"] = Value::Null;
    let mut unknown = extra.clone();
    unknown.as_object_mut().unwrap().remove("edgeIds");
    unknown["startId"] = json!("absent");
    let responses = run([missing, extra, null, unknown, request()]
        .map(|value| value.to_string())
        .join("\n")
        .into_bytes());
    for response in &responses[..2] {
        assert_eq!(response["error"]["code"], "invalid-operation-input");
        assert_eq!(response["requestId"], "example");
        assert!(response["snapshot"].is_null());
    }
    assert_eq!(responses[2]["error"]["code"], "invalid-request");
    assert_eq!(responses[3]["error"]["code"], "invalid-search");
    assert!(responses[3]["snapshot"].is_object());
    assert_eq!(responses[4]["outcome"], "assessed");
}
