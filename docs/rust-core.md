# Rust navigation core

The `wayform-core` crate currently implements immutable directed topology. Construction rejects duplicate IDs, missing endpoints, self-loops and nonpositive/nonfinite distances. Parallel arcs retain separate IDs. Indexing uses UTF-16 lexical ID order, matching the TypeScript contract, independently of input collection order. Node and edge limits bound accepted graph size.

`Graph::route` now searches an explicit snapshot of `EdgeDecision` records. Every arc must have exactly one suitable, unsuitable or unknown decision and a finite nonnegative preference penalty. Missing or duplicate decisions are errors. Search returns the shortest suitable path; unknown edges are used only in a separate reachability check to distinguish insufficient evidence from infeasibility. It never returns an unknown path to follow.

The heap orders by accumulated cost and then the complete UTF-16 lexical edge-ID sequence. Costs that overflow or cannot advance in floating-point arithmetic return an error. A cumulative path-copy budget bounds this initial implementation's work; exhausting it is a resource error, not evidence that a route is impossible. These limits need benchmarking before large-region use.

Tests compare every endpoint pair across all 729 three-node suitability assignments against an independent exhaustive path enumerator. They also cover parallel arcs, input permutation, closure monotonicity, asymmetric travel, preference penalties, Unicode ties, cycles, incomplete decisions and numeric/resource limits.

`assess_edge` combines [typed edge eligibility](native-eligibility.md), including access, legacy and stairs exclusions, physical requirements, operational policies and entrance registration. `EvidenceSnapshot` supplies current accepted evidence; `assess_measurement` evaluates intervals and categories; `PolicySnapshot` evaluates [operational requirements](native-policies.md). Unknown references, malformed metadata and inconsistent snapshot times are errors. `MobilitySnapshot::from_json` now [validates full documents](native-loading.md), topology, frame references, provenance syntax, timestamps and input identities before constructing these inputs. Its `assess_path` method issues [draft path receipts](native-path-receipts.md). The snapshot route API now evaluates every edge and checks a selected path against its receipt before returning it. Legacy route and instruction parity remain to be implemented. The interfaces below describe the planned boundaries rather than the currently exported API. Keep the compiler and authoring tools in TypeScript. Add mobile bindings only after the Rust CLI passes the same fixture corpus.

The `wayform-search` binary exposes evaluated-graph search through a bounded [JSON-lines protocol](native-search-protocol.md). `npm run test:native` feeds it decisions from the TypeScript oracle for all ten portable contract examples and six additional search scenarios, each with a reordered-input variant. Returned paths are assessed again in TypeScript, including distance and preference cost. The separate `wayform-mobility` [document protocol](native-mobility-protocol.md) loads complete documents, selects suitable routes and issues path receipts; its driver checks 378 requests against the portable contract and loader fixtures. The evaluated-graph CLI retains its caller-supplied decisions and does not generate contract receipts.

## Preserve two explicit contracts

The isolated [legacy search component and CLI](native-legacy-search.md) now cover exact node-path and distance selection across the 7,238 pinned journeys, plus resolved closures and focused search cases. This is one boundary of the planned legacy mode below; loading, overlay validation and complete legacy output are still owned by the TypeScript harness.

1. **Legacy compatibility mode** accepts the pinned VoiceGIS package/profile/overlay contract. It reproduces the existing undirected routing, tie behavior, instruction generation, graphic clearance and receipts. It never claims the new evidence semantics.
2. **Mobility draft mode** accepts `0.1.0-draft.1` bundle/profile/context documents. It uses directed edge IDs, hard requirements, evidence and policy freshness. It must not convert legacy true flags into physical measurements.

The directed adapter is a data migration boundary, not proof that the two modes return identical suitability. New profiles will deliberately return unknown for imported, unsurveyed attributes.

## Suggested public Rust API

```rust
// Sketch only. All public records need documented, versioned serialization.
pub fn load_legacy_package(json: &[u8]) -> Result<LegacyVenue, ValidationErrors>;
pub fn route_legacy(
    venue: &LegacyVenue,
    request: &LegacyRouteRequest,
) -> Result<LegacyRouteResult, ValidationErrors>;

pub fn load_bundle(json: &[u8]) -> Result<MobilityGraph, ValidationErrors>;
pub fn assess_path(
    graph: &MobilityGraph,
    profile: &MobilityProfile,
    context: &EvaluationContext,
    request: &PathRequest,
) -> PathAssessment;

pub fn route(
    graph: &MobilityGraph,
    profile: &MobilityProfile,
    context: &EvaluationContext,
    request: &RouteRequest,
) -> RouteOutcome;

pub struct RouteRequest {
    pub start_id: String,
    pub destination_id: String,
}
pub struct PathRequest {
    pub start_id: String,
    pub destination_id: String,
    pub edge_ids: Vec<String>,
}
pub enum RouteOutcome {
    Routed { edge_ids: Vec<String>, assessment: PathAssessment },
    Infeasible { diagnostic: RouteDiagnostic },
    InsufficientEvidence { diagnostic: RouteDiagnostic },
    Rejected { issues: ValidationErrors },
}
```

