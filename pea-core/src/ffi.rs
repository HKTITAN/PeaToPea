//! Minimal C ABI for linking pea-core as a static library from Android (NDK) or other C/C++ hosts.
//! Full JNI or C API can be added in platform implementations (e.g. 03-android).

use crate::protocol::PROTOCOL_VERSION;

/// Returns the current protocol version. Used so the staticlib exports a C symbol and is linkable.
#[no_mangle]
pub extern "C" fn pea_core_version() -> u8 {
    PROTOCOL_VERSION
}
