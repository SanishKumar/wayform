# Native legacy expiry lease

`LegacyOperationalLease` ports the local lease in VoiceGIS `operationalLease.ts` at commit `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`. The original source and rights statement remain under `compat/voicegis/upstream/`. It retains one overlay's clock history so a host can block guidance after policy expiry or clock failure.

The constructor takes an `Arc<LegacyPolicyIndex>`, an owned overlay JSON value, Unix wall time in milliseconds, and a monotonic reading in milliseconds. The policy index is immutable and can be shared across leases. The overlay is owned; later changes to the caller's source data cannot change the lease. Each lease has independent clock state and cannot be cloned or reset through its API.

`read(now_wall_ms, now_monotonic_ms)` returns `LegacyOperationalFreshness` with `status: "current" | "unavailable"`, `overlayId` and a nullable reason. A returned error must also block guidance. The lease always represents a supplied overlay: malformed or null data never becomes the separate legacy `none` state.

## Clock and expiry behavior

At creation, the monotonic deadline is fixed at `initialMonotonic + (validUntil - initialWall)`. A read latches expiry when either wall time reaches `validUntil` or monotonic time reaches that deadline. Later readings cannot clear expiry.

The lease tracks the greatest wall and monotonic readings observed by `read`. A wall rollback of more than 1,000 ms from that high-water value, any monotonic rollback, or an unusable clock latches `clock-unreliable`. Exactly 1,000 ms of wall rollback is tolerated. Several small backward steps are compared with the high-water value, so their cumulative retreat can trigger failure. Clock failure also cannot clear on later reads.

For resolved policies, reason precedence matches the reference: clock failure, expiry, then the first overlay issue. A future policy initially returns `overlay-not-active`. It is resolved again when wall time reaches its start; ordinary reads reuse the cached resolution. As in the reference, a small tolerated wall correction after activation does not return the cached policy to its future state. The fixed expiry deadline still applies.

`delay(now_wall_ms, now_monotonic_ms)` returns a scheduling hint from 1 to 1,000 ms, based on the earlier remaining duration. It may be fractional. Calling it does not update the lease, latch failure or authorize guidance. A timer is only a wakeup: the host must call `read` again before use, including after resuming from suspension.

## Host integration

Create the lease from the same verified package projection and overlay used for the route. Retain that instance throughout the journey. Before displaying or speaking usable guidance, read both clocks and require `LegacyFreshnessStatus::Current`; pause on `Unavailable` or any error. Do not reconstruct a fresh lease on every read, infer validity from the delay, or replace failure with explicit policy absence.

Changed or removed policy data requires the host to invalidate affected guidance and obtain/revalidate its replacement. The lease cannot observe external mutations or a missing feed because it intentionally owns a snapshot. Package verification, source/route binding, authentication, revision floors, trusted clock sources, persistence across application restarts and user recovery remain host responsibilities. A local lease is not proof that a lift is working or a route is physically passable.

This change supplies the Rust library component and conformance traces. The batch routing CLI remains stateless and does not create live journey handles. Android/Swift bindings and a navigation session that combines route ownership, policy changes and lease checks are separate integration work.

## Native boundaries and errors

- Overlay timestamps use the resolver's canonical `YYYY-MM-DDTHH:mm:ss.sssZ` format and valid four-digit Gregorian years. Broader `Date.parse` normalization is outside compatibility.
- Finite wall readings are truncated toward zero to milliseconds for date conversion, as with JavaScript `Date`. Clock comparison and deadline arithmetic retain the supplied binary64 values. Wall readings outside the native year range latch clock failure; this is narrower than JavaScript's full date range.
- Non-finite initial clocks or a non-finite computed deadline mark the native lease unreliable immediately. This is a conservative construction boundary; the reference reports invalid initial clocks on its first read. Delay hints before that first read are not claimed equivalent for invalid initial clocks.
- The existing policy limits apply: at most 500,000 closure records and one million closure visits plus expanded edge matches per resolution. Excessive closure counts reject construction. Resolution errors are cached, remain errors on subsequent reads, and never yield a partial freshness result.
- Inputs are already parsed JSON values. Hosts must bound JSON size and parsing work before construction; `parse_bounded_json` provides the existing 16 MiB limit and duplicate-field rejection. The lease does not read system time, start timers, fetch policy data or access storage.

## Conformance

`compat/voicegis/expected/operational-leases.json` contains 105 traces and 332 read/delay results, generated from the unchanged TypeScript lease with copied overlays. Groups share package projections, including the pinned Asterion fixture. Non-finite clock values use explicit `NaN`, `Infinity` and `-Infinity` fixture strings; these are test encodings, not a new wire protocol.

The TypeScript test recomputes complete expectations. Rust tests replay them in order and compare statuses, reasons, identities and delay values. Cases cover both expiry clocks, permanent failure, rollback thresholds, fractional clocks, future activation, invalid policies, calendar boundaries and timer hints that do not mutate state. Additional native tests check owned inputs, independent leases, cached resolution, resource failures and the stricter timestamp range.

Run `npm run check` and `cargo test --workspace --locked`. Expectations are recorded only with `node --import tsx scripts/record-legacy-lease-examples.ts --record`. Original route fingerprints, policy outcomes and overlay fixtures remain unchanged. CI also runs the existing route conformance suite on Linux and Windows.