Immutable graphs own their parsed data and indexes. Calls receive time/policy snapshots explicitly and do not read the system clock, open a network connection, start a sensor or mutate policy revisions. Authentication, storage, durable revision floors and lifecycle revalidation stay with the host. Bundle integrity checking and schema validation occur before graph construction.

`RouteDiagnostic` must include the input identities, relevant reason codes and a bounded explanation of why qualification failed. It must not expose an uncertain candidate as a route suitable to follow. FFI error records need stable codes rather than parsing human text.

## Routing and deterministic decisions

Search only suitable edges for the returned route. Known unsuitable edges are never reinstated by a preference. If the suitable subgraph has no route, a separate diagnostic reachability check may include unknown edges while still excluding known unsuitable edges. If that makes the destination reachable, return `InsufficientEvidence`; otherwise return `Infeasible` relative to the supplied graph and requirements. Invalid policy/bundle/path structure returns `Rejected`.

Start with Dijkstra (zero heuristic), since cross-frame coordinates and changing cost semantics must not invalidate an A* lower bound. New-contract cost is accumulated length plus explicit edge-kind preference penalties. Keep costs finite and nonnegative. Use stable edge identity throughout predecessor tracking, parallel-edge lookup, connector receipts and instructions.

Define new-contract ties by cost, then lexical complete edge-ID sequence using a documented ordering compatible with the oracle. Retain the legacy heap and input-order behavior in legacy mode. Do not retrofit a new tie-breaker into the frozen reference. Document handling of floating-point comparisons separately from physical measurement uncertainty. A single scalar profile cost does not require Pareto-frontier routing.

Entrance transforms connect coordinate frames; they do not replace local geometry with latitude/longitude. Route instruction generation across frames needs explicit headings and floor/connector transitions. The supplied-path oracle currently checks suitability and receipts only; native instruction semantics remain to be defined.

## CLI boundary before UniFFI

Use line-delimited JSON requests/responses for `legacy-route`, `assess-path` and `route`, one response per input. A TypeScript driver can feed the pinned corpus and portable contract examples to a release build. No platform UI is needed for this gate. Set input/resource bounds and return structured failures for oversized/malformed documents.

After conformance, expose typed records and immutable graph handles through UniFFI. Prefer coarse calls such as load/evaluate/route rather than per-node FFI access. Kotlin/Swift binding generation, packaging and device tests are separate deliverables.

## Acceptance gates

- Original package validation, deterministic source fixtures and all 7,238 legacy queries pass; compare the complete observable output, including path, instructions, policy exclusions and receipts.
- Exact recorded legacy behavior is the starting gate. Equal-cost alternate paths are not silently accepted as a porting regression. Any intended behavior/numeric change must have a separately reviewed versioned expectation and justification.
- New-contract portable examples match, including evidence IDs, reasons, input identities and directed edge sequences. Additional native tests cover missing/contradictory evidence, intervals, stale policies, parallel arcs, reverse slopes and entrance transforms.
- Property-based tests establish that selected paths are connected and eligible, tightening hard constraints never introduces a newly suitable edge, adding a closure never creates a suitable route, and collection reordering never changes decisions. Compare search costs against a simple exhaustive or reference solver on bounded generated graphs.
- Mutation/fuzz tests target schema parsing, references, non-finite values, huge inputs and no-panic behavior. Invalid inputs cannot construct partially valid graphs.
- Benchmarks report fixture sizes, toolchain/build profile, host hardware, load time, route latency distribution and peak memory. Set performance budgets from measurements; do not claim a Rust speedup before measuring.
- Keep the TypeScript contract oracle and immutable legacy subset until native conformance is demonstrated. Afterward, choose one production owner per contract; fixtures remain shared references.

## Open decisions

The current contract identity procedure uses ECMAScript numeric serialization. [Native identity checks](native-identities.md) now cover the stored receipts plus numeric, Unicode and property-order cases. This remains a draft-specific format; replacing it with a standardized canonicalization requires a contract revision. Trusted-clock behavior, signatures, evidence supersession and full physical mobility geometry remain separate work.
