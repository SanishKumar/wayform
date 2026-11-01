# Native legacy route sessions

`LegacyRouteSession` connects the existing legacy search and expiry lease in one Rust library handle. It plans against one owned overlay and only returns its path after checking that same lease. This is a native integration API, not a port of a complete VoiceGIS navigation UI or a new JSON protocol.

## Starting and reading a session

1. Verify a complete venue package in the host, then build `LegacySessionVenue::new(nodes, edges, package_projection)`. Both projections are validated, and the policy edge inventory must exactly match the graph. Share the immutable venue through `Arc`.
2. Construct `LegacySessionRequest` with start/destination IDs and the legacy `accessible_only` and `allow_restricted` flags. There is no caller-supplied closure list.
3. Call `LegacyRouteSession::new(venue, request, overlay, wall_ms, monotonic_ms)`. It owns the overlay, creates and reads its lease, and plans using only that lease's resolved closures. Policy rejection happens before search, including stationary and unknown endpoint handling. Future policies reject construction as unavailable; activation requires a new explicit start.
4. Sample both clocks again after planning, then call `read(wall_ms, monotonic_ms)` before presenting guidance. Construction exposes no path, so a policy that expires during planning is blocked by the first read.

`read` returns one of these views:

| Result | Meaning |
| --- | --- |
| `Ready { path_ids, total_distance, overlay_id }` | The owned route's policy is current at the supplied clocks |
| `PolicyUnavailable(freshness)` | Guidance must stop; includes the lease's reason and overlay identity |
| `Invalidated(reason)` | The host permanently withdrew this session |
| `Err(error)` | Guidance must stop; never substitute an empty closure list |

The path is borrowed from the session. Ordinary reads check clocks without repeating search or policy resolution. There is no public route getter, reset, replacement-policy method, serialization or session clone. An unavailable view has no path or distance. The underlying lease permanently latches expiry and clock failures, including while a host is suspended. A stationary route receives the same checks.

Planning failures return `LegacySessionError`: an unavailable policy, unknown endpoint, no route, inventory mismatch, or an underlying policy/search validation, resource or numeric error. No partially usable session is returned. The existing graph and policy work limits apply.

## Policy changes and recovery

Call `invalidate(reason)` as soon as the host learns of a relevant change. Reasons are `PolicyChanged`, `PolicyRemoved`, `RouteContextChanged` and `Cancelled`. Route-context changes include package/profile changes, a changed start or destination, or localization uncertainty requiring a different plan. Invalidation clears the stored path and retains the first explicit reason; later reads cannot reactivate it. It takes precedence over later clock checks because the route has already been withdrawn.

For a policy update, invalidate the old session **before** attempting to verify or plan its replacement. An invalid replacement must not leave the old route active. Even a replacement with the same overlay ID requires this handling when its content changes. Policy removal has no implicit transition to an open or policy-free route. This API requires an overlay; the existing batch CLI's explicit `none` mode remains separate.

Recovery requires a new explicit session after the host verifies current inputs and handles any clock or localization fault. Do not repeatedly reconstruct sessions to erase clock history. The host must decide when recovery is justified; constructing a new handle cannot establish that a clock is trustworthy or a lift is operating.

## Host responsibilities and limits

`Ready` applies only to the supplied readings. It is not permission to keep displaying a copied route indefinitely. Read again before each display/speech action and on resume; arrange periodic checks while guidance is visible. Clear displayed instructions and cancel queued speech on any non-ready result, error or invalidation. A route view copied earlier does not override a subsequent failure. The library does not run timers, read clocks or cancel platform output itself.

The host must use a consistent monotonic clock for the whole session, sample fresh clocks after planning, and serialize policy-change events with guidance delivery. It must invalidate immediately on relevant changes and avoid using stale queued results. System clock sources, policy fetching/authentication, revision floors, persistence across process restarts, package hashing and localization remain host responsibilities.

Matching inventories only prevents mismatched edge sets; it does not prove geometry or source mappings came from the claimed package hash. The session preserves legacy accessibility flags and undirected route semantics. It does not certify physical passability or implement the mobility draft's measured-evidence semantics. Returned paths contain node IDs and distance, not native turn instructions, graphics or legacy receipts. Android/Swift bindings and a running application guidance loop remain separate work. The batch CLI is unchanged and stateless.

## Validation

Native integration tests replay all 26 frozen search-kernel cases through session planning, converting their explicit closures to overlays without changing expectations. Lifecycle tests cover source closures and access profiles, both expiry deadlines, expiry between planning and first read, permanent clock failure, stationary routes, policy rejection before endpoint handling, explicit invalidation, mismatched inventories, owned inputs, independent sessions and resource/numeric failures. Existing lease traces and full legacy route conformance continue to run unchanged.

Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`. These tests establish library behavior on synthetic inputs; they do not validate a deployed guidance loop or real-world navigation.
