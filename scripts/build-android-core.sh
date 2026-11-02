#!/usr/bin/env bash
set -euo pipefail

# Run from the repository root with JDK 17 and the Android SDK installed.
: "${ANDROID_HOME:?Set ANDROID_HOME to the Android SDK directory}"
ndk="$ANDROID_HOME/ndk/28.2.13676358"
if [[ ! -d "$ndk" ]]; then
  echo 'Install Android NDK 28.2.13676358 with sdkmanager before building.' >&2
  exit 1
fi
target_dir="${CARGO_TARGET_DIR:-target}"
case "$(uname -s)" in
  Linux*) host=linux-x86_64 ; library="$target_dir/release/libwayform_mobile.so" ;;
  Darwin*) host=darwin-x86_64 ; library="$target_dir/release/libwayform_mobile.dylib" ;;
  *) echo 'Use Linux or macOS for this cross-build script.' >&2; exit 1 ;;
esac
output="${WAYFORM_NATIVE_OUTPUT:-$target_dir/android-mobile}"
mkdir -p "$output/kotlin" "$output/jniLibs/arm64-v8a" "$output/jniLibs/x86_64"
rustup target add aarch64-linux-android x86_64-linux-android
cargo build --package wayform-mobile --release --locked
cargo run --package wayform-mobile --features bindgen --bin wayform-bindgen --locked -- generate --library "$library" --language kotlin --config crates/wayform-mobile/uniffi.toml --out-dir "$output/kotlin"
toolchain="$ndk/toolchains/llvm/prebuilt/$host/bin"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$toolchain/aarch64-linux-android26-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$toolchain/x86_64-linux-android26-clang"
cargo build --package wayform-mobile --release --locked --target aarch64-linux-android --target x86_64-linux-android
cp "$target_dir/aarch64-linux-android/release/libwayform_mobile.so" "$output/jniLibs/arm64-v8a/"
cp "$target_dir/x86_64-linux-android/release/libwayform_mobile.so" "$output/jniLibs/x86_64/"
git rev-parse HEAD > "$output/source-commit.txt"
