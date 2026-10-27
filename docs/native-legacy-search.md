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

Run `npm run check` and `cargo test --workspace --locked`. Expectations are only recorded with `node --import tsx scripts/record-legacy-search-examples.ts --record`. This file is a new component baseline; the original 7,238 full-result fingerprints remain unchanged. Full-corpus native search comparison and remaining legacy output parity are subsequent gates.
