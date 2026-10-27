# Native edge eligibility

`assess_edge` combines the draft mobility rules for one directed edge: restricted access, legacy exclusions, the explicit step-free stairs rule, operational requirements, entrance registration, and physical measurements. It returns suitability, ordered reasons and considered evidence IDs. An exclusion takes precedence over uncertainty without discarding uncertainty reasons. It does not generate a route receipt.

## Inputs and host boundary

The function accepts typed `EdgeFacts`, `EdgeRequirements`, `EvidenceSnapshot` and `PolicySnapshot` values. Both snapshots must use the same evaluation time; a mismatch is an error. Every assessed edge must exist in the policy snapshot's inventory, even if it has no operational requirements.

Requirements must be valid and contain at most one entry per physical attribute. All supplied observations are validated, including unconstrained attributes and observations on excluded edges. Wrong types, duplicate IDs and missing evidence references produce errors. Door-operation requirements apply only to door edges. Legacy true flags never fill missing measurements; legacy false flags are exclusions only when the profile enables that rule.

The host still validates full bundle JSON, topology, unique IDs across the entire bundle, source provenance, exact bundle hashes and coordinate-frame references. It extracts observations for the correct directed edge and supplies matching facts to search. The typed evaluator is not a replacement for those load-time checks. Snapshot time equality prevents combining different evaluation times; it does not authenticate or prove a shared source bundle.

## Entrance registration

Cross-frame transitions require an entrance edge with registration geometry. Same-frame edges cannot carry registration. Geometry contains local plan handedness, rotation, translation, uncertainty, at least two nondegenerate horizontal control pairs, and the local/ENU positions of the portal endpoints. Raw geometry must be finite, and its evidence reference must exist.

Registration uses the draft rigid transform and checks both control residuals and endpoint correspondence in three dimensions. Stale registration evidence and error above the profile budget produce unknown. Non-finite computed error, including arithmetic overflow, also produces unknown. It never turns an unusable transformation into suitability. This does not validate the physical survey, geographic anchor or WGS84-to-ENU projection.

## Routing integration and conformance

Use the returned status as the suitability of the matching `EdgeDecision`, with preference penalties taken from the same profile. Search continues to return paths made only of suitable edges. A native integration test checks selection of a longer usable route around a closure and insufficient evidence when that closure expires and the detour is excluded.

The 26 synthetic records in `packages/mobility-contract/examples/edges-v0.1.0-draft.1.jsonl` are produced by the existing TypeScript evaluator. Tests compare status, complete ordered reasons and evidence IDs in Rust under reordered observations, evidence, requirements, policies and control pairs. They include overlapping exclusions and unknowns, stairs, doors, reverse entrances, Y-down plans, stale registration and numeric overflow. Additional native tests reject malformed inputs and inconsistent snapshot times.

Run `npm run check` and `cargo test --workspace --locked`. Record changed expectations explicitly with `node --import tsx scripts/record-edge-examples.ts --record` and review the differences. The JSON-lines search CLI still accepts caller-evaluated decisions. Full native bundle loading, raw input parity, receipts and legacy route/instruction parity remain separate deliverables.
