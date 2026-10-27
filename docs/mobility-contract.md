# Mobility data contract — 0.1.0-draft.1

Status: proposed, executable design. Types, exported JSON schemas and semantic validation are present. `assessPath` is a reference evaluator for a supplied path, not a shortest-path implementation or permission to navigate independently.

## Inputs and validation boundary

The three exported schemas describe `MobilityBundle`, `MobilityProfile` and `EvaluationContext`. All properties are explicit; extra fields, coerced booleans and non-finite numbers are rejected. UTC timestamps use exactly `YYYY-MM-DDTHH:mm:ss.sssZ` and must round-trip to a real date. Consumers must run both JSON shape validation and the semantic checks in `validate.ts`: schema validation alone does not check references, observation units, review chronology or geographic consistency.

Schema version changes are explicit. Unsupported versions are invalid. No defaults create accessibility facts. Integer counters/revisions must fit exactly in a JavaScript safe integer (0 through 9,007,199,254,740,991). `revision` is a human/system version label; SHA-256 input identities in receipts additionally detect changed inputs under a reused label.

## Directed topology

Every edge has its own ID, `from`, `to`, positive length in metres, kind, access rule, optional entrance transform, and optional legacy assertion. There is no implicit reverse traversal. Opposite directions and parallel arcs retain distinct IDs and observations. Instructions and path reconstruction in Rust must use edge IDs, never unordered endpoint keys.

Kinds currently cover walkway, door, ramp, stairs, lift, crossing and entrance. Every physical door whose operation matters must be represented by a door edge. This draft cannot prove the completeness of a map or infer omitted barriers. Restricted access is enforced independently of physical suitability. The graph must contain the operator's actual traversable topology.

## Facts, evidence and uncertainty

Observations attach to a specific directed edge and attribute. Units are part of attribute names/definitions:

| Attribute | Value and meaning |
| --- | --- |
| `clearWidthMeters` | Nonnegative minimum clear passage width over the entire arc, metres |
| `slopePercent` | Signed worst relevant longitudinal grade for this directional arc; uphill positive |
| `crossSlopePercent` | Nonnegative maximum cross-slope magnitude, percent |
| `stepCount` | Nonnegative integer count |
| `surface` | Declared categorical surface |
| `doorOperation` | Declared categorical operation, evaluated on door edges |

An arc containing materially different grades or surfaces must be segmented. A single signed slope cannot represent a hill that rises and falls. Vertical lift travel is not a physical pedestrian ramp; data authors must explicitly survey/describe boarding and exiting arcs and supply facts appropriate to the profile. Domain-specific lift-capacity and turning-envelope rules are not modeled in this draft.

Numeric uncertainty is an absolute error bound in the attribute's units; it is not a confidence percentage. A missing uncertainty bound makes a numeric observation insufficient. Categorical observations have null uncertainty. A null value requires an explicit unknown reason and null uncertainty. Absence of an observation also remains unknown. A complete numeric observation has no unknown reason.

Evidence records carry source type, actor, method, observation time, optional expiry and review provenance. Each attribute can cite different evidence. Reviewed evidence must name the reviewer and review time. Eligibility uses the caller's accepted source types, review requirement and maximum age. Future observations/reviews, rejected evidence, unaccepted sources and expired information cannot establish suitability. Evidence is current at its observation time, at the maximum-age boundary, and strictly before `validUntil`.

Among usable observations, conflicting categorical values or disjoint numeric intervals produce `unknown`. A newer observation does not automatically win. Overlapping intervals use their outer envelope, without averaging or shrinking uncertainty. Expired or otherwise unaccepted observations are not treated as current contradictory measurements. Explicit evidence supersession/dispute resolution is future work.

For a hard numeric limit, an interval wholly within the limits qualifies; an interval wholly outside fails; an interval straddling a limit remains unknown. Limits are inclusive. Arithmetic comparisons allow only `8 × Number.EPSILON × max(1, abs(a), abs(b))` to absorb binary floating-point noise. This is separate from survey uncertainty and must match in a native implementation. The tests include a width just below the boundary that still remains unknown.

Legacy flags are retained under `legacy` with meaning `author-assertion`, original source commit, package hash and edge ID. A true flag never supplies a measurement. A false flag is an exclusion only when the profile explicitly respects legacy exclusions. That policy can reject an arc without asserting that a survey measured the barrier.

## Profile and result semantics

Each hard requirement is a limit or an explicit null meaning that the user did not constrain that attribute. Null is not a measured zero. Allowed surface/door categories are nonempty sets when specified. An explicit step-free requirement also excludes an edge declared to be stairs. Door-operation requirements apply to door edges; map completeness remains an authoring obligation.

Preferences currently add a nonnegative metre-equivalent penalty for selected edge kinds. They affect ranking only after eligibility. They cannot waive a hard constraint, reopen a closure or promote missing evidence.

