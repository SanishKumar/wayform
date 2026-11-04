# Android application

The Android app provides a searchable offline catalog of four synthetic VoiceGIS venues, floor maps, native route previews and manual step-by-step guidance. Starting guidance requires an explicitly confirmed starting place and current operating conditions. The selected venue and authored stairs-exclusion option survive process restarts; active journeys and location confirmations do not. No network permission is requested.

The conditions panel explicitly simulates open paths, closed lifts or a 30-second expiry in the bundled fictional venues. These simulations are not operator reports. Each active journey owns a native policy lease. The host checks wall and monotonic clocks before displaying the initial result, advancing a step and at the lease's timer deadline. Expiry, clock failure, changed conditions, recovery and suspension remove the route. Backgrounding requires a new location confirmation. Movement and arrival are confirmed manually; there is no continuous positioning.

QR scanning uses the optional camera permission only when the scanner is opened. A typed-code fallback uses the same native resolver. Recognition proposes a start; the user must confirm being beside the named sign. Unknown or foreign codes cannot confirm a position. Speech selects an installed English voice that does not require a network connection. Every read-aloud action checks the native lease, and changing steps, expiry, recovery and suspension stop speech. Written directions remain available if no offline voice is installed.

Minimum clear width, maximum slope and surveyed zero-step requirements are saved on the device. The Android flow currently loads legacy venue packages, which do not supply surveyed evidence. Enabling any of these hard requirements withholds route previews and guidance and reports missing evidence; it never converts the authored access flags into measurements. The separate draft mobility snapshot API is available in the native bindings, but draft mobility-bundle import is not part of this Android slice.

The conditions panel also accepts a legacy overlay JSON file through Android's document picker. Reads are limited to 64 KiB. Original bytes reach the native parser, which checks duplicates, venue/package binding, closure references and validity before use. Importing or replacing conditions withdraws the current trip first; failed imports cannot restore it. Conditions remain in memory and must be loaded again after process death. The import does not authenticate the operator or establish that the reported conditions match the physical venue.

The app targets Android 8.0 (API 26) and later on arm64 or x86_64. Build requirements are JDK 17, the pinned Rust toolchain, Android platform 36 and NDK 28.2.13676358. The Gradle wrapper pins Gradle 8.13 and its distribution checksum.

From the repository root on Linux or macOS:

```sh
bash scripts/build-android-core.sh
cd apps/android
./gradlew :app:assembleDebug :app:lintDebug
```

On Windows, generate the native outputs on a compatible build host or download the matching `android-native-core` artifact from a successful workflow run, verify its source commit and unpack it into `target/android-mobile`. Then run `gradlew.bat` from `apps/android`. `-PwayformNativeDir=/absolute/path` selects another native output directory. Generated Kotlin and `.so` files must come from the same build; UniFFI checks their interface checksums at runtime.

The APK is written to `apps/android/app/build/outputs/apk/debug/app-debug.apk`. This is a development build, not a signed store release. Android CI compiles both device test and app APKs and runs lint; a successful build alone does not establish device behavior.

With an arm64 or x86_64 emulator/device connected:

```sh
cd apps/android
./gradlew :app:connectedDebugAndroidTest
```

Device tests exercise the actual native library, verified catalog and route preview, rejection of corrupt packages, SQLite reopen and failed-write preservation. Sample assets are copied from the pinned compatibility source at build time; their original README and source lock are included in the APK. They are not surveyed real venues. Hash verification detects changed bytes, not publisher authenticity or physical accessibility.

The app was checked on an Android 15 / API 35 x86_64 emulator: all twelve device tests and two JVM unit tests passed. They cover native package loading, persistence, search, previews, recognized/foreign check-in payloads, lease expiry and clock rollback, closed lifts, location recovery, backgrounding, venue switching, hard requirements and rejected condition imports. Lint reported no errors. Physical camera recognition and voice audibility still require phone checks. The app limits stored venue packages to 1 MiB to keep database rows within older Android cursor windows.

Map coordinates use the venue's plan axes. Pinch and pan change the view; the reset control restores it. Route strokes never exceed the core's checked physical drawing width, and are withheld if its graphic assessment has issues. Endpoint markers are screen-space annotations. Floor changes do not imply a detected user position. The preview has no current operational lease and must not be presented as live guidance.
