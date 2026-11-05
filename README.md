# WAYFORM

WAYFORM is an offline Android wayfinding app for getting from a venue entrance to a specific room. Its survey planner checks which paths have enough measured clearance, slope and step evidence for a visitor’s requirements.

**[Download the Android preview](https://github.com/SanishKumar/wayform/releases/download/v0.1.0-preview.1/wayform-0.1.0-preview.1.apk)** · [Release notes and checksum](https://github.com/SanishKumar/wayform/releases/tag/v0.1.0-preview.1) · [Build from source](docs/android.md)

The public preview runs in airplane mode and uses fictional venues. You can also open a private survey file on the device for a dated route assessment. **Real-world navigation still needs a surveyed venue and a supervised field trial.**

| Find a destination | Preview a floor | Check a width requirement |
| --- | --- | --- |
| <img src="docs/images/destination-search.png" width="245" alt="Searching for cardiology finds Heart and Vascular Clinic in the offline venue catalog"> | <img src="docs/images/floor-preview.png" width="245" alt="Ground-floor plan with a route line and a 71 metre, nine-instruction preview"> | <img src="docs/images/survey-route.png" width="245" alt="A 90 centimetre width requirement selects the 18 metre side-lobby route in the fictional clinic"> |

## Try it in three minutes

Install the preview APK on an Android 8.0 or newer arm64 phone or x86_64 emulator. You can open the downloaded APK on the device, or use `adb install -r wayform-0.1.0-preview.1.apk`. The preview installs as `app.wayform.preview`. Its bundled examples need no account or network.

1. **Walk through a sample journey.** In Asterion University Medical Center, search for **cardiology** and open **Heart & Vascular Clinic**. Look at the floor plan, open **Venue conditions → All modeled paths open**, confirm **Civic Plaza Entrance** as the starting place, then tap **Start sample journey**. Advance through the nine instructions or use **I'm not here** to exercise recovery. Keep the optional hard mobility requirements blank for this legacy sample.
2. **See a measurement change the route.** Return to places and choose **Survey routes → Try the clinic example**. Under **Mobility requirements**, set only the minimum clear width. Save and tap **Check route** after each change.

   | Minimum clear width | Expected result from Plaza entrance to Reception |
   | --- | --- |
   | 75 cm | The 12 m path through the front door qualifies. |
   | 90 cm | The front door measures 82 ± 1 cm. The planner selects the 18 m path through the side lobby. |
   | 120 cm | Both entrances fall below the requirement. The app reports **No eligible path**. |
   | 90 cm plus surveyed zero steps | The example contains only width measurements. The app reports **More evidence needed**. |

3. **Check your own file.** Choose **Open survey file** and select a mobility bundle. The app uses the device clock, the chosen endpoints and your saved requirements. The built-in example uses a fixed time so the results above can be reproduced. [Example JSON](examples/survey/clinic-entrance.json) · [Prepare a venue file](docs/venue-pilot.md)

## How it works

- **Android** uses Kotlin, Jetpack Compose and SQLite. Four bundled venues support offline search, floor maps, written directions, QR or manual check-in and optional offline English speech. Visitors confirm their location and advance instructions themselves. Expired conditions, location recovery and suspension withdraw active guidance. [Android behavior and build](docs/android.md)
- **Rust** owns routing and policy evaluation. The survey planner evaluates directed edges against physical measurements, uncertainty, provenance and freshness. Only eligible edges enter the path search. A result includes the chosen segments and evidence. Missing evidence and an excluded path have separate outcomes. Android calls the same Rust code through UniFFI. [Mobility contract](docs/mobility-contract.md) · [Native route protocol](docs/native-mobility-protocol.md) · [Mobile bindings](docs/mobile-bindings.md)
- **VoiceGIS compatibility** preserves 35 upstream Git blobs at commit `305c70bd418d7fd2b6f0ebf7a2ecd0ddcb9b9b7f`. The migration compares 7,238 complete synthetic route results with the TypeScript reference, including step-free routing, instructions and receipts. The imported source retains its original rights notice. [Compatibility and provenance](docs/compatibility.md) · [Native parity](docs/native-legacy-routes.md) · [Upstream notice](compat/voicegis/upstream/README.md)

## Reproduce the checks

From a clone with Node.js 22.14+ and npm, run the contract tests and the upstream journey audit.

```sh
npm ci --ignore-scripts --no-audit --no-fund
npm run check
npm run compat:audit
```

With the pinned Rust toolchain installed, run the native suite and compare native results with the reference fixtures.

```sh
cargo test --workspace --locked
npm run test:native
```

The Android 15 emulator suite covers offline venue loading, navigation and recovery, policy expiry, survey imports and route changes. To reproduce it, build the native bindings first, connect an arm64 or x86_64 device, then run the following from `apps/android`. The Android guide includes Windows setup and toolchain versions. [Android guide](docs/android.md)

```sh
./gradlew :app:connectedDebugAndroidTest :app:testDebugUnitTest :app:lintDebug
```

[Compatibility CI](https://github.com/SanishKumar/wayform/actions/workflows/quality.yml) · [Android build CI](https://github.com/SanishKumar/wayform/actions/workflows/android.yml)

## Taking it to a real venue

The first field milestone is one operator-approved entrance-to-room journey. It needs a scaled plan, surveyed entrance registration and per-segment measurements, a person responsible for closures, and a supervised trial on physical phones. Survey imports currently support planning. Live guidance for imported surveys still needs map and sign binding and authenticated, expiring operating conditions. [Pilot inputs and field checks](docs/venue-pilot.md)
