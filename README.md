# WAYFORM

Foundations for mobility-aware outdoor-to-indoor navigation. This repository currently contains a pinned VoiceGIS compatibility harness and a proposed mobility-data contract. It is not a deployed navigation product.

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

Native [edge eligibility](docs/native-eligibility.md) combines physical requirements with access rules, closures and entrance registration. The [document loader](docs/native-loading.md) validates complete bundle/profile/context JSON and computes their identities before evaluating edges or issuing [supplied-path receipts](docs/native-path-receipts.md). The `wayform-mobility` [document protocol](docs/native-mobility-protocol.md) exposes supplied-path assessment and suitable-route selection over JSON lines. Legacy routing parity, Android screens, Go services, geographic ingestion and production synchronization are later work.

An isolated [legacy search component](docs/native-legacy-search.md) preserves VoiceGIS's undirected adjacency and queue tie behavior. Its component tests cover node-path selection and distance; full legacy output parity remains incomplete.

## Updating baselines

`npm run compat:record` explicitly regenerates expected legacy results. Review every change against the pinned source and documented compatibility decisions; never use regeneration to silence a regression. `npm run schema:write` exports schemas from their TypeScript definitions. Contract examples are explicitly recorded with `node --import tsx scripts/record-contract-examples.ts --record`.

The [native measurement corpus](docs/native-measurements.md) is recorded with `node --import tsx scripts/record-measurement-examples.ts --record`. Both TypeScript and Rust tests read the committed expectations; normal tests do not rewrite them.

The imported VoiceGIS README retains its original rights statement. No new license is imposed on the imported code.
