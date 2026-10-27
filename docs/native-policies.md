# Native operational policy evaluation

`PolicySnapshot` evaluates the operational-policy portion of mobility contract `0.1.0-draft.1`. Its typed inputs are a bundle identity, the complete directed edge inventory, policy requirements, and an explicit policy/time context. It does not parse the full bundle or compute its digest.

Construction rejects malformed identities, duplicate IDs, foreign closure targets, invalid validity intervals, duplicate policies, and missing or invalid requirement references. Policy revisions and trusted minimum revisions must fit a JavaScript safe integer. A missing floor differs from an explicit zero floor. Times are Unix milliseconds after host validation of the original UTC timestamp syntax.

For each requirement the evaluator checks, in order: presence, bundle ID/revision/digest, trusted issuer, minimum revision, and the half-open validity interval `[validFrom, validUntil)`. Failures produce unknown with the same reason codes as TypeScript. Only a current accepted closure produces unsuitable. At expiry a closed edge becomes unknown; expiry never silently reopens it.

Edges without policy requirements are operationally suitable. Reverse arcs remain independent. Multiple requirements preserve all reasons; a known closure takes precedence over an unknown requirement. Unknown edge queries are errors. Results own their data and do not change when the caller updates its input records.

The host must provide the identity of the validated bundle, authenticate policy ingestion, persist revision floors and supply trustworthy evaluation time. The component validates the declared digest's form and equality; it does not prove that it hashes the supplied map, authenticate an issuer or provide durable rollback protection. Operational suitability alone does not establish physical suitability.

## Conformance

The 23 synthetic records in `packages/mobility-contract/examples/policies-v0.1.0-draft.1.jsonl` are projected from the TypeScript supplied-path evaluator with physical requirements disabled. They cover expiry boundaries, identity changes, issuer trust, revision floors, directionality, multiple requirements and reason precedence. Native tests compare status and ordered reason codes with both input collection orders, and separately reject malformed inputs and check reopening behavior.

`npm run check` verifies the committed expectations against TypeScript. `cargo test --workspace --locked` runs native checks. Regenerate expectations explicitly with `node --import tsx scripts/record-policy-examples.ts --record` and review changed outcomes.
