use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn request() -> Value {
    json!({
        "protocolVersion": "0.1.0", "requestId": "example",
        "snapshot": {"bundleDigest": "a".repeat(64), "profileDigest": "b".repeat(64), "contextDigest": "c".repeat(64)},
        "nodes": ["a", "b"],
        "edges": [{"id": "ab", "from": "a", "to": "b", "lengthMeters": 10}],
        "decisions": [{"edgeId": "ab", "suitability": "suitable", "penaltyMeters": 2}],
        "startId": "a", "destinationId": "b"
    })
}

fn run(input: Vec<u8>) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wayform-search"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    // Drain output concurrently, including on platforms with small pipe buffers.
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
fn requests_are_isolated_and_diagnostics_do_not_expose_unknown_paths() {
    let valid = request();
    let mut unknown = valid.clone();
    unknown["decisions"][0]["suitability"] = json!("unknown");
    let mut closed = valid.clone();
    closed["decisions"][0]["suitability"] = json!("unsuitable");
    let mut reverse = valid.clone();
    reverse["startId"] = json!("b");
    reverse["destinationId"] = json!("a");
    let input = [&valid, &unknown, &closed, &reverse, &valid]
        .map(Value::to_string)
        .join("\r\n");
    let responses = run(input.into_bytes());
    assert_eq!(responses.len(), 5);
    assert_eq!(responses[0], responses[4]);
    assert_eq!(responses[0]["protocolVersion"], "0.1.0");
    assert_eq!(responses[0]["requestId"], valid["requestId"]);
    assert_eq!(responses[0]["snapshot"], valid["snapshot"]);
    assert_eq!(responses[0]["edgeIds"], json!(["ab"]));
    assert_eq!(responses[0]["distanceMeters"], 10.0);
    assert_eq!(responses[0]["costMeters"], 12.0);
    for (index, outcome) in [
        (1, "insufficient-evidence"),
        (2, "infeasible"),
        (3, "infeasible"),
    ] {
        assert_eq!(responses[index]["outcome"], outcome);
        assert!(responses[index].get("edgeIds").is_none());
    }
}

#[test]
fn malformed_and_incomplete_requests_are_rejected_without_stopping_the_stream() {
    let mutations = [
        ("unknown field", json!(true), "invalid-request"),
        ("protocolVersion", json!("future"), "unsupported-protocol"),
        ("requestId", json!(""), "invalid-request-id"),
        ("requestId", json!("x".repeat(257)), "invalid-request-id"),
        (
            "snapshot",
            json!({"bundleDigest": "bad", "profileDigest": "b".repeat(64), "contextDigest": "c".repeat(64)}),
            "invalid-snapshot",
        ),
        ("nodes", json!(["a", "a"]), "invalid-graph"),
        ("decisions", json!([]), "invalid-search"),
        ("startId", json!("absent"), "invalid-search"),
        (
            "edges",
            json!([{"id": "ab", "from": "a", "to": "b", "lengthMeters": -1}]),
            "invalid-graph",
        ),
        (
            "decisions",
            json!([{"edgeId": "ab", "suitability": "maybe", "penaltyMeters": 0}]),
            "invalid-request",
        ),
        (
            "decisions",
            json!([{"edgeId": "ab", "suitability": "suitable", "penaltyMeters": -1}]),
            "invalid-search",
        ),
        (
            "edges",
            json!([{"id": "ab", "from": "a", "to": "b", "lengthMeters": 1, "extra": true}]),
            "invalid-request",
        ),
    ];
    let mut lines = vec!["{broken".to_owned(), String::new(), "null".to_owned()];
    for (key, value, _) in &mutations {
        let mut item = request();
        item[*key] = value.clone();
        lines.push(item.to_string());
    }
    // A duplicate JSON field is invalid even if both values agree.
    lines.push(
        request()
            .to_string()
            .replacen('{', "{\"requestId\":\"example\",", 1),
    );
    lines.push(request().to_string());
    let responses = run(lines.join("\n").into_bytes());
    assert_eq!(responses.len(), lines.len());
    for response in &responses[..3] {
        assert_eq!(response["error"]["code"], "invalid-request");
        assert!(response["requestId"].is_null());
    }
    for (response, (_, _, code)) in responses[3..].iter().zip(mutations) {
        assert_eq!(response["outcome"], "rejected");
        assert_eq!(response["error"]["code"], code);
    }
    assert_eq!(
        responses[responses.len() - 2]["error"]["code"],
        "invalid-request"
    );
    assert_eq!(responses.last().unwrap()["outcome"], "routed");
}

#[test]
fn oversized_and_non_utf8_frames_are_drained_before_the_next_request() {
    let mut input = vec![b' '; 16 * 1024 * 1024 + 1];
    input.extend_from_slice(b"\n\xff\n");
    input.extend_from_slice(request().to_string().as_bytes());
    let responses = run(input);
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["error"]["code"], "frame-too-large");
    assert_eq!(responses[1]["error"]["code"], "invalid-request");
    assert_eq!(responses[2]["outcome"], "routed");
}
