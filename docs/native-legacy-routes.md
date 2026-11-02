# Complete native legacy route results

`LegacyVenuePackage::calculate_route(start, destination, options)` composes policy resolution, deterministic legacy search, graphic assessment, landmark directions and the `0.1.0` legacy route receipt. It returns the original JSON result shape. `LegacyCompiledOptions` accepts the two access flags, an optional owned operational overlay and an optional explicit evaluation time. The receipt profile is `wheelchair` when `accessible_only` is true, otherwise `standard`.

Invalid operational overlays reject the request before endpoint handling, including stationary routes. Unreachable paths have an unroutable receipt. Invalid authored centerlines return a rejected receipt without path or directions. A valid centerline whose graphic envelope does not fit retains written directions and a withheld display assessment. Receipts include the package identity, applied closures, excluded-edge counts, selected connectors, policy issues and route distance.

Input absence is explicit: `None` means no operational overlay; a supplied malformed value is rejected rather than treated as absence. Selected connector IDs currently require ASCII letters, digits and hyphens so the receipt ordering can reproduce the pinned runtime's English collation without a platform locale dependency. Other IDs return `UnsupportedConnectorId`. All four pinned venues meet this boundary. The original aliases and runtime-generated current time are not part of this API; callers supply the profile flags and evaluation time explicitly.

Eight complete representative results and the original twelve policy outcomes are compared with untouched reference results. Eleven altered-map cases verify that invalid geometry cannot leak guidance while insufficient graphic clearance can retain text. Bearing comparisons use the existing nine-decimal comparison boundary; other result fields are exact. The component-level corpora separately cover all 7,238 original paths, base/landmark instructions and display assessments.

This API is a snapshot. A mobile host must use a live session to gate displayed and spoken guidance, invalidate on policy/profile/package changes and reacquire location when needed. Neither an integrity hash nor a legacy `wheelchair` receipt proves physical accessibility. Individual measured requirements belong to the separate mobility contract.

## Complete-result protocol

`wayform-legacy-search` protocol `0.4.0` accepts the same verified `venuePackage` envelope and explicit per-query policy as `0.3.0`. Each result contains `queryId` and the complete legacy result, including its receipt. Invalid overlays are represented by rejected legacy receipts. Resource and unsupported-input failures instead contain `outcome: "rejected"` and an error code, with no path or directions. There is no separate `overlayResolution` field in this version.

Complete-result batches allow 1–100 queries and retain the 16 MiB request/response bounds. A response that exceeds the bound rejects the batch without partial results. Versions `0.1.0` through `0.3.0` retain their existing response shapes and limits.

`scripts/native-legacy-route-conformance.ts` sends all 7,238 pinned journeys through this protocol in bounded batches. Each expected complete result must first match its original fingerprint; then native and reference paths, metadata, directions, graphic assessments and receipts are compared, with only the documented bearing rounding. This runs in addition to the retained 21,868 search/projection comparisons. CLI tests cover expired stationary journeys, lift outages, unknown endpoints, tampering, projection overrides and the complete-result query limit.
