# PeaPod Android

Android protocol implementation for PeaPod (VPNService, discovery, transport). Uses pea-core (Rust) via JNI when NDK is configured.

## Environment

- **Android Studio** or **SDK + NDK** (see [Build](#build)).
- **Minimum SDK**: 24 (Android 7.0).
- **Target SDK**: 34 (Android 14).
- **NDK**: Required to build and link the Rust pea-core static library; version r25c or later recommended. Path is typically `$ANDROID_HOME/ndk/<version>` or set in `local.properties` as `ndk.dir`.

## Build

From **Android Studio**: Open the `pea-android` directory and build (or run). From the **command line** (with [Gradle](https://gradle.org/install) or the wrapper present):

```bash
./gradlew assembleDebug
# or from repo root: ./pea-android/gradlew -p pea-android assembleDebug
```

To build pea-core for Android and link (after 00 §4.2):

```bash
# From repo root
cargo build -p pea-core --target aarch64-linux-android --release
# Then configure app build to use the .a and link; see .tasks/03-android.md
```

## Tasks

See [.tasks/03-android.md](../.tasks/03-android.md) for the full Android implementation checklist.
