use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use wayform_core::LEGACY_SOURCE_COMMIT;

fn query(id: &str) -> Value {
    json!({
        "queryId": id, "startId": "a", "destinationId": "d",
        "options": { "accessibleOnly": false, "allowRestricted": false, "closedEdgeIds": [] }
    })
}

fn request() -> Value {
    json!({
        "protocolVersion": "0.1.0", "sourceCommit": LEGACY_SOURCE_COMMIT, "requestId": "batch",
        "nodes": ["a", "b", "c", "d"].map(|id| json!({"id": id, "x": 0, "y": 0, "floor": "g"})),
        "edges": [
            {"id": "ab", "from": "a", "to": "b", "distance": 2, "accessible": false, "restricted": false},
            {"id": "bd", "from": "b", "to": "d", "distance": 2, "accessible": true, "restricted": false},
            {"id": "ac", "from": "a", "to": "c", "distance": 3, "accessible": true, "restricted": true},
            {"id": "cd", "from": "c", "to": "d", "distance": 3, "accessible": true, "restricted": false}
        ],
        "queries": [query("default")]
    })
}

fn run(input: Vec<u8>) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wayform-legacy-search"))
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
fn queries_preserve_order_and_isolate_options_and_failures() {
    let mut batch = request();
    let mut queries: Vec<Value> = [
        "default",
        "closed",
        "detour",
        "step-free",
        "step-free-restricted",
        "stationary",
        "unknown",
        "bad-closure",
        "again",
    ]
    .map(query)
    .into();
    for index in [1, 2] {
        queries[index]["options"]["closedEdgeIds"] = json!(["ab"]);
    }
    for index in [2, 3] {
        queries[index]["options"]["allowRestricted"] = json!(true);
    }
    for index in [3, 4] {
        queries[index]["options"]["accessibleOnly"] = json!(true);
    }
    queries[5]["destinationId"] = json!("a");
    queries[6]["destinationId"] = json!("missing");
    queries[7]["options"]["closedEdgeIds"] = json!(["missing"]);
    batch["queries"] = json!(queries);
    let responses = run(format!("{batch}\r\n{batch}").into_bytes());
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0], responses[1]);
    assert_eq!(responses[0]["sourceCommit"], LEGACY_SOURCE_COMMIT);
    assert_eq!(responses[0]["protocolVersion"], "0.1.0");
    assert_eq!(responses[0]["requestId"], "batch");
    assert_eq!(responses[0]["outcome"], "completed");
    let results = responses[0]["results"].as_array().unwrap();
    assert_eq!(results.len(), queries.len());
    for (index, outcome) in [
        "routed",
        "no-route",
        "routed",
        "routed",
        "no-route",
        "routed",
        "unknown-endpoint",
        "rejected",
        "routed",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(results[index]["queryId"], queries[index]["queryId"]);
        assert_eq!(results[index]["outcome"], *outcome);
    }
    for index in [0, 8] {
        assert_eq!(results[index]["pathIds"], json!(["a", "b", "d"]));
        assert_eq!(results[index]["totalDistance"], 4.0);
    }
    for index in [2, 3] {
        assert_eq!(results[index]["pathIds"], json!(["a", "c", "d"]));
        assert_eq!(results[index]["totalDistance"], 6.0);
    }
    assert_eq!(results[5]["pathIds"], json!(["a"]));
    assert_eq!(results[5]["totalDistance"], 0.0);
    assert_eq!(results[7]["error"]["code"], "invalid-closure");
}

