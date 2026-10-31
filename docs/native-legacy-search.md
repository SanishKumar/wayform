# Native legacy search component

`LegacyGraph` preserves the search portion of VoiceGIS `routingCore.ts` at commit `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`. It is separate from the directed mobility engine. The original source and rights statement remain in `compat/voicegis/upstream/`.

This component returns selected node IDs and total distance, or unknown-endpoint/no-route outcomes. It does not yet load a venue package, verify package integrity, resolve operational overlays, generate instructions, validate route graphics or issue the original route receipt. The existing TypeScript compatibility harness remains authoritative for those boundaries.

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

The native CLI driver verifies all 35 upstream source blobs and loads the four verified packages. It recomputes each of the 7,238 full TypeScript results, checks its unchanged recorded fingerprint, and compares the native node path and distance with that result's search projection. It also checks two lift-outage queries using closure IDs resolved by TypeScript and the 26 component cases: 7,266 queries in 30 batches. The closure cases include a standard-profile detour and a step-free no-route result.

Run `npm run check`, `cargo test --workspace --locked`, and `npm run test:native`. The latter builds the binaries and runs the cross-language comparisons. `WAYFORM_LEGACY_SEARCH_BIN` can select an already built executable; otherwise the driver uses `CARGO_TARGET_DIR` (default `target`) and its debug executable. It imposes a 60-second process timeout for this corpus, not a product latency guarantee.

Component expectations are only recorded with `node --import tsx scripts/record-legacy-search-examples.ts --record`. The original 7,238 full-result fingerprints remain unchanged. Native full-result parity remains incomplete: these comparisons do not establish native package validation, overlay resolution, directions, graphics or receipts.

## Batch JSON-lines protocol

`wayform-legacy-search` accepts one projected graph and a nonempty ordered list of queries per line. Batching avoids repeating a venue graph for every journey. All fields below are required; unknown and duplicate object fields are rejected.

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

`sourceCommit` selects the supported reference behavior; it is **not** proof that the supplied graph came from that commit or a verified package. The host remains responsible for package verification, profile mapping and operational overlay validation before projecting the graph. The response identifies the supported implementation even on rejected requests. This protocol carries no mobility evidence or original route receipts.

### Bounds and error codes

Request IDs require 1–256 UTF-8 bytes. Query IDs require 1–512 UTF-8 bytes and must be unique within a batch. A batch contains at most 10,000 queries. Input frames and serialized completed responses are each limited to 16 MiB, excluding the final newline. Response accounting includes JSON escaping; oversized results discard the whole batch. Messages are capped at 512 Unicode scalar values. LF, CRLF and a final frame without a newline are accepted; invalid UTF-8 is rejected.

Envelope/batch errors use `invalid-request`, `invalid-request-id`, `unsupported-protocol`, `unsupported-source`, `invalid-batch`, `frame-too-large`, `response-too-large` or `encoding-failed`. Graph and query errors use `invalid-graph`, `invalid-closure`, `unrepresentable-cost` or `work-limit-exceeded`. Hosts should use codes, not parse messages. The graph/search bounds above still apply. The work limit is per query, not a whole-batch CPU deadline; hosts must apply their own timeout and batch-size policy for untrusted workloads.
