# Native evidence and measurement components

`EvidenceSnapshot` and `assess_measurement` implement the physical-measurement portion of mobility contract `0.1.0-draft.1`. They are Rust library components with typed inputs, not a new JSON contract or a complete edge-eligibility API. The search CLI continues to accept decisions evaluated by its caller.

## Evidence snapshots

Construct `EvidenceSnapshot` from evidence records, an `EvidencePolicy` and an explicit evaluation time. It validates IDs, actor/method metadata, review provenance, chronology, accepted-source sets and the maximum age before exposing results. `issue(id)` returns no issue for accepted evidence, a reason for unaccepted/rejected/future/stale evidence, or an error for an unknown reference. Records cannot be changed through the snapshot.

Times use integer Unix milliseconds in the four-digit-year UTC range. The host must first validate the original `YYYY-MM-DDTHH:mm:ss.sssZ` timestamp and its real calendar date. This component checks the numeric range and chronology; it does not parse date strings. It never reads a clock. Maximum-age equality is current, while equality with `validUntil` is expired. Source, review, future-time and age checks follow the TypeScript reason precedence.

Source acceptance and reviewer metadata are declared inputs. These checks do not authenticate a publisher, reviewer or physical survey.

## Measurement decisions

Call `assess_measurement` with one requirement, observations grouped for one directed edge and attribute, and an evidence snapshot. Requirements cover minimum width, signed uphill/downhill limits, maximum cross-slope, step count, allowed surfaces and door operations. An absent requirement should not generate a call. The caller applies door-operation requirements only to door edges.

Observation attributes must match the requirement. Duplicate observation IDs, missing evidence references, wrong value types, negative error bounds, non-finite inputs and unsafe/nonintegral step counts are errors. Explicit unknown values need a reason and no error bound; numeric values without an error bound remain unknown.

Only current accepted observations contribute measurements. Disjoint numeric intervals or conflicting categories produce unknown. Overlapping intervals use their outer envelope, preserving survey uncertainty. Inclusive thresholds use the same limited floating-point tolerance as TypeScript. Arithmetic interval overflow remains unbounded uncertainty; it never creates a finite measurement. When no usable observation exists, reasons retain the evidence IDs considered. IDs are deduplicated and sorted using UTF-16 order.

The result contains suitability, reason codes and evidence IDs for that one requirement. It does not include a route receipt. It also does not apply restricted-access rules, legacy exclusions, the explicit stairs rule, operational closures or entrance transforms. A complete edge evaluator must combine those checks and preserve their reasons before supplying a decision to search.

## Shared corpus and checks

`packages/mobility-contract/examples/measurements-v0.1.0-draft.1.jsonl` contains 63 fictional cases. `scripts/measurement-cases.ts` builds isolated single-requirement bundle/profile/context inputs, calls the existing TypeScript `assessPath`, and projects its status, reasons and evidence IDs. Timestamp strings are converted to Unix milliseconds only after that evaluator validates the source inputs. Fixtures are comparison data for these components, not a publication format.

`npm run check` verifies that committed expectations still match TypeScript. `cargo test --workspace --locked` evaluates the same cases in Rust in both collection orders. Separate native tests cover malformed inputs and tightening width requirements across 451 numeric intervals. Expectations are recorded explicitly with `node --import tsx scripts/record-measurement-examples.ts --record`; review any changed result before accepting it.

These checks establish component agreement for the supplied synthetic cases. Native full-bundle validation, raw JSON input parity, complete edge decisions and canonical receipts remain separate migration gates.
