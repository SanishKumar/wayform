# Complete legacy guidance sessions

`LegacyGuidanceSession` binds a verified package's complete route result to the exact operational policy and evaluation time used to plan it. It builds the result only after a current lease has been established. Missing paths, rejected policy and inconsistent mapped centerlines cannot construct a guidance handle.

Construction exposes no guidance. The host must sample wall and monotonic clocks again after planning and call `read` before displaying or speaking any result. A ready read includes the path, landmark instructions, graphic assessment and receipt together. Expiry, clock failure, policy errors and explicit invalidation withdraw the entire stored result. There is no reset, replacement or unchecked route getter. Diagnostic formatting does not print the route.

`invalidate` preserves the first cancellation/context/policy-change reason. New packages, profiles, policies or confirmed start positions require a new session. A lost position or app suspension should clear rendered/queued guidance and require location recovery. The host owns these events; the core does not observe cameras, lifecycle callbacks or external files.

`next_check_delay` forwards the lease's bounded scheduling hint. It does not authorize retaining or speaking an earlier result. Hosts still need fresh reads on every guidance action and must cancel speech and clear route output whenever a read is unavailable. Timers cannot guarantee delivery while a mobile application is suspended.

The original projection-based `LegacyRouteSession` remains available for search-only compatibility. Mobile integration should use this complete-package session. Legacy sessions require an explicit operational overlay; no policy means a snapshot preview, not a current operational journey. Authenticating an imported policy and trusting wall time remain host responsibilities.

Tests cover complete-result binding, expiry before the first read, monotonic expiry, clock failures, permanent invalidation, expired stationary journeys, closed paths and inconsistent geometry. These tests establish software behavior, not real-world sign placement or route passability.
