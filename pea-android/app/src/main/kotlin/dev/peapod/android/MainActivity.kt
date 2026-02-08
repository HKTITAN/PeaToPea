package dev.peapod.android

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

/**
 * Placeholder activity for PeaPod Android. Protocol implementation (VPNService, discovery, transport)
 * will be added per .tasks/03-android.md.
 */
class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(android.R.layout.activity_list_item)
    }
}
