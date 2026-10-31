# Native legacy search component

`LegacyGraph` preserves the search portion of VoiceGIS `routingCore.ts` at commit `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`. It is separate from the directed mobility engine. The original source and rights statement remain in `compat/voicegis/upstream/`.

This component returns selected node IDs and total distance, or unknown-endpoint/no-route outcomes. The batch CLI also supports native operational-overlay validation before search. It does not yet load a complete venue package, verify package integrity, generate instructions, validate route graphics or issue the original route receipt. The existing TypeScript compatibility harness remains authoritative for those boundaries.

## Preserved behavior

- Edges are inserted in both directions, retaining input order. Parallel edges keep distinct IDs for closure filtering.
- The original minimum-priority heap behavior is reproduced without a secondary tie-breaker. Equal-cost paths can change when edges are reordered; that is intentional legacy behavior, unlike the mobility contract's ordering rules.
- Same-floor heuristic scale is computed from edge distance divided by coordinate distance. Cross-floor estimates are zero. Numeric and string floor IDs remain distinct.
- Restricted edges are excluded unless explicitly allowed. The step-free projection excludes authored `accessible: false` edges. Resolved closures exclude their exact edge IDs in either direction.
- Improved costs replace predecessors; obsolete queue entries are skipped. A known stationary endpoint returns its single node and zero distance.

