# Native legacy venue packages

`legacy_package_content_hash` reproduces the pinned VoiceGIS package content identity: exclude the root manifest, sort object keys, serialize with two-space indentation and ECMAScript number formatting, append a newline, then calculate SHA-256. Nested fields named `manifest` remain part of the content. This format is separate from the mobility draft's compact identity format.

The native serializer accepts ASCII identifier property names (a letter followed by letters or digits). All keys in the four pinned packages meet this boundary. It reproduces their reference runtime's case-insensitive primary ordering and lowercase-before-uppercase ties. Arbitrary Unicode, punctuation and numeric property names are rejected rather than approximating locale-dependent collation. String values can contain Unicode. The maximum depth is 64 and the serialized content is limited to 16 MiB.

Four package hashes and additional frozen number, Unicode-value, case-ordering and nested-manifest examples are checked against the unchanged TypeScript implementation. Normal tests never rewrite expectations. Record the additional cases explicitly with `node --import tsx scripts/record-legacy-package-identities.ts --record`.

The hash function alone does not validate package shape, verify the claimed manifest, authenticate a publisher or establish survey quality.

## Verified loading

`LegacyVenuePackage::from_json(bytes)` now parses the full package, rejects duplicate JSON names, validates its records and references, verifies the manifest hash, and constructs the legacy search graph. Inputs are limited to 16 MiB; nodes and semantic record collections allow at most 100,000 entries, and graph edges at most 500,000. Polygon arrays share the 100,000-element bound. Nested identity serialization retains the depth and output-size limits above.

The package owns its document and graph. `document()` returns an immutable view. `route()` provides snapshot search; `policy_package()` and `session_venue()` derive matching projections from the verified document. These functions do not fetch data or make a route continuously valid. Operational policy resolution and session freshness checks still apply.

The native boundary is deliberately stricter than the original runtime shape inspector: fields used by instructions and maps must have their declared types, coordinates must be finite, units must be metres, and node/source kinds and floors must agree. Portal connections, connector stops/endpoints, site records and source references are checked. Anchor payloads must be unique. A public entry POI must actually have a routing node. Explicit accessibility and restriction flags remain mandatory. Unknown ASCII-identifier extension fields remain in the hashed document but do not acquire navigation meaning.

`LegacyPackageError` distinguishes JSON parsing, identity serialization, structured validation, content-hash mismatch and graph construction failures. Validation returns the first error with a path/code; it does not reproduce the TypeScript inspector's entire ordered issue list. Extra native validation rules and key restrictions are compatibility boundaries, not changes to the frozen upstream verifier.

Tests load every pinned package and compare eight representative route paths/distances with the original results. Negative cases include altered content, duplicate JSON fields, rehashed malformed records, missing flags, dangling references, invalid coordinates, duplicate IDs/payloads and missing public entry points. A recomputed hash never overrides a validation error.

Integrity verifies internal content consistency, not who published the map, whether it was surveyed, geometric traversability or live operating conditions. Full compiler-semantic and route-graphic checks are not replaced by the loader.

## Visitor metadata

`runtime()` exposes immutable visitor nodes, edge labels, public destinations and the default entry point derived from the verified package. Floor/space names, aliases, campus building labels and legacy accessibility flags match the pinned runtime. Four recorded hashes cover the complete node/edge projections, with separate public-destination inventories. Re-record these with `node --import tsx scripts/record-legacy-runtime.ts --record`. The source attribution and pinned snapshot remain unchanged.

Legacy metadata uses the original undirected, last-edge-wins pair lookup. It must not be used as the new directed mobility model. Unknown category icons use the first Unicode character; unlike JavaScript's UTF-16 slice, a non-BMP initial remains a whole character. This deliberate display difference does not affect the four pinned venues. Public visibility and authored accessibility flags do not establish physical suitability.

## Complete-package batch protocol

`wayform-legacy-search` protocol `0.3.0` accepts `venuePackage` instead of the earlier `nodes`, `edges` and policy `package` projections. The envelope still requires `sourceCommit`, `requestId` and `queries`. Queries retain protocol `0.2.0`'s explicit policy and two access flags. Graph/closure inventories are derived only after the complete package passes native verification. Projection overrides and caller-supplied closed-edge lists are rejected.

Responses have the same per-query search and overlay-resolution fields, with `protocolVersion: "0.3.0"`. Invalid package data rejects the entire batch before query results; invalid overlays remain isolated query failures. The existing 16 MiB framing/output bounds, query limits and stateless behavior apply. This protocol does not add live sessions or native directions.

The conformance driver sends all 7,238 original journey requests through this loader in addition to the two retained projection protocols: 21,868 total queries in 40 batches. Original complete-result fingerprints are checked in TypeScript before comparison; native paths/distances are then compared independently for all three protocols. Focused CLI tests also cover native package-derived closures, rejected expired stationary journeys, tampering, duplicate JSON names and attempted projection overrides.
