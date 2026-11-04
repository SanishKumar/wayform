# Android application

The Android app provides a searchable offline catalog of four synthetic VoiceGIS venues, floor maps, native route previews and manual step-by-step guidance. Starting guidance requires an explicitly confirmed starting place and current operating conditions. The selected venue and authored stairs-exclusion option survive process restarts; active journeys and location confirmations do not. No network permission is requested.

The conditions panel explicitly simulates open paths, closed lifts or a 30-second expiry in the bundled fictional venues. These simulations are not operator reports. Each active journey owns a native policy lease. The host checks wall and monotonic clocks before displaying the initial result, advancing a step and at the lease's timer deadline. Expiry, clock failure, changed conditions, recovery and suspension remove the route. Backgrounding requires a new location confirmation. Movement and arrival are confirmed manually; there is no continuous positioning.

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

The app was checked on an Android 15 / API 35 x86_64 emulator: all eight device tests passed. They cover native package loading, persistence, search, previews, lease expiry and clock rollback, closed lifts, location recovery and backgrounding. Lint reported no errors. The app limits stored venue packages to 1 MiB to keep database rows within older Android cursor windows.

Map coordinates use the venue's plan axes. Pinch and pan change the view; the reset control restores it. Route strokes never exceed the core's checked physical drawing width, and are withheld if its graphic assessment has issues. Endpoint markers are screen-space annotations. Floor changes do not imply a detected user position. The preview has no current operational lease and must not be presented as live guidance.
