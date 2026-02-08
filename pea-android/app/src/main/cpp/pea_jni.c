#include <jni.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/* pea-core C API (from pea-core/src/ffi.rs); stubbed in pea_stub.c when not linked */
extern uint8_t pea_core_version(void);
extern void* pea_core_create(void);
extern void pea_core_destroy(void* h);
extern int pea_core_device_id(void* h, void* out_buf, size_t out_len);
extern int pea_core_on_request(void* h, const uint8_t* url, size_t url_len,
    uint64_t range_start, uint64_t range_end, uint8_t* out_buf, size_t out_buf_len);
extern int pea_core_peer_joined(void* h, const uint8_t* device_id_16, const uint8_t* public_key_32);
extern int pea_core_peer_left(void* h, const uint8_t* device_id_16, uint8_t* out_buf, size_t out_buf_len);
extern int pea_core_on_message_received(void* h, const uint8_t* peer_id_16,
    const uint8_t* msg, size_t msg_len, uint8_t* out_buf, size_t out_buf_len);
extern int pea_core_on_chunk_received(void* h, const uint8_t* transfer_id_16,
    uint64_t start, uint64_t end, const uint8_t* hash_32,
    const uint8_t* payload, size_t payload_len, uint8_t* out_buf, size_t out_buf_len);
extern int pea_core_tick(void* h, uint8_t* out_buf, size_t out_buf_len);

#define PEA_CORE_JNI "dev/peapod/android/PeaCore"

JNIEXPORT jint JNI_OnLoad(JavaVM *vm, void *reserved) {
    (void)reserved;
    (void)vm;
    (void)pea_core_version;
    return JNI_VERSION_1_6;
}

JNIEXPORT jlong JNICALL
Java_dev_peapod_android_PeaCore_nativeCreate(JNIEnv *env, jclass clazz) {
    (void)env;
    (void)clazz;
    return (jlong)(uintptr_t)pea_core_create();
}

JNIEXPORT void JNICALL
Java_dev_peapod_android_PeaCore_nativeDestroy(JNIEnv *env, jclass clazz, jlong handle) {
    (void)env;
    (void)clazz;
    pea_core_destroy((void*)(uintptr_t)handle);
}

JNIEXPORT jbyteArray JNICALL
Java_dev_peapod_android_PeaCore_nativeDeviceId(JNIEnv *env, jclass clazz, jlong handle) {
    (void)clazz;
    uint8_t buf[16];
    if (pea_core_device_id((void*)(uintptr_t)handle, buf, 16) != 0)
        return NULL;
    jbyteArray out = (*env)->NewByteArray(env, 16);
    if (out)
        (*env)->SetByteArrayRegion(env, out, 0, 16, (jbyte*)buf);
    return out;
}

JNIEXPORT jint JNICALL
Java_dev_peapod_android_PeaCore_nativeOnRequest(JNIEnv *env, jclass clazz, jlong handle,
    jstring url, jlong rangeStart, jlong rangeEnd, jbyteArray outBuf) {
    (void)clazz;
    if (!url || !outBuf) return -1;
    const char* url_chars = (*env)->GetStringUTFChars(env, url, NULL);
    if (!url_chars) return -1;
    size_t url_len = strlen(url_chars);
    jsize out_len = (*env)->GetArrayLength(env, outBuf);
    jbyte* out = (*env)->GetByteArrayElements(env, outBuf, NULL);
    if (!out) {
        (*env)->ReleaseStringUTFChars(env, url, url_chars);
        return -1;
    }
    int r = pea_core_on_request((void*)(uintptr_t)handle,
        (const uint8_t*)url_chars, url_len,
        (uint64_t)rangeStart, (uint64_t)rangeEnd,
        (uint8_t*)out, (size_t)out_len);
    (*env)->ReleaseByteArrayElements(env, outBuf, out, 0);
    (*env)->ReleaseStringUTFChars(env, url, url_chars);
    return (jint)r;
}

JNIEXPORT jint JNICALL
Java_dev_peapod_android_PeaCore_nativePeerJoined(JNIEnv *env, jclass clazz, jlong handle,
    jbyteArray deviceId, jbyteArray publicKey) {
    (void)clazz;
    if (!deviceId || (*env)->GetArrayLength(env, deviceId) < 16) return -1;
    if (!publicKey || (*env)->GetArrayLength(env, publicKey) < 32) return -1;
    jbyte* id = (*env)->GetByteArrayElements(env, deviceId, NULL);
    jbyte* pk = (*env)->GetByteArrayElements(env, publicKey, NULL);
    if (!id || !pk) return -1;
    int r = pea_core_peer_joined((void*)(uintptr_t)handle, (uint8_t*)id, (uint8_t*)pk);
    (*env)->ReleaseByteArrayElements(env, deviceId, id, JNI_ABORT);
    (*env)->ReleaseByteArrayElements(env, publicKey, pk, JNI_ABORT);
    return (jint)r;
}

