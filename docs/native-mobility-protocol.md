# Mobility document protocol

`wayform-mobility` accepts one UTF-8 JSON request per line and flushes one JSON response per line. Build with `cargo build --package wayform-core --bin wayform-mobility --locked`. Its protocol version is `0.1.0`; its mobility documents use `0.1.0-draft.1`.

This command loads complete bundle, profile and context documents, computes their identities and performs native supplied-path assessment. The existing `wayform-search` command remains available for [caller-evaluated graph search](native-search-protocol.md).

## Request

All fields below are required. Objects are closed: additional fields, including caller-supplied `snapshot` or `decisions`, are rejected.

| Field | Value |
| --- | --- |
| `protocolVersion` | `"0.1.0"` |
| `requestId` | String with 1–256 UTF-8 bytes |
| `operation` | `"assess-path"` |
| `bundle` | Complete mobility bundle object |
| `profile` | Complete mobility profile object |
| `context` | Complete evaluation context object, including explicit UTC time |
| `startId`, `destinationId` | Endpoint strings |
| `edgeIds` | Ordered array of directed edge-ID strings; may be empty |

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

## Response and failures

Every response contains `protocolVersion`, `requestId`, `snapshot` and `outcome`. `snapshot` contains the computed `bundleDigest`, `profileDigest` and `contextDigest` only after all documents load successfully. It is null when loading fails. Each request builds a fresh immutable snapshot; there is no implicit previous bundle, clock reading or policy cache.

`outcome: "assessed"` includes the complete [path assessment and receipt](native-path-receipts.md) under `assessment`. Its status may be suitable, unsuitable, unknown or invalid. An assessed unknown/unsuitable path is a diagnostic result, not a recommended route. Invalid path structure retains the draft's receipt semantics. For example, an empty edge-ID string produces an invalid-contract assessment with null receipt digests, while the outer snapshot still identifies the successfully loaded documents.

`outcome: "rejected"` includes `error.code`, `error.document` and `error.message`. Document is `bundle`, `profile`, `context` or null. Messages are diagnostic text capped at 512 Unicode scalar values; clients should use codes.

| Code | Meaning |
| --- | --- |
| `invalid-request` | Malformed envelope, unknown operation, wrong field type, duplicate/unknown envelope field or invalid encoding |
| `invalid-request-id` | Request ID outside its byte-length bounds |
| `unsupported-protocol` | Unsupported protocol version |
| `invalid-document` | Embedded JSON rejected by the strict loader, including duplicate names |
| `invalid-contract` | Invalid document shape, values or references |
| `document-too-large`, `identity-error` | Document or identity processing could not complete within its rules and limits |
| `path-limit-exceeded` | Supplied-path work/output limits exceeded |
| `evaluation-failed` | Native evaluation failed; no assessment is issued |
| `frame-too-large` | Request line exceeds its transport limit |

Envelope parse failures, invalid request IDs and oversized frames return null request ID and snapshot. Once the envelope and request ID are valid, rejections retain the request ID. Only errors after successful loading retain a snapshot. Rejection does not terminate the stream.

## Limits and reproduction

The entire line, including all documents, whitespace and optional CR, allows at most 16 MiB before LF. Oversized frames are drained through LF or EOF without retaining the excess bytes. Final lines without LF are processed; blank lines are rejected; empty EOF emits nothing. I/O failure exits unsuccessfully. The loader and path evaluator retain their own graph, depth, numeric and work limits. Hosts remain responsible for process lifetime, authenticated policy ingestion and durable revision floors.

`npm run test:native` checks both binaries. The document driver sends 320 requests: ten original portable examples and 22 additional path examples in two collection orders, followed by all 256 loader mutations. It compares full receipts and computed identities with TypeScript and checks that malformed documents cannot produce an assessment or claimed snapshot. Rust process tests cover request isolation, duplicate fields inside documents, UTF-8 byte limits, exact frame boundaries, oversized-frame recovery and path resource errors. `WAYFORM_MOBILITY_BIN` selects an explicit executable; otherwise the driver uses `CARGO_TARGET_DIR` or `target`, under `debug`.

This transport does not yet select a route from loaded documents. Native search integration, legacy instruction parity and real-world validation remain separate work.