The Euclidean calculation uses scaled binary64 arithmetic in the order used by the reference runtime's two-argument `Math.hypot`. The comparison reference is [Node 24.18.0's V8 implementation](https://github.com/nodejs/node/blob/v24.18.0/deps/v8/src/builtins/math.tq#L388-L404). Using a platform `hypot` implementation can change rounding and consequently priorities. This is a compatibility choice, not a claim that every JavaScript engine produces identical results. The heuristic scale is exposed for exact numerical conformance checks.

## Input boundary and limits

The host projects a verified runtime graph into `LegacyNode` and `LegacyEdge`, including explicit accessibility and restriction flags. The constructor rejects duplicate IDs, missing endpoints, invalid coordinates/floors, and nonpositive or nonfinite distances. It does not reproduce the old low-level helper's absent-flag fallback or silent skipping of malformed edges. Positive self-loops are accepted; they cannot improve a path cost.

Graph limits are 100,000 nodes and 500,000 edges. Closure IDs must exist; duplicate closure IDs are harmless. Search limits queue removals plus adjacency visits to one million. Overflowing coordinates/priorities and nonadvancing accumulated distances return `UnrepresentableCost`. Limit and validation failures are errors, not no-route claims. These stricter boundaries are outside the selected-path compatibility claim.

## Conformance

Twenty-six small synthetic cases compare selected paths, distances and heuristic scales with the pinned TypeScript core. Cases cover equal priorities and reordered edges, stale queue entries, floors, closures, restrictions, legacy step-free flags, parallel edges, stationary paths, failures and very large/small coordinates. Native tests also exercise malformed inputs and explicit numeric errors.

The native CLI driver verifies all 35 upstream source blobs and loads the four verified packages. It recomputes each of the 7,238 full TypeScript results, checks its unchanged recorded fingerprint, and compares the native node path and distance under both protocol versions. The original version retains two lift-outage queries using resolved closure IDs and 26 component cases. The policy version adds 126 requests derived from the 58 overlay fixtures, explicit absence, and expired-policy stationary/unknown endpoints. Together these are 14,630 queries in 36 batches. Policy comparisons include complete resolutions and exact search results; a lift outage reroutes the standard profile and returns no route for the step-free profile.

Run `npm run check`, `cargo test --workspace --locked`, and `npm run test:native`. The latter builds the binaries and runs the cross-language comparisons. `WAYFORM_LEGACY_SEARCH_BIN` can select an already built executable; otherwise the driver uses `CARGO_TARGET_DIR` (default `target`) and its debug executable. It imposes a 60-second process timeout for this corpus, not a product latency guarantee.

Component expectations are only recorded with `node --import tsx scripts/record-legacy-search-examples.ts --record`. The original 7,238 full-result fingerprints remain unchanged. Native full-result parity remains incomplete: these comparisons do not establish native package validation, directions, graphics, original receipts or a running-journey expiry lease.

## Batch JSON-lines protocol

`wayform-legacy-search` accepts one projected graph and a nonempty ordered list of queries per line. Batching avoids repeating a venue graph for every journey. Version `0.1.0` accepts resolved closure IDs from the host. Version `0.2.0` resolves overlays natively. Protocol records must be objects, not positional arrays. All fields below are required; unknown and duplicate fields are rejected. The overlay payload exception is described under version `0.2.0`.

```json
{
  "protocolVersion": "0.1.0",
  "sourceCommit": "305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f",
  "requestId": "example",
  "nodes": [
    { "id": "a", "x": 0, "y": 0, "floor": "ground" },
    { "id": "b", "x": 1, "y": 0, "floor": "ground" }
  ],
  "edges": [
    { "id": "ab", "from": "a", "to": "b", "distance": 1, "accessible": true, "restricted": false }
  ],
  "queries": [
    { "queryId": "outbound", "startId": "a", "destinationId": "b", "options": { "accessibleOnly": false, "allowRestricted": false, "closedEdgeIds": [] } }
  ]
}
```

This formatted example must be serialized on one line. A completed response contains `protocolVersion`, `sourceCommit`, `requestId`, `outcome: "completed"` and ordered `results`. Each result contains its `queryId` and one of:

- `outcome: "routed"`, `pathIds` and `totalDistance`;
- `outcome: "unknown-endpoint"` or `outcome: "no-route"`;
- `outcome: "rejected"` and `error: { code, message }` for a query-specific validation, numeric or work-limit error.

Queries do not mutate the graph or one another's options. A bad closure ID rejects only its query. Invalid envelopes, invalid graphs and excessive response sizes reject the whole batch with `outcome: "rejected"` and `error`, without partial results. An accepted request ID is echoed on these failures. Malformed JSON, invalid request IDs and oversized frames return `requestId: null`. Subsequent frames are still processed.

`sourceCommit` selects the supported reference behavior; it is **not** proof that the supplied graph came from that commit or a verified package. The host remains responsible for package verification and profile mapping. Under `0.1.0` it also validates overlays before supplying resolved closures. Responses identify the selected supported protocol; unparseable frames and unsupported versions use `0.1.0` for their error envelope. This protocol carries no mobility evidence or original route receipts.

### Version 0.2.0: validate policy before search

Keep the same graph fields and set `protocolVersion` to `0.2.0`. Add a required `package` projection with `buildingId`, `packageHash` and `edges: [{ id, sourceId }]`. Its edge IDs must match the search graph exactly, including uniqueness; missing, additional or duplicate IDs reject the batch with `invalid-policy-package`. The index also validates identity syntax and nonempty source IDs. This checks consistency of supplied projections, not the hash of a complete package or authenticity of the publisher.

Each query still has `queryId`, `startId` and `destinationId`. Its required `options` object contains only `accessibleOnly` and `allowRestricted`; supplying `closedEdgeIds`, even an empty array, is an error. Each query must explicitly choose one of these policy records:

```json
{ "mode": "none" }
```

```json
{
  "mode": "overlay",
  "evaluatedAt": "2026-07-22T12:00:00.000Z",
  "overlay": {
    "schemaVersion": "0.1.0",
    "id": "example-outage",
    "buildingId": "example",
    "packageHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "validFrom": "2026-07-22T08:00:00.000Z",
    "validUntil": "2026-07-23T00:00:00.000Z",
    "closures": [{ "id": "close-lift", "target": { "type": "source", "id": "lift" }, "reason": "Synthetic example" }]
  }
}
```

The example identity and conditions are fictional. The payload uses the [legacy overlay resolver](native-legacy-policies.md), including canonical UTC timestamps and its documented compatibility limits. Overlay payloads may contain extra fields, matching the legacy shape check. Duplicate JSON names are rejected at every level, including ignored payload extensions and escaped spellings of the same name.

An overlay resolves **before** endpoint handling or path search. Invalid shape, mismatched binding, future/expired time and unknown targets return query-level `rejected` with `error.code: "invalid-overlay"` and the complete `overlayResolution`. No path or distance is returned, including for stationary or unknown endpoints. Resource exhaustion returns `work-limit-exceeded` without a partial resolution. A valid resolution supplies the only closure IDs used by search; `overlayResolution` accompanies the resulting route, no-route, unknown-endpoint or search error.

With `mode: "none"`, routing uses no operational closures and omits `overlayResolution`. This is an explicit caller decision, not confirmation that a venue is open. A host must not use it as fallback after an invalid/expired policy or disappearance of an expected feed. Missing/null policy records, missing clocks or missing overlay fields reject the entire batch as malformed. An explicitly supplied null overlay instead resolves as invalid shape; an empty clock string resolves as invalid time. A rejected query does not affect subsequent queries.

The host still verifies package bytes and source mappings, authenticates policy inputs, supplies a trustworthy clock and manages expiry during a journey. The [native lease API](native-legacy-leases.md) provides a stateful expiry check for that integration. The CLI evaluates the supplied snapshot only; it does not create a lease or issue a complete legacy route receipt.

### Bounds and error codes

Request IDs require 1–256 UTF-8 bytes. Query IDs require 1–512 UTF-8 bytes and must be unique within a batch. A batch contains at most 10,000 queries. Input frames and serialized completed responses are each limited to 16 MiB, excluding the final newline. Response accounting includes JSON escaping and complete overlay resolutions; oversized results discard the whole batch. Transport error messages are capped at 512 Unicode scalar values; original overlay issue messages are preserved within the total byte limit. LF, CRLF and a final frame without a newline are accepted; invalid UTF-8 is rejected.

Envelope/batch errors use `invalid-request`, `invalid-request-id`, `unsupported-protocol`, `unsupported-source`, `invalid-batch`, `invalid-policy-package`, `frame-too-large`, `response-too-large` or `encoding-failed`. Graph and query errors use `invalid-graph`, `invalid-closure`, `invalid-overlay`, `unrepresentable-cost` or `work-limit-exceeded`. Hosts should use codes, not parse messages. The graph/search and overlay-resolution bounds still apply. Work limits are per query, not a whole-batch CPU deadline; hosts must apply their own timeout and batch-size policy for untrusted workloads.
