# Native mobility document loading

`MobilitySnapshot::from_json(bundle, profile, context)` accepts the three complete UTF-8 JSON documents for `0.1.0-draft.1`. It validates them together and returns an immutable evaluation snapshot. A failure returns `LoadError`; it cannot expose a partially validated graph. The caller supplies evaluation time explicitly.

The loader checks closed object shapes, required nullable properties, safe integer fields, UTC timestamp syntax and calendar validity, IDs and references, legacy provenance syntax, measurement types, entrance frames and controls, profile requirements and operational policy targets. Validation includes unused observations and transforms. Missing data that is valid under the contract remains unknown during assessment; malformed data rejects loading.

`input_digests()` contains [computed input identities](native-identities.md). Closure evaluation binds policies to the digest of the validated bundle rather than trusting an identity supplied separately by the caller. A correctly shaped policy with a wrong bundle digest, untrusted issuer, stale interval or missing revision floor produces unknown eligibility. It does not make the bundle malformed.

`assess_edge(id)` evaluates a directed edge from the loaded documents. `decisions()` returns complete decisions in the graph's stable order, including preference penalties, for the existing graph search API. `assess_path` produces [supplied-path assessments and receipts](native-path-receipts.md). Read-only accessors expose topology and the explicit evaluation metadata. These do not authenticate publishers, survey measurements or policy issuers; authentication and persistent revision floors belong to the host.

## Input bounds and compatibility

Each document is limited to 16 MiB. Graph construction retains the native node and edge limits. Nesting beyond 64 levels is rejected. Duplicate JSON object names, including names that become equal after escape decoding, are rejected rather than allowing the last occurrence to replace the first. Lone UTF-16 surrogates are rejected because native strings require Unicode scalar values. These are deliberate stricter input gates than JavaScript's `JSON.parse`.

Nullable fields must be present as explicit `null` when unknown. Numbers use binary64 semantics; schema integers accept equivalent spellings such as `3`, `3.0` and `3e0` while enforcing the nonnegative safe-integer range. Timestamps require the exact millisecond UTC spelling and a real Gregorian date, including years `0000` through `9999`. Leap seconds and offset spellings are not accepted by this draft.

`route(start_id, destination_id)` evaluates all edges from the snapshot, selects a suitable path and checks its complete receipt against the search result. Failure returns a bounded frontier diagnostic without a candidate path. Numeric/resource errors remain separate errors. This API and the [mobility CLI](native-mobility-protocol.md) use the same snapshot for eligibility and receipts.

The loader currently builds a complete snapshot for each set of documents. Reusable loaded-bundle handles, streaming ingestion and large-venue performance measurements remain future work.

## Reproduction

`loading-v0.1.0-draft.1.json` contains one synthetic base and 256 explicit mutations with TypeScript validation expectations. Cases remove every modeled required property and add unknown properties, then exercise duplicate IDs, references, timestamp boundaries, unsafe integers, measurement errors and policy binding. Native tests also load all ten original portable examples and sixteen identity examples without fixture adapters, and test raw JSON ambiguity and resource limits.

Run `npm run check` and `cargo test --workspace --locked`. Expectations are only replaced by `node --import tsx scripts/record-loader-examples.ts --record`; ordinary tests verify them without writing files. This is contract conformance, not field validation or a completed legacy route port.
