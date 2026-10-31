# Native legacy overlay resolution

`LegacyPolicyIndex` ports the operational-overlay resolver from VoiceGIS commit `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`. It is separate from WAYFORM's evidence-based mobility policy evaluator. The frozen source and its rights statement remain under `compat/voicegis/upstream/`.

The host supplies a `LegacyPolicyPackage` projection containing the building ID, package hash, and each edge's ID and source ID. Construction rejects an empty building ID, malformed SHA-256 label, duplicate/empty edge IDs and empty source IDs. It owns its index and accepts at most 500,000 edges. A syntactically valid hash is not verified package integrity or publisher authentication; full package verification still belongs to the host.

`resolve(&overlay_json_value, evaluated_at)` returns `LegacyOverlayResolution` with validity, overlay identity, evaluation time, active closure IDs, closed edge IDs and ordered issues. Edge targets close one edge; source targets expand to every edge with that source ID. Overlapping targets deduplicate. Active closure IDs and closed edge IDs use JavaScript-compatible UTF-16 sorting, including supplementary Unicode characters.

Issue codes, messages and ordering follow the pinned resolver: version, building, package binding, time, then closure issues in authored order. A repeated closure ID produces `duplicate-closure` and skips further processing of that duplicate. Malformed input produces `invalid-shape`. Any issue clears both active ID lists and sets `valid: false`.

**An invalid resolution must block routing. Its empty closed-edge list does not mean the venue is open.** The same applies to a returned `LegacyPolicyError`, including exhausted resource limits. This component does not yet wire policy validation into the search-only batch CLI or produce original route receipts; the current conformance driver still resolves its lift-outage inputs in TypeScript.

## Compatibility boundaries

- Validity begins inclusively at `validFrom` and ends exclusively at `validUntil`. Evaluation time is always supplied explicitly; the resolver never reads the system clock.
- Native time parsing accepts exactly `YYYY-MM-DDTHH:mm:ss.sssZ`, with valid Gregorian dates and four-digit years. Other forms, invalid dates and empty times produce `invalid-time`. The original `Date.parse` accepts additional forms and can normalize invalid calendar dates. That wider behavior is deliberately outside native compatibility; hosts must not silently reinterpret ambiguous inputs to bypass rejection.
- Like the original shape check, overlay objects may have extra fields and the overlay ID may be an empty string. Closure IDs, target IDs and reasons must be nonempty strings; target types must be `edge` or `source`. These legacy allowances do not change the new mobility contract.
- Inputs are already parsed JSON values. This API does not define raw JSON duplicate-key handling or a byte-framing protocol. Hosts must bound parsing/input size before calling it.
- At most 500,000 closure records are accepted. A resolution allows one million closure visits plus expanded edge matches; repeated large source targets consume that budget even if the resulting edge IDs deduplicate. Exceeding either bound returns an error without partial results.

The index does not supply a trusted clock, authenticate a closure feed, retain a revision floor, refresh data or enforce a running journey's expiry lease. Callers must re-evaluate before using a route as time or policy changes. Native lease migration is a separate boundary.

## Reproduction

`compat/voicegis/expected/operational-overlays.json` records 51 synthetic resolver cases and seven cases using the pinned Asterion lift-outage projection. The TypeScript test recomputes all 58 expectations with the unchanged upstream resolver. Native tests compare complete resolutions, including messages and ordering, and also test invalid projections, time-format differences and resource exhaustion. Reordering the package edge inventory must not change results. Record-shaped arrays and object-valued target types are rejected rather than accepting broader deserializer representations.

Run `npm run check` and `cargo test --workspace --locked`. Recording requires the explicit command `node --import tsx scripts/record-legacy-overlay-examples.ts --record`; normal tests never update expectations. The original full-route fingerprints and policy cases remain unchanged. Passing these fixtures proves software compatibility within the stated boundary, not real-world closure freshness or navigability.
