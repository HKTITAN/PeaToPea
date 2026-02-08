package dev.peapod.android

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

/**
 * Placeholder activity for PeaPod Android. Protocol implementation (VPNService, discovery, transport)
 * will be added per .tasks/03-android.md. PeaCore (JNI to pea-core) is used when libpea_core.a is linked.
 */
class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(android.R.layout.activity_list_item)
        // Exercise JNI: create core and get device ID (no-op when using stub)
        val handle = PeaCore.nativeCreate()
        if (handle != 0L) {
            PeaCore.nativeDeviceId(handle)
            PeaCore.nativeDestroy(handle)
        }
    }
}