| Result | Meaning |
| --- | --- |
| `suitable` | This supplied path satisfies the declared requirements using currently accepted inputs |
| `unsuitable` | At least one declared requirement, access rule or current closure excludes the path |
| `unknown` | No known exclusion establishes failure, but required evidence or current policy is insufficient |
| `invalid` | Inputs or the supplied directed path violate the contract |

Result precedence is invalid, unsuitable, unknown, suitable. Reasons for both exclusions and unknowns are retained. A zero-edge path is permitted only when valid start and destination IDs are equal. There is no silent relaxation or fallback to stairs. An unknown candidate is never presented as verified suitable.

`suitable` is relative to an explicit profile and accepted data, not a general accessibility certification. The evaluator does not search all paths, so rejection of one candidate does not establish that the venue has no usable route.

## Operational policy and offline decisions

A bundle declares which directed edges require an operational policy. Each policy is bound to a bundle ID, revision, exact bundle digest and requirement ID, with issuer, monotonic revision, validity interval and closed edge IDs. A required missing, stale, future, untrusted, wrong-bundle or rolled-back policy makes the affected path unknown. Reusing a revision label with changed bundle content requires a replacement policy. Duplicate policies and foreign closure targets are invalid.

Evaluation uses an explicit timestamp. The interval is `[validFrom, validUntil)`. A trusted minimum revision is required for each policy used; a lower revision cannot restore an earlier all-clear. The supplied context is an input from authenticated ingestion, not a self-authenticating document. Issuer allowlists and revision numbers do not implement signatures, persistent rollback protection or a trusted clock.

Maps and static measurements can remain usable offline. An expired required policy remains unknown until a valid replacement is available. The evaluator has no network fallback. Clients must reevaluate on policy/evidence expiry and lifecycle transitions, then reacquire location before resuming where necessary. That active-trip lifecycle is later product work; only the original legacy lease is tested here.

## Entrance registration and coordinates

Indoor coordinates remain local metres with an explicit plan handedness. Outdoor coordinates in this draft use an east/north/up metric frame anchored at a WGS84 longitude, latitude and ellipsoidal height. The anchor is not substituted into indoor coordinates. Public geographic ingestion and WGS84-to-ENU projection are not implemented.

An entrance transform connects one local frame to one ENU frame. It flips plan Y when needed, rotates counterclockwise, then translates in metres. It records an uncertainty bound, survey evidence and at least two noncoincident horizontal control pairs. Reflections, scale/shear fitting and calibration estimation are outside this rigid transform.

A cross-frame edge must be an entrance with the matching transform. Its two endpoints represent the same surveyed portal within the permitted error; approach paths must be separate edges. Qualification checks control residuals and endpoint registration in 3D, adding the declared transform uncertainty before comparison with the profile's registration budget. Bad alignment or stale evidence produces unknown; missing references or degenerate controls are invalid.

Two supplied controls only test internal consistency. They do not prove an independent survey occurred. Field collection must establish anchor placement, control quality, error bounds and the correctness of the geographic anchor.

An unrepresentable registration error, including arithmetic overflow from otherwise finite inputs, also produces unknown with `transform-error-exceeds-budget`.

## Receipts and native conformance

Receipts retain the contract version, bundle identity, profile, evaluation time, exact directed edge sequence, input digests, supplied policy revisions and path costs. Reasons carry edge/attribute/evidence IDs. Receipts include considered evidence even when it causes an unknown result. Stored input snapshots are still needed for later reconstruction; a digest alone cannot recover them.

For input identity, unordered top-level bundle collections are sorted by ID; set-valued category/edge lists and policy collections are sorted; control pairs are sorted by their canonical JSON. Coordinates and ordered path arrays retain order. Object keys use UTF-16 lexical ordering and numbers use ECMAScript JSON serialization; the digest is SHA-256 of UTF-8 compact JSON. This is a draft identity procedure, not a claim of a complete cryptographic publication protocol. Native serialization must match the portable examples before these receipts become a stable cross-language format.

Array-index property names (`0` through `4294967294`) are enumerated numerically by `JSON.stringify` even after lexical insertion; other keys remain in UTF-16 order. This matters for numeric requirement IDs in revision-floor objects. Native consumers must reproduce this behavior rather than substitute a different canonical JSON standard.

The ten `examples/decisions-v0.1.0-draft.1.json` scenarios contain fictional facts and expected full decisions. New-contract tests additionally exercise directed reverse slopes, parallel arcs, boundary errors, policy revision floors, contradictory evidence and malformed inputs. [Native identity checks](native-identities.md) cover additional numeric and Unicode serialization cases.

## Remaining design work

Turning space, door force, surface taxonomy, user-assistance requirements, complex crossings, lift capacity, routing over time-dependent conditions, map coverage certification, evidence supersession, trust roots and survey procedures require further design. No numeric threshold in the synthetic fixtures is a recommended clinical or accessibility standard. Users and venue operators must participate in setting real requirements and validating journeys.
