# VoiceGIS compatibility baseline

## Source identity and extraction

Upstream: https://github.com/SanishKumar/voicegis-indoor-ar

Pinned source commit: `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`.

`scripts/import-voicegis.mjs` reads committed Git blobs, not potentially modified working files. It extracts 35 files: the original direction audit and its transitive source dependencies, package verification, compiler dependencies, the local policy lease, four source/package/report triples, one lift-outage overlay, workspace manifests and the upstream README. This is an executable reference subset, not a new editable copy of the full application.

To reproduce extraction into a fresh WAYFORM checkout without an existing import:

```sh
node scripts/import-voicegis.mjs /path/to/voicegis-indoor-ar
```

The source repository must contain the pinned commit. The importer refuses to overwrite an existing source lock. It does not change the source repository. Ordinary installation/testing uses only the vendored files and the npm lockfile; Git access is not required.

`source-lock.json` records each exact file size, SHA-256 digest and original Git blob SHA-1. Validation checks the complete inventory, including unexpected files. `.gitattributes` disables newline conversion for pinned bytes, expected results and exported schemas. The commit and file count are separately pinned in tests. These checks detect drift; they are not a signature from the original publisher.

## What is preserved

| Venue | Synthetic route evaluations |
| --- | ---: |
| Reference Medical Centre | 242 |
| Asterion Medical Center | 2,496 |
| Harbor Exchange | 200 |
| Meridian Park Campus | 4,300 |
| Total | 7,238 |

For both legacy profiles, every public destination is reached from every other public place and every scannable check-in anchor. Same-node pairs are excluded using the original audit's rules. Each stored result fingerprint covers the entire returned object: path, nodes, distances, instructions, graphic clearance and policy receipt. Object keys are sorted for hashing; array order and numeric outputs are retained. This captures the selected path and legacy tie behavior, not merely a success count.

The harness also asserts the audit invariants directly: route found, drawable graphics, correct endpoints, no U-turns or duplicate adjacent instructions, and no stairs/escalators in step-free instructions. `compat:audit` executes the original script unchanged as a second check.

Before routing, each package passes the original shape/version/reference/hash verifier. Each source is compiled twice, and both the package and validation report must match the exact committed artifact bytes. Full representative results are retained in `representative-routes.json`; `route-digests.jsonl` records per-query hashes to avoid duplicating millions of bytes of repeated graph nodes.

## Negative and policy cases

`policy-cases.json` records twelve full route results: standard, step-free, lift outage under each profile, expired policy, future policy, missing evaluation time, wrong package binding, unknown closure target, unknown start, unknown destination and same-node routing. Tests assert that closure failure never substitutes stairs for the step-free profile. The expiry boundary is exclusive.

The baseline includes a current → expired → still-unavailable lease sequence, plus explicit rejection of content tampering and missing accessibility flags. All evaluation times are fixture inputs, never the test machine's current clock.

## Legacy quirks remain isolated

- The original core is distance-based, undirected and uses unordered endpoint-pair lookup. Its exact behavior remains frozen here.
- The low-level core defaults an absent accessibility flag to allowed; the verified-package entry rejects the missing flag first. WAYFORM never imports this fallback into its new contract.
- Authored graphic clearance is a rendering safeguard, not a wheelchair clearance measurement.
- Original `standard` / `wheelchair` identifiers are retained in old receipts; they do not become individualized suitability claims.
- Compiler `northOffsetDegrees`, plan origin, portal widths and flags are not promoted into measured geographic registration or survey evidence.

The adapter emits two distinct directed arcs for each legacy undirected edge, retaining each source edge ID and assertion. It does not invent a reverse slope, observation, policy feed or geographic transform. Escalators conservatively map to `stairs` in this draft; the preserved source-edge identity makes the loss explicit. Authored portal topology is preserved, but this adapter does not claim a complete physical door model.

## Baseline change policy

Run `npm run compat:check` to compare, or `npm run compat:record` only for a reviewed baseline update. The tests never accept new expected values automatically. On a mismatch, the test identifies the route row/query or changed artifact. Inspect the result with the preserved `calculateCompiledRoute` entry point and the recorded query; readable samples cover one route per venue/profile and all policy cases.

A future source refresh must separately review the source lock, dependency lock, counts, compiler outputs and behavior changes. Do not edit `upstream/` to implement WAYFORM features. The new native engine will become authoritative for the new contract after parity gates pass; the old subset stays a frozen oracle.

The full original browser, localization, authoring and device test suites are outside this extracted harness. Passing it is not a claim that all VoiceGIS behavior has been migrated.

The [native legacy search component](native-legacy-search.md) preserves selected node paths, distances and heuristic scales without changing the immutable upstream subset or replacing the original full-result fingerprints. Its policy protocol resolves overlays before routing, with the documented canonical-UTC boundary. A separate [native lease](native-legacy-leases.md) compares 105 clock traces with the original lifecycle rules. Instructions, package verification, route graphics, receipts and application-level journey integration still require separate native work.
