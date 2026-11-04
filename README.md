# WAYFORM

Foundations for mobility-aware outdoor-to-indoor navigation: a native routing core, an Android offline venue catalog, a pinned VoiceGIS compatibility harness and a draft mobility-data contract. It is not a deployed navigation product.

## Run the checks

Use Node.js 22.14+ (locally verified with Node 24) and npm:

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run check
npm run compat:audit
```

After dependency installation, these checks need no network or adjacent VoiceGIS checkout. `check` runs TypeScript validation, exported-schema consistency, legacy compatibility, and contract tests. `compat:audit` runs the unmodified upstream direction audit over all four venues.

## Components

- **Compatibility baseline:** 35 exact Git blobs from VoiceGIS commit `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`, verified venue packages, deterministic recompilation and 7,238 recorded route results.
- **Mobility contract:** versioned JSON schemas, semantic validation, supplied-path assessment, a conservative legacy adapter and portable decision fixtures. The contract is `0.1.0-draft.1`.
- **Quality checks:** TypeScript validation, regression tests and GitHub Actions on Linux/Windows with Node 22/24.
- **Native core:** validated mobility documents, evidence and policy evaluation, deterministic directed route search and path receipts in `crates/wayform-core`. Snapshot-based routing distinguishes known exclusions from insufficient evidence and only selects suitable paths. Shared TypeScript fixtures check component, receipt and CLI behavior.
- **Android app:** offline venue search, floor maps, native previews and manual guidance under expiring conditions, backed by verified packages and SQLite. Bundled venues and condition simulations are fictional. See the [Android build and device checks](docs/android.md) and [mobile bindings](docs/mobile-bindings.md).

| Location | Responsibility |
| --- | --- |
| `compat/voicegis/upstream/` | Immutable upstream source subset and synthetic venue fixtures |
| `compat/voicegis/source-lock.json` | Upstream revision, SHA-256 digests and Git blob identities |
| `compat/voicegis/expected/` | Recorded legacy behavior; normal tests never rewrite it |
| `packages/mobility-contract/` | Proposed data model, schemas, evaluator and examples |
| `docs/compatibility.md` | Exact compatibility scope and reproduction instructions |
| `docs/mobility-contract.md` | Normative draft semantics, trust boundaries and limitations |
| `docs/rust-core.md` | Native interface design and acceptance gates |

The synthetic data demonstrates software behavior. It does not establish physical passability, positioning accuracy, publisher authenticity or a live closure service. A legacy accessibility assertion is never converted into surveyed evidence.

The Rust toolchain is pinned in `rust-toolchain.toml`. Run `cargo test --workspace --locked` for native tests and `cargo clippy --workspace --all-targets --locked -- -D warnings` for lint checks. `npm run test:native` builds the JSON-lines search CLI and checks it against TypeScript-evaluated mobility fixtures on Linux and Windows in CI. The [CLI protocol](docs/native-search-protocol.md) accepts already evaluated edges; its snapshot labels are supplied by the caller.

Native [edge eligibility](docs/native-eligibility.md) combines physical requirements with access rules, closures and entrance registration. The [document loader](docs/native-loading.md) validates complete bundle/profile/context JSON and computes their identities before evaluating edges or issuing [supplied-path receipts](docs/native-path-receipts.md). The `wayform-mobility` [document protocol](docs/native-mobility-protocol.md) exposes supplied-path assessment and suitable-route selection over JSON lines. Go services, geographic ingestion and production synchronization are later work.

An isolated [legacy search component and batch CLI](docs/native-legacy-search.md) preserve VoiceGIS's undirected adjacency and queue tie behavior. `test:native` retains 21,868 search/projection comparisons and separately compares all 7,238 [complete native route results](docs/native-legacy-routes.md) through protocol `0.4.0`: paths, destination metadata, landmark directions, graphic assessments and receipts. Complete packages are verified in Rust. Bearings use a documented nine-decimal comparison boundary; other result fields are exact. The original TypeScript fingerprints remain unchanged.

A [native venue loader](docs/native-legacy-packages.md) now validates complete legacy package records, references and content hashes before constructing the search graph. Its immutable package can supply matching policy/session projections. The original TypeScript verifier remains the comparison reference.

The [native legacy overlay resolver](docs/native-legacy-policies.md) checks closure targets, package binding and expiry against a host-verified package projection. Protocol `0.2.0` validates these policies before routing, blocks invalid overlays and requires explicit policy absence. Its comparisons preserve legacy issue ordering and UTF-16 sorting. A [native expiry lease](docs/native-legacy-leases.md) now latches wall/monotonic expiry and clock failures, with 105 reference traces. Hosts must retain and check it during guidance; the batch CLI remains stateless.

The [native route session](docs/native-legacy-sessions.md) now owns a planned path and its lease together. Every path read checks freshness; policy removal or other host-reported changes can permanently withdraw the route. Mobile bindings expose complete guidance under this lease. Android clears guidance on expiry, recovery and suspension; speech is being integrated.

## Updating baselines

`npm run compat:record` explicitly regenerates expected legacy results. Review every change against the pinned source and documented compatibility decisions; never use regeneration to silence a regression. `npm run schema:write` exports schemas from their TypeScript definitions. Contract examples are explicitly recorded with `node --import tsx scripts/record-contract-examples.ts --record`.

The [native measurement corpus](docs/native-measurements.md) is recorded with `node --import tsx scripts/record-measurement-examples.ts --record`. Both TypeScript and Rust tests read the committed expectations; normal tests do not rewrite them.

The imported VoiceGIS README retains its original rights statement. No new license is imposed on the imported code.