#[test]
fn malformed_requests_are_rejected_before_any_batch_results() {
    let valid = request();
    let mut lines = vec!["{broken".to_owned(), "null".to_owned(), String::new()];
    let mut codes = vec!["invalid-request"; 3];
    let mut edits = Vec::new();
    for (key, value, code) in [
        ("protocolVersion", json!("future"), "unsupported-protocol"),
        ("sourceCommit", json!("unverified"), "unsupported-source"),
        ("requestId", json!(""), "invalid-request-id"),
        ("requestId", json!("é".repeat(129)), "invalid-request-id"),
        ("queries", json!([]), "invalid-batch"),
        (
            "queries",
            json!([query("same"), query("same")]),
            "invalid-batch",
        ),
        ("queries", json!([query("")]), "invalid-batch"),
        ("queries", json!([query(&"q".repeat(513))]), "invalid-batch"),
        (
            "queries",
            json!(
                (0..10_001)
                    .map(|i| query(&i.to_string()))
                    .collect::<Vec<_>>()
            ),
            "invalid-batch",
        ),
        ("unexpected", json!(true), "invalid-request"),
    ] {
        let mut input = valid.clone();
        input[key] = value;
        edits.push((input, code));
    }
    let mut missing_flag = valid.clone();
    missing_flag["edges"][0]
        .as_object_mut()
        .unwrap()
        .remove("accessible");
    edits.push((missing_flag, "invalid-request"));
    let mut missing_option = valid.clone();
    missing_option["queries"][0]["options"]
        .as_object_mut()
        .unwrap()
        .remove("closedEdgeIds");
    edits.push((missing_option, "invalid-request"));
    let mut invalid_graph = valid.clone();
    invalid_graph["edges"][0]["to"] = json!("missing");
    edits.push((invalid_graph, "invalid-graph"));
    for (input, code) in edits {
        lines.push(input.to_string());
        codes.push(code);
    }
    lines.push(
        valid
            .to_string()
            .replacen('{', "{\"requestId\":\"duplicate\",", 1),
    );
    codes.push("invalid-request");
    lines.push(valid.to_string().replacen(
        "\"accessible\":false",
        "\"accessible\":true,\"accessible\":false",
        1,
    ));
    codes.push("invalid-request");
    lines.push(valid.to_string());
    let responses = run(lines.join("\n").into_bytes());
    assert_eq!(responses.len(), codes.len() + 1);
    for (index, code) in codes.iter().enumerate() {
        assert_eq!(responses[index]["outcome"], "rejected", "case {index}");
        assert_eq!(responses[index]["error"]["code"], *code, "case {index}");
        assert!(responses[index].get("results").is_none());
        if matches!(*code, "invalid-request" | "invalid-request-id") {
            assert!(responses[index]["requestId"].is_null());
        } else {
            assert_eq!(responses[index]["requestId"], "batch");
        }
    }
    assert_eq!(responses.last().unwrap()["outcome"], "completed");
}

#[test]
fn numeric_query_errors_do_not_become_no_route_or_poison_later_queries() {
    let mut batch = request();
    batch["edges"][0]["distance"] = json!(1e300);
    batch["edges"][1]["distance"] = json!(1);
    let mut stationary = query("stationary");
    stationary["destinationId"] = json!("a");
    batch["queries"] = json!([query("precision"), stationary]);
    let responses = run(batch.to_string().into_bytes());
    assert_eq!(responses[0]["outcome"], "completed");
    assert_eq!(responses[0]["results"][0]["outcome"], "rejected");
    assert_eq!(
        responses[0]["results"][0]["error"]["code"],
        "unrepresentable-cost"
    );
    assert_eq!(responses[0]["results"][1]["outcome"], "routed");
}

#[test]
fn oversized_output_rejects_the_whole_batch_and_recovers() {
    let mut batch = request();
    // Small endpoints keep queries compact while the path contains long IDs.
    let ids: Vec<String> = std::iter::once("a".to_owned())
        .chain((0..100).map(|i| format!("{i}-{}", "x".repeat(1024))))
        .chain(std::iter::once("d".to_owned()))
        .collect();
    batch["nodes"] = json!(
        ids.iter()
            .map(|id| json!({"id": id, "x": 0, "y": 0, "floor": 0}))
            .collect::<Vec<_>>()
    );
    batch["edges"] = json!(
        ids.windows(2)
            .enumerate()
            .map(|(i, pair)| json!({
                "id": format!("e{i}"), "from": pair[0], "to": pair[1], "distance": 1,
                "accessible": true, "restricted": false
            }))
            .collect::<Vec<_>>()
    );
    batch["queries"] = json!(
        (0..200)
            .map(|i| query(&format!("q{i}")))
            .collect::<Vec<_>>()
    );
    batch["requestId"] = json!("\n".repeat(256));
    let encoded = batch.to_string();
    assert!(encoded.len() < 16 * 1024 * 1024);
    let responses = run(format!("{encoded}\n{}", request()).into_bytes());
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["outcome"], "rejected");
    assert_eq!(responses[0]["error"]["code"], "response-too-large");
    assert!(responses[0].get("results").is_none());
    assert_eq!(responses[0]["requestId"], batch["requestId"]);
    assert_eq!(responses[1]["outcome"], "completed");
}

#[test]
fn oversized_and_non_utf8_input_frames_recover() {
    let mut input = vec![b' '; 16 * 1024 * 1024 + 1];
    input.extend_from_slice(b"\n\xff\n");
    input.extend_from_slice(request().to_string().as_bytes());
    let responses = run(input);
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["error"]["code"], "frame-too-large");
    assert_eq!(responses[1]["error"]["code"], "invalid-request");
    assert!(responses[0]["requestId"].is_null());
    assert!(responses[1]["requestId"].is_null());
    assert_eq!(responses[2]["outcome"], "completed");
}
