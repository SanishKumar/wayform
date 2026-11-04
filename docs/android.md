# Android application

The Android app currently provides a searchable offline catalog of four synthetic VoiceGIS venues. It verifies package bytes with the Rust core before storing them in SQLite and verifies them again when opening a venue. The selected venue survives process restarts. Failed imports cannot replace stored bytes. No network permission is requested. Route guidance screens are still being integrated.

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

The initial catalog was checked on an Android 15 / API 35 x86_64 emulator: all three device tests passed, including the Compose search and venue-selection flow. Lint reported no errors. The app limits stored venue packages to 1 MiB to keep database rows within older Android cursor windows.
