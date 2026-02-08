# PeaPod Android

Android protocol implementation for PeaPod (VPNService, discovery, transport). Uses pea-core (Rust) via JNI when NDK is configured.

## Environment

- **Android Studio** or **SDK + NDK** (see [Build](#build)).
- **Minimum SDK**: 24 (Android 7.0).
- **Target SDK**: 34 (Android 14).
- **NDK**: Required to build and link the Rust pea-core static library.
  - **Install**: Android Studio → SDK Manager → SDK Tools → NDK (Side by side), or:  
    `sdkmanager --install "ndk;25.2.9519653"` (use a version ≥ 25).
  - **Path**: Set `ndk.dir` in `pea-android/local.properties`, e.g.  
    `ndk.dir=C\:\\Users\\You\\AppData\\Local\\Android\\Sdk\\ndk\\25.2.9519653`  
    or leave unset to use `ANDROID_HOME/ndk/<version>` (Gradle picks the `ndkVersion` from the app build).

## Build

From **Android Studio**: Open the `pea-android` directory and build (or run). From the **command line** (with [Gradle](https://gradle.org/install) or the wrapper present):

```bash
./gradlew assembleDebug
# or from repo root: ./pea-android/gradlew -p pea-android assembleDebug
```

### Linking pea-core (Rust)

From the **repo root**, build pea-core for each ABI and put the static libs where the app expects them:

```bash
# Add targets once
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

# Build release staticlibs (output under target/<triple>/release/libpea_core.a)
for abi in aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android; do
  cargo build -p pea-core --target $abi --release
done

# Copy into pea-android so CMake can link (optional: or set PEA_CORE_LIB_DIR)
mkdir -p pea-android/rust-out/{arm64-v8a,armeabi-v7a,x86,x86_64}
cp target/aarch64-linux-android/release/libpea_core.a pea-android/rust-out/arm64-v8a/
cp target/armv7-linux-androideabi/release/libpea_core.a pea-android/rust-out/armeabi-v7a/
cp target/i686-linux-android/release/libpea_core.a pea-android/rust-out/x86/
cp target/x86_64-linux-android/release/libpea_core.a pea-android/rust-out/x86_64/
```

Then build the app; CMake links `libpea_core.a` from `pea-android/rust-out/<abi>/` (or set `PEA_CORE_LIB_DIR` in gradle.properties to the repo `target` layout). If the libs are missing, the native build is skipped (see app `build.gradle.kts`).

## Tasks

See [.tasks/03-android.md](../.tasks/03-android.md) for the full Android implementation checklist.
