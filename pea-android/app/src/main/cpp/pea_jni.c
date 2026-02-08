#include <jni.h>
#include <stdint.h>

extern uint8_t pea_core_version(void);

JNIEXPORT jint JNI_OnLoad(JavaVM *vm, void *reserved) {
    (void)reserved;
    (void)vm;
    (void)pea_core_version;
    return JNI_VERSION_1_6;
}
