# PeaPod Android

Android protocol implementation for PeaPod (VPNService, discovery, transport). Uses pea-core (Rust) via JNI when NDK is configured.

**Note:** There is no one-line installer for Android. Build the APK using Android Studio or Gradle (see [Build](#build) below).

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

Then build the app; CMake links `libpea_core.a` from `pea-android/rust-out/<abi>/`. If the libs are missing, the stub (`pea_stub.c`) is used and JNI calls return safe defaults (e.g. `PeaCore.nativeCreate()` returns 0).

**JNI API:** `dev.peapod.android.PeaCore` exposes native methods that call into pea-core's C FFI: create/destroy, deviceId, onRequest, peerJoined, peerLeft, onMessageReceived, onChunkReceived, tick. See `pea-core/src/ffi.rs` for the C layout of request result and outbound actions.

**Optional:** WiFi Direct (Wi-Fi P2P) for discovery is documented as optional in [.tasks/03-android.md](../.tasks/03-android.md) §3.2.

### Release build and signing (§8.2)

To build a signed release AAB/APK, set Gradle properties (e.g. in `pea-android/gradle.properties` or environment):

- `RELEASE_STORE_FILE`: path to keystore file  
- `RELEASE_STORE_PASSWORD`  
- `RELEASE_KEY_ALIAS`  
- `RELEASE_KEY_PASSWORD`  

Then run `gradle assembleRelease` or `gradle bundleRelease` (AAB). If these are not set, the release build type still builds but uses no signing (or debug); configure signing for Play Store or sideload.

### CI

GitHub Actions (`.github/workflows/ci.yml`) builds pea-core for `aarch64-linux-android` and `x86_64-linux-android`, copies libs into `rust-out/`, and runs `gradle assembleDebug`. Test the resulting APK on an emulator (x86_64) or real device (arm64).

## Tasks

See [.tasks/03-android.md](../.tasks/03-android.md) for the full Android implementation checklist.
