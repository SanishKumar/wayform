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
    let nodes = ["a", "b", "c", "d"].map(|id| json!({"id": id, "x": 0, "y": 0, "floor": "g"}));
    json!({
        "protocolVersion": "0.1.0", "sourceCommit": LEGACY_SOURCE_COMMIT, "requestId": "batch",
        "nodes": nodes,
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

fn overlay() -> Value {
    json!({
        "schemaVersion": "0.1.0", "id": "outage", "buildingId": "fixture", "packageHash": "a".repeat(64),
        "validFrom": "2026-07-22T08:00:00.000Z", "validUntil": "2026-07-23T00:00:00.000Z",
        "closures": [{"id": "close-lift", "target": {"type": "source", "id": "lift"}, "reason": "Fixture"}]
    })
}

fn policy_query(id: &str, active: bool) -> Value {
    let mut input = query(id);
    input["options"]
        .as_object_mut()
        .unwrap()
        .remove("closedEdgeIds");
    input["policy"] = if active {
        json!({"mode": "overlay", "overlay": overlay(), "evaluatedAt": "2026-07-22T12:00:00.000Z"})
    } else {
        json!({"mode": "none"})
    };
    input
}

fn policy_request() -> Value {
    let mut input = request();
    input["protocolVersion"] = json!("0.2.0");
    input["edges"][0]["accessible"] = json!(true);
    input["edges"][2]["restricted"] = json!(false);
    input["edges"][2]["accessible"] = json!(false);
    input["package"] = json!({
        "buildingId": "fixture", "packageHash": "a".repeat(64),
        "edges": [
            {"id": "ab", "sourceId": "lift"}, {"id": "bd", "sourceId": "lift"},
            {"id": "ac", "sourceId": "stairs"}, {"id": "cd", "sourceId": "stairs"}
        ]
    });
    input["queries"] = json!([policy_query("none", false)]);
    input
}

#[test]
fn native_policy_routing_blocks_invalid_overlays_before_search_and_recovers() {
    let mut input = policy_request();
    let mut queries = vec![
        policy_query("none", false),
        policy_query("detour", true),
        policy_query("step-free", true),
    ];
    queries[2]["options"]["accessibleOnly"] = json!(true);
    for (id, destination) in [
        ("expired", "d"),
        ("stationary-expired", "a"),
        ("unknown-expired", "missing"),
    ] {
        let mut item = policy_query(id, true);
        item["destinationId"] = json!(destination);
        item["policy"]["evaluatedAt"] = json!("2026-07-23T00:00:00.000Z");
        queries.push(item);
    }
    let mut mismatch = policy_query("package-mismatch", true);
    mismatch["policy"]["overlay"]["packageHash"] = json!("b".repeat(64));
    queries.push(mismatch);
    let mut null = policy_query("null-overlay", true);
    null["policy"]["overlay"] = Value::Null;
    queries.push(null);
    let mut no_clock = policy_query("empty-clock", true);
    no_clock["policy"]["evaluatedAt"] = json!("");
    queries.push(no_clock);
    queries.push(policy_query("after", false));
    input["queries"] = json!(queries);
    let responses = run(format!("{input}\n{}", request()).into_bytes());
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["protocolVersion"], "0.2.0");
    assert_eq!(responses[0]["outcome"], "completed");
    let results = responses[0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 10);
    for index in [0, 9] {
        assert_eq!(results[index]["pathIds"], json!(["a", "b", "d"]));
        assert!(results[index].get("overlayResolution").is_none());
    }
    assert_eq!(results[1]["pathIds"], json!(["a", "c", "d"]));
    assert_eq!(results[1]["totalDistance"], 6.0);
    assert_eq!(
        results[1]["overlayResolution"]["closedEdgeIds"],
        json!(["ab", "bd"])
    );
    assert_eq!(results[2]["outcome"], "no-route");
    assert_eq!(results[2]["overlayResolution"]["valid"], true);
    for (offset, code) in [
        "overlay-expired",
        "overlay-expired",
        "overlay-expired",
        "package-mismatch",
        "invalid-shape",
        "invalid-time",
    ]
    .iter()
    .enumerate()
    {
        let result = &results[offset + 3];
        assert_eq!(result["outcome"], "rejected");
        assert_eq!(result["error"]["code"], "invalid-overlay");
        assert_eq!(result["overlayResolution"]["valid"], false);
        assert_eq!(result["overlayResolution"]["issues"][0]["code"], *code);
        assert!(result.get("pathIds").is_none());
        assert!(result.get("totalDistance").is_none());
    }
    assert_eq!(responses[1]["protocolVersion"], "0.1.0");
    assert_eq!(responses[1]["outcome"], "completed");
}

#[test]
fn protocol_versions_cannot_omit_or_override_policy_inputs() {
    let valid = policy_request();
    let mut inputs = Vec::new();
    let mut add = |input: Value, code: &str| inputs.push((input.to_string(), code.to_owned()));
    let mut missing_package = valid.clone();
    missing_package.as_object_mut().unwrap().remove("package");
    add(missing_package, "invalid-request");
    let mut missing_policy = valid.clone();
    missing_policy["queries"][0]
        .as_object_mut()
        .unwrap()
        .remove("policy");
    add(missing_policy, "invalid-request");
    for policy in [
        Value::Null,
        json!("none"),
        json!({"mode": "none", "overlay": overlay()}),
        json!({"mode": "none", "evaluatedAt": "2026-07-22T12:00:00.000Z"}),
        json!({"mode": "none", "closedEdgeIds": []}),
        json!({"mode": "overlay", "evaluatedAt": "2026-07-22T12:00:00.000Z"}),
        json!({"mode": "overlay", "overlay": overlay()}),
        json!({"mode": "invented"}),
    ] {
        let mut input = valid.clone();
        input["queries"][0]["policy"] = policy;
        add(input, "invalid-request");
    }
    let mut supplied_closures = valid.clone();
    supplied_closures["queries"][0]["options"]["closedEdgeIds"] = json!([]);
    add(supplied_closures, "invalid-request");
    let mut old_version_policy = request();
    old_version_policy["queries"][0]["policy"] = json!({"mode": "none"});
    add(old_version_policy, "invalid-request");
    for kind in [
        "missing-edge",
        "extra-edge",
        "duplicate-edge",
        "empty-source",
        "bad-hash",
    ] {
        let mut input = valid.clone();
        match kind {
            "missing-edge" => {
                input["package"]["edges"].as_array_mut().unwrap().pop();
            }
            "extra-edge" => input["package"]["edges"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id": "other", "sourceId": "lift"})),
            "duplicate-edge" => input["package"]["edges"][1]["id"] = json!("ab"),
            "empty-source" => input["package"]["edges"][0]["sourceId"] = json!(""),
            _ => input["package"]["packageHash"] = json!("unverified"),
        }
        add(input, "invalid-policy-package");
    }
    let mut overlay_input = valid.clone();
    overlay_input["queries"] = json!([policy_query("overlay", true)]);
    inputs.push((
        overlay_input.to_string().replacen(
            "\"validUntil\":",
            "\"validUntil\":\"2099-01-01T00:00:00.000Z\",\"validUntil\":",
            1,
        ),
        "invalid-request".into(),
    ));
    inputs.push((
        overlay_input.to_string().replacen(
            "\"reason\":",
            "\"rea\\u0073on\":\"duplicate\",\"reason\":",
            1,
        ),
        "invalid-request".into(),
    ));
    let encoded = inputs
        .iter()
        .map(|(line, _)| line.as_str())
        .chain(std::iter::once(""))
        .collect::<Vec<_>>()
        .join("\n")
        + &valid.to_string();
    let responses = run(encoded.into_bytes());
    assert_eq!(responses.len(), inputs.len() + 1);
    for (index, (response, (_, code))) in responses.iter().zip(&inputs).enumerate() {
        assert_eq!(response["outcome"], "rejected", "case {index}: {response}");
        assert_eq!(response["error"]["code"], *code, "case {index}: {response}");
        assert!(response.get("results").is_none());
    }
    assert_eq!(responses.last().unwrap()["outcome"], "completed");
}

#[test]
fn positional_arrays_cannot_replace_protocol_records() {
    let mut inputs = Vec::new();
    for base in [request(), policy_request()] {
        for kind in ["node", "edge", "query", "options"] {
            let mut input = base.clone();
            match kind {
                "node" => input["nodes"][0] = json!(["a", 0, 0, "g"]),
                "edge" => input["edges"][0] = json!(["ab", "a", "b", 2, false, false]),
                "query" => {
                    input["queries"][0] =
                        json!(["default", "a", "d", base["queries"][0]["options"]])
                }
                _ => {
                    input["queries"][0]["options"] = if base["protocolVersion"] == "0.1.0" {
                        json!([false, false, []])
                    } else {
                        json!([false, false])
                    }
                }
            }
            inputs.push(input);
        }
    }
    let mut package = policy_request();
    package["package"] = json!(["fixture", "a".repeat(64), package["package"]["edges"]]);
    inputs.push(package);
    let mut edge = policy_request();
    edge["package"]["edges"][0] = json!(["ab", "lift"]);
    inputs.push(edge);
    let mut policy = policy_request();
    policy["queries"][0]["policy"] = json!(["none"]);
    inputs.push(policy);
    let responses = run(inputs
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes());
    assert_eq!(responses.len(), inputs.len());
    for response in responses {
        assert_eq!(response["error"]["code"], "invalid-request");
        assert!(response.get("results").is_none());
    }
}

#[test]
fn policy_resource_exhaustion_is_not_an_empty_closure_set() {
    let mut input = policy_request();
    let mut excessive = policy_query("excessive", true);
    excessive["policy"]["overlay"]["closures"] =
        Value::Array(vec![Value::Null; wayform_core::MAX_EDGES + 1]);
    input["queries"] = json!([excessive, policy_query("later", false)]);
    let responses = run(input.to_string().into_bytes());
    let results = responses[0]["results"].as_array().unwrap();
    assert_eq!(results[0]["outcome"], "rejected");
    assert_eq!(results[0]["error"]["code"], "work-limit-exceeded");
    assert!(results[0].get("pathIds").is_none());
    assert!(results[0].get("overlayResolution").is_none());
    assert_eq!(results[1]["outcome"], "routed");
}

#[test]
fn policy_resolution_output_counts_toward_the_batch_byte_limit() {
    let mut input = policy_request();
    let ids: Vec<_> = (0..200)
        .map(|i| format!("{i}-{}", "x".repeat(1024)))
        .collect();
    input["edges"] = json!(ids.iter().map(|id| json!({"id": id, "from": "a", "to": "d", "distance": 1, "accessible": true, "restricted": false})).collect::<Vec<_>>());
    input["package"]["edges"] = json!(
        ids.iter()
            .map(|id| json!({"id": id, "sourceId": "lift"}))
            .collect::<Vec<_>>()
    );
    input["queries"] = json!(
        (0..100)
            .map(|i| policy_query(&i.to_string(), true))
            .collect::<Vec<_>>()
    );
    let encoded = input.to_string();
    assert!(encoded.len() < 16 * 1024 * 1024);
    let responses = run(format!("{encoded}\n{}", policy_request()).into_bytes());
    assert_eq!(responses[0]["error"]["code"], "response-too-large");
    assert!(responses[0].get("results").is_none());
    assert_eq!(responses[1]["outcome"], "completed");
}
