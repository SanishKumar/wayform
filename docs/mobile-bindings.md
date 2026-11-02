# Mobile native bindings

`wayform-mobile` exports coarse UniFFI 0.32.2 bindings under Kotlin package `app.wayform.core`. `MobileVenue` owns a fully verified legacy venue. It exposes visitor/map metadata, QR resolution, snapshot previews and explicit-policy trip creation. `MobileTrip` serializes reads and invalidation through a mutex; each read checks the complete guidance session's wall and monotonic clocks before returning any route. Expired or invalidated reads contain no route.

JSON results retain the tested native contracts. Package/policy bytes reach the core parser without an intermediate JSON decoder, preserving duplicate-name rejection. Responses are limited to 16 MiB and scanned payloads to 4,096 bytes. No networking, camera acquisition, timers, storage or speech lives inside this crate.

`plan_mobility` exposes the separate draft bundle/profile/context route evaluation. It returns a suitable-path assessment, insufficient-evidence diagnostic or infeasibility diagnostic. It is a snapshot API, not a live mobility journey. A legacy authored flag does not become a measurement through these bindings.

The step-free option applies the legacy authored exclusions and rejects a resulting route containing stairs or escalators, even if incorrectly marked accessible. This guard withholds an inconsistent result; it does not search for a replacement. It does not verify widths, slopes or surfaces. The binding crate requires Rust 1.88 or newer for its UniFFI dependencies; the core's minimum remains 1.85.

Build Android libraries with `bash scripts/build-android-core.sh` from the repository root. Install the pinned Rust toolchain and Android NDK `28.2.13676358`, and set `ANDROID_HOME`. The script builds API 26 libraries for `arm64-v8a` and `x86_64`, generates Kotlin bindings from the host library and writes them under `target/android-mobile/`. Android CI publishes that directory as an artifact with its source commit. Generated bindings and compiled libraries are build outputs, not tracked source.

Kotlin requires JNA's Android AAR, version 5.12 or newer. Binding checksums remain enabled to detect mismatched generated code and native libraries. Call native planning off the UI thread, retain one trip handle, check it before every display/speech action, and invalidate it on policy/profile/package/location changes or suspension. Clear rendered guidance and stop speech when a read is unavailable. Closing a Kotlin object releases its native reference; it does not cancel another retained reference automatically.

The binding crate contains UniFFI-generated FFI glue. The separate navigation crate retains its ban on unsafe Rust. No native runtime behavior is duplicated in Kotlin.
