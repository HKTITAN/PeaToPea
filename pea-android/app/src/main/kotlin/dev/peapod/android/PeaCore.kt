package dev.peapod.android

/**
 * JNI bridge to pea-core (Rust). Init core, feed request/peers/messages/chunks, tick.
 * See .tasks/03-android.md §1.2.3 and §5.1. When libpea_core.a is not linked, native calls
 * use stubs (e.g. nativeCreate returns 0).
 */
object PeaCore {
    init {
        System.loadLibrary("pea_jni")
    }

    /** Create core instance. Returns 0 if stub or failure. */
    @JvmStatic
    external fun nativeCreate(): Long

    /** Destroy core instance. */
    @JvmStatic
    external fun nativeDestroy(handle: Long)

    /** This device's ID (16 bytes), or null on error. */
    @JvmStatic
    external fun nativeDeviceId(handle: Long): ByteArray?

    /**
     * On incoming request. Returns 0 = Fallback, 1 = Accelerate (outBuf filled with assignment), -1 = error.
     * outBuf must be large enough for accelerate result (see pea-core ffi layout).
     */
    @JvmStatic
    external fun nativeOnRequest(
        handle: Long,
        url: String,
        rangeStart: Long,
        rangeEnd: Long,
        outBuf: ByteArray
    ): Int

    /** Peer joined. deviceId 16 bytes, publicKey 32 bytes. Returns 0 on success, -1 on error. */
    @JvmStatic
    external fun nativePeerJoined(handle: Long, deviceId: ByteArray, publicKey: ByteArray): Int

    /** Peer left. Optionally fills outBuf with outbound actions. Returns bytes written or 0. */
    @JvmStatic
    external fun nativePeerLeft(handle: Long, deviceId: ByteArray, outBuf: ByteArray?): Int

    /** Message received from peer. Fills outBuf with (body_len, body?, outbound_actions). Returns bytes written, -1 on error. */
    @JvmStatic
    external fun nativeOnMessageReceived(
        handle: Long,
        peerId: ByteArray,
        msg: ByteArray,
        outBuf: ByteArray
    ): Int

    /** Chunk received. Returns 0 = in progress, 1 = complete (reassembled body in outBuf), -1 = error. */
    @JvmStatic
    external fun nativeOnChunkReceived(
        handle: Long,
        transferId: ByteArray,
        start: Long,
        end: Long,
        hash: ByteArray,
        payload: ByteArray,
        outBuf: ByteArray
    ): Int

    /** Tick. Fills outBuf with serialized outbound actions. Returns bytes written or 0. */
    @JvmStatic
    external fun nativeTick(handle: Long, outBuf: ByteArray): Int

    /** Build discovery beacon frame. Returns bytes written to outBuf, or -1 on error. */
    @JvmStatic
    external fun nativeBeaconFrame(handle: Long, listenPort: Int, outBuf: ByteArray): Int

    /** Build DiscoveryResponse frame (send to beacon sender). Returns bytes written, or -1 on error. */
    @JvmStatic
    external fun nativeDiscoveryResponseFrame(handle: Long, listenPort: Int, outBuf: ByteArray): Int

    /** Decode Beacon or DiscoveryResponse frame. Fills outDeviceId (16), outPublicKey (32), outListenPort[0]. Returns 0 on success, -1 on error. */
    @JvmStatic
    external fun nativeDecodeDiscoveryFrame(
        frame: ByteArray,
        outDeviceId: ByteArray,
        outPublicKey: ByteArray,
        outListenPort: IntArray
    ): Int
}