JNIEXPORT jint JNICALL
Java_dev_peapod_android_PeaCore_nativePeerLeft(JNIEnv *env, jclass clazz, jlong handle,
    jbyteArray deviceId, jbyteArray outBuf) {
    (void)clazz;
    if (!deviceId || (*env)->GetArrayLength(env, deviceId) < 16) return -1;
    jbyte* id = (*env)->GetByteArrayElements(env, deviceId, NULL);
    if (!id) return -1;
    uint8_t* out = outBuf && (*env)->GetArrayLength(env, outBuf) > 0
        ? (uint8_t*)(*env)->GetByteArrayElements(env, outBuf, NULL) : NULL;
    size_t out_len = out ? (size_t)(*env)->GetArrayLength(env, outBuf) : 0;
    int r = pea_core_peer_left((void*)(uintptr_t)handle, (uint8_t*)id, out, out_len);
    (*env)->ReleaseByteArrayElements(env, deviceId, id, JNI_ABORT);
    if (out) (*env)->ReleaseByteArrayElements(env, outBuf, (jbyte*)out, 0);
    return (jint)r;
}

JNIEXPORT jint JNICALL
Java_dev_peapod_android_PeaCore_nativeOnMessageReceived(JNIEnv *env, jclass clazz, jlong handle,
    jbyteArray peerId, jbyteArray msg, jbyteArray outBuf) {
    (void)clazz;
    if (!peerId || !msg || !outBuf) return -1;
    jbyte* pid = (*env)->GetByteArrayElements(env, peerId, NULL);
    jbyte* m = (*env)->GetByteArrayElements(env, msg, NULL);
    jbyte* out = (*env)->GetByteArrayElements(env, outBuf, NULL);
    if (!pid || !m || !out) {
        if (pid) (*env)->ReleaseByteArrayElements(env, peerId, pid, JNI_ABORT);
        if (m) (*env)->ReleaseByteArrayElements(env, msg, m, JNI_ABORT);
        if (out) (*env)->ReleaseByteArrayElements(env, outBuf, out, JNI_ABORT);
        return -1;
    }
    jsize msg_len = (*env)->GetArrayLength(env, msg);
    jsize out_len = (*env)->GetArrayLength(env, outBuf);
    int r = pea_core_on_message_received((void*)(uintptr_t)handle,
        (uint8_t*)pid, (uint8_t*)m, (size_t)msg_len, (uint8_t*)out, (size_t)out_len);
    (*env)->ReleaseByteArrayElements(env, peerId, pid, JNI_ABORT);
    (*env)->ReleaseByteArrayElements(env, msg, m, JNI_ABORT);
    (*env)->ReleaseByteArrayElements(env, outBuf, out, 0);
    return (jint)r;
}

JNIEXPORT jint JNICALL
Java_dev_peapod_android_PeaCore_nativeOnChunkReceived(JNIEnv *env, jclass clazz, jlong handle,
    jbyteArray transferId, jlong start, jlong end, jbyteArray hash, jbyteArray payload,
    jbyteArray outBuf) {
    (void)clazz;
    if (!transferId || !hash || !payload || !outBuf) return -1;
    jbyte* tid = (*env)->GetByteArrayElements(env, transferId, NULL);
    jbyte* h = (*env)->GetByteArrayElements(env, hash, NULL);
    jbyte* p = (*env)->GetByteArrayElements(env, payload, NULL);
    jbyte* out = (*env)->GetByteArrayElements(env, outBuf, NULL);
    if (!tid || !h || !p || !out) {
        if (tid) (*env)->ReleaseByteArrayElements(env, transferId, tid, JNI_ABORT);
        if (h) (*env)->ReleaseByteArrayElements(env, hash, h, JNI_ABORT);
        if (p) (*env)->ReleaseByteArrayElements(env, payload, p, JNI_ABORT);
        if (out) (*env)->ReleaseByteArrayElements(env, outBuf, out, JNI_ABORT);
        return -1;
    }
    jsize payload_len = (*env)->GetArrayLength(env, payload);
    jsize out_len = (*env)->GetArrayLength(env, outBuf);
    int r = pea_core_on_chunk_received((void*)(uintptr_t)handle,
        (uint8_t*)tid, (uint64_t)start, (uint64_t)end, (uint8_t*)h,
        (uint8_t*)p, (size_t)payload_len, (uint8_t*)out, (size_t)out_len);
    (*env)->ReleaseByteArrayElements(env, transferId, tid, JNI_ABORT);
    (*env)->ReleaseByteArrayElements(env, hash, h, JNI_ABORT);
    (*env)->ReleaseByteArrayElements(env, payload, p, JNI_ABORT);
    (*env)->ReleaseByteArrayElements(env, outBuf, out, 0);
    return (jint)r;
}

JNIEXPORT jint JNICALL
Java_dev_peapod_android_PeaCore_nativeTick(JNIEnv *env, jclass clazz, jlong handle, jbyteArray outBuf) {
    (void)clazz;
    if (!outBuf) return 0;
    jbyte* out = (*env)->GetByteArrayElements(env, outBuf, NULL);
    if (!out) return 0;
    jsize out_len = (*env)->GetArrayLength(env, outBuf);
    int r = pea_core_tick((void*)(uintptr_t)handle, (uint8_t*)out, (size_t)out_len);
    (*env)->ReleaseByteArrayElements(env, outBuf, out, 0);
    return (jint)r;
}
