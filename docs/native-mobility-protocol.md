# Mobility document protocol

`wayform-mobility` accepts one UTF-8 JSON request per line and flushes one JSON response per line. Build with `cargo build --package wayform-core --bin wayform-mobility --locked`. Its protocol version is `0.1.0`; its mobility documents use `0.1.0-draft.1`.

This command loads complete bundle, profile and context documents, computes their identities and performs native supplied-path assessment or route selection. The existing `wayform-search` command remains available for [caller-evaluated graph search](native-search-protocol.md).

## Request

All fields below except `edgeIds` are required. Objects are closed: additional fields, including caller-supplied `snapshot` or `decisions`, are rejected.

| Field | Value |
| --- | --- |
| `protocolVersion` | `"0.1.0"` |
| `requestId` | String with 1–256 UTF-8 bytes |
| `operation` | `"assess-path"` or `"route"` |
| `bundle` | Complete mobility bundle object |
| `profile` | Complete mobility profile object |
| `context` | Complete evaluation context object, including explicit UTC time |
| `startId`, `destinationId` | Endpoint strings |
| `edgeIds` | Required for `assess-path`: ordered array of directed edge-ID strings, possibly empty. Must be omitted for `route`; null is invalid |

Documents are embedded JSON objects, not JSON-encoded strings or file paths. They remain raw JSON until the loader checks duplicate property names and validates their content. In particular, a duplicate policy revision floor cannot silently replace an earlier floor during envelope parsing.

The portable examples contain complete runnable documents and expected results. To construct a request from an example in JavaScript:

```js
const request = {
  protocolVersion: '0.1.0', requestId: example.id, operation: 'assess-path',
  bundle: example.bundle, profile: example.profile, context: example.context,
  ...example.request,
};
process.stdout.write(JSON.stringify(request) + '\n');
```

For route selection, use `operation: 'route'`, the same documents and endpoint IDs, and omit `edgeIds`. The process computes every edge decision from those documents before search.

## Response and failures

Every response contains `protocolVersion`, `requestId`, `snapshot` and `outcome`. `snapshot` contains the computed `bundleDigest`, `profileDigest` and `contextDigest` only after all documents load successfully. It is null when loading fails. Each request builds a fresh immutable snapshot; there is no implicit previous bundle, clock reading or policy cache.

`outcome: "assessed"` includes the complete [path assessment and receipt](native-path-receipts.md) under `assessment`. Its status may be suitable, unsuitable, unknown or invalid. An assessed unknown/unsuitable path is a diagnostic result, not a recommended route. Invalid path structure retains the draft's receipt semantics. For example, an empty edge-ID string produces an invalid-contract assessment with null receipt digests, while the outer snapshot still identifies the successfully loaded documents.

`outcome: "routed"` also includes `assessment`, with suitable status and the selected path in `assessment.receipt.edgeIds`. Search uses only suitable edges, minimizes distance plus preference penalties and resolves equal-cost ties by the complete UTF-16 edge-ID sequence. The native wrapper reassesses that path against the same immutable snapshot and checks its status, distance and cost before issuing a routed response. A stationary route between known identical endpoints has an empty path and zero cost.

`outcome: "insufficient-evidence"` means no suitable path exists, but including unknown edges would connect the endpoints. `outcome: "infeasible"` means even that expanded graph cannot connect them. Both return `diagnostic` with `suitableReachableNodes`, `unknownFrontierEdges`, `unsuitableFrontierEdges` and sorted unique `reasonCodes`. The frontier consists of edges leaving the region reachable through suitable edges. It may include branches unrelated to the destination, so this is a bounded summary of qualification failures, not a minimal explanation or a substitute route. Neither outcome contains an assessment, receipt or candidate path. Their outer `snapshot` still records the evaluated inputs.

`outcome: "rejected"` includes `error.code`, `error.document` and `error.message`. Document is `bundle`, `profile`, `context` or null. Messages are diagnostic text capped at 512 Unicode scalar values; clients should use codes.

| Code | Meaning |
| --- | --- |
| `invalid-request` | Malformed envelope, unknown operation, wrong field type, duplicate/unknown envelope field or invalid encoding |
| `invalid-request-id` | Request ID outside its byte-length bounds |
| `unsupported-protocol` | Unsupported protocol version |
| `invalid-operation-input` | Missing `edgeIds` for `assess-path`, or supplied `edgeIds` for `route` |
| `invalid-document` | Embedded JSON rejected by the strict loader, including duplicate names |
| `invalid-contract` | Invalid document shape, values or references |
| `document-too-large`, `identity-error` | Document or identity processing could not complete within its rules and limits |
| `path-limit-exceeded` | Supplied-path work/output limits exceeded |
| `evaluation-failed` | Native evaluation failed; no assessment is issued |
| `invalid-search` | Invalid search query, including an unknown endpoint |
| `unrepresentable-cost` | Search cost overflows or cannot advance in binary64 arithmetic |
| `work-limit-exceeded` | Search exceeded its cumulative path-copy budget |
| `frame-too-large` | Request line exceeds its transport limit |

Envelope parse failures, invalid request IDs and oversized frames return null request ID and snapshot. Once the envelope and request ID are valid, rejections retain the request ID. Only errors after successful loading retain a snapshot. Rejection does not terminate the stream.

## Limits and reproduction

The entire line, including all documents, whitespace and optional CR, allows at most 16 MiB before LF. Oversized frames are drained through LF or EOF without retaining the excess bytes. Final lines without LF are processed; blank lines are rejected; empty EOF emits nothing. I/O failure exits unsuccessfully. The loader and path evaluator retain their own graph, depth, numeric and work limits. Hosts remain responsible for process lifetime, authenticated policy ingestion and durable revision floors.

`npm run test:native` checks both binaries. The document driver sends 378 requests: ten original portable examples and 22 additional path examples in two collection orders, all 256 loader mutations, and 29 route scenarios in two collection orders. It compares full receipts and computed identities with TypeScript and checks that malformed documents cannot produce an assessment or claimed snapshot. Route scenarios cover suitable detours, explicit stairs exclusions, closure expiry, edited bundle identities, one-way travel, parallel-edge ties, stationary routes, diagnostics and numeric failures. Rust process tests cover request isolation, duplicate fields inside documents, UTF-8 byte limits, exact frame boundaries, oversized-frame recovery, operation-specific arguments and path resource errors. `WAYFORM_MOBILITY_BIN` selects an explicit executable; otherwise the driver uses `CARGO_TARGET_DIR` or `target`, under `debug`.

The route corpus is explicitly recorded with `node --import tsx scripts/record-route-examples.ts --record`. Native and TypeScript tests compare the committed expectations without rewriting them. This transport covers the new mobility draft; legacy route/instruction parity and real-world validation remain separate work.
