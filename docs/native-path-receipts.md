# Native supplied-path receipts

`MobilitySnapshot::assess_path(start_id, destination_id, edge_ids)` evaluates an ordered directed path using the validated documents and explicit time captured by the snapshot. It returns the draft assessment and receipt as serializable records. The caller cannot substitute a bundle digest or edge decision during this operation.

The result includes suitability, sorted reasons, deduplicated evidence IDs, all three input digests, the traversed edge sequence, relevant policy revisions, distance and preference cost. Unsuitable dominates unknown while retaining both sets of reasons. Repeated traversals retain repeated reasons and costs; evidence IDs remain deduplicated. Policy revisions record policies affecting traversed edges even when those policies are untrusted or stale; recording a revision is not acceptance of its authority.

Unknown endpoints, absent or reversed arcs, unfinished paths and overflowing costs produce `invalid` assessments with the same reason codes and nullable receipt costs as the TypeScript oracle. Empty edge IDs produce `invalid-contract` with empty receipt edge IDs and null digests. Malformed bundle/profile/context JSON is instead rejected by `MobilitySnapshot::from_json` before a snapshot exists; the loader does not manufacture an invalid-contract receipt from arbitrary documents.

Costs preserve the draft's binary64 accumulation behavior. A finite sum that loses a very small increment still matches the supplied-path oracle. The existing graph search deliberately rejects nonadvancing costs: it needs strictly increasing labels for its search invariants. This distinction is covered by a conformance example.

## Bounds and checks

Paths are limited to 100,000 edge references. Expansion is bounded by a 16 MiB budget for path and assessment text and one million counted output items; these are work limits, not an exact JSON response-size guarantee. Exceeding a limit returns `PathError::ResourceLimit`, never a claim that a route is unsuitable or impossible. Per-edge assessments are cached within a call so cycles do not repeat evidence evaluation.

Native tests compare complete receipts for all ten original portable examples and 22 additional cases in two collection orders. The added cases cover stationary and malformed paths, preference costs, multiple edges, conflicting suitability, unused policies, untrusted policy revisions, Unicode sorting, cycles and numeric overflow. Accepted loader corpus cases also compare complete receipts.

`npm run check` checks the portable expectations against TypeScript. `cargo test --workspace --locked` checks native outputs. Record additional examples only with `node --import tsx scripts/record-path-examples.ts --record`. Comparison normalizes JSON number spellings such as `10` versus `10.0`; it does not tolerate different numeric values, hashes or decisions.

This API assesses a supplied path, including unsuitable or unknown paths for diagnosis. It does not recommend such paths for travel. The `wayform-mobility` [document protocol](native-mobility-protocol.md) now exposes loaded snapshots and supplied-path receipts. The existing search CLI still accepts caller-evaluated edges. Legacy VoiceGIS route/instruction parity and physical field validation remain outstanding.
