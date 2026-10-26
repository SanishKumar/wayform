# Evaluated-graph search protocol

`wayform-search` accepts one UTF-8 JSON request per line on stdin and writes one JSON response per line on stdout, flushing each response. Protocol version `0.1.0` is separate from the mobility-data schema version. Build with `cargo build --package wayform-core --bin wayform-search --locked`.

This is a local integration boundary for already evaluated edges. The caller validates bundle/profile/context, evaluates every edge against the same snapshot, and supplies its decisions and preference penalties. Rust validates graph and search structure but does not verify that decisions agree with evidence. Snapshot digests are correlation labels echoed from the caller; they are neither authenticated nor recomputed. A response is not a mobility-contract receipt.

## Request

```json
{
  "protocolVersion": "0.1.0",
  "requestId": "example",
  "snapshot": {
    "bundleDigest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "profileDigest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    "contextDigest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  },
  "nodes": ["a", "b"],
  "edges": [{"id": "ab", "from": "a", "to": "b", "lengthMeters": 10}],
  "decisions": [{"edgeId": "ab", "suitability": "suitable", "penaltyMeters": 2}],
  "startId": "a",
  "destinationId": "b"
}
```

The digest values above are fictional. All fields are required; unknown and duplicate object fields are rejected. `requestId` requires 1–256 UTF-8 bytes and each digest requires 64 lowercase hexadecimal digits. Each edge needs exactly one decision, including edges that are unreachable from the requested start. Suitability is `suitable`, `unsuitable` or `unknown`. Reverse travel requires a separately identified edge and decision. Penalties are finite nonnegative metres added to length, never overrides for exclusions.

## Response

Every response includes `protocolVersion`, `requestId`, `snapshot` and `outcome`. Routed output adds `edgeIds`, `distanceMeters` and `costMeters`. For the example, the path is `["ab"]`, distance is 10 and cost is 12.

| Outcome | Meaning |
| --- | --- |
| `routed` | Shortest path using only suitable edges; ties use the complete UTF-16 lexical edge-ID sequence |
| `insufficient-evidence` | No suitable path exists, but allowing unknown edges would connect the endpoints |
| `infeasible` | Even suitable and unknown edges together cannot connect the endpoints |
| `rejected` | Invalid input or a resource/numeric limit; includes `error.code` and diagnostic `error.message` |

Only `routed` includes a path. A stationary request with known identical endpoints returns an empty path after validating the entire decision snapshot. Infeasibility is relative to the supplied graph and decisions.

Rejection codes are `invalid-request`, `invalid-request-id`, `unsupported-protocol`, `invalid-snapshot`, `invalid-graph`, `invalid-search`, `frame-too-large`, `unrepresentable-cost` and `work-limit-exceeded`. Codes are stable within this protocol version; messages are diagnostic text. Parse failures, oversized frames and invalid request IDs use null `requestId` and `snapshot`. After successful parsing and request-ID validation, both fields echo the request, including on rejection; an echoed snapshot may itself be invalid.

## Limits and recovery

Frames allow at most 16 MiB before LF, including whitespace and an optional CR. Oversized frames are drained to LF or EOF before processing resumes; their remaining bytes are not retained. An unterminated final line is accepted. Blank lines are rejected. EOF with no pending bytes produces no response. Malformed JSON, invalid UTF-8 and excessive JSON nesting are rejected by the parser. The graph accepts at most 100,000 nodes and 500,000 edges, subject to the smaller frame limit. The search's cumulative path-copy budget is 1,000,000 edge-index copies; exhaustion returns `work-limit-exceeded`.

Per-request rejection does not terminate the process. I/O failure exits unsuccessfully. This binary has no authentication, policy cache, network access or clock dependency; hosts need their own process lifetime and resource controls before exposing it as a service.

## Conformance

`npm run test:native` evaluates each fixture edge with the TypeScript supplied-path oracle, invokes this binary, then assesses every returned path again. It checks ten portable contract scenarios, five shortcut/detour cases and one stationary case, with input-order permutations. The reversed-path fixture becomes a valid search query whose outcome is infeasible. It does not establish native evidence/receipt parity or field navigability.

The driver defaults to `target/debug/wayform-search` (`.exe` on Windows), respects `CARGO_TARGET_DIR`, and accepts `WAYFORM_SEARCH_BIN` for an explicitly selected binary, such as a release build. The per-edge reference evaluation is intended for this small corpus; it repeatedly validates the bundle and is not a production bulk-evaluation implementation.
