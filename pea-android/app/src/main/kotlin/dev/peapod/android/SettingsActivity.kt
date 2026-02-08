package dev.peapod.android

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat

/**
 * Settings screen (.tasks/03-android §6.2): battery saver, start on boot.
 */
class SettingsActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        val batterySaver = findViewById<SwitchCompat>(R.id.settings_battery_saver)
        val startOnBoot = findViewById<SwitchCompat>(R.id.settings_start_on_boot)

        batterySaver.isChecked = PeaPodPreferences.batterySaver(this)
        startOnBoot.isChecked = PeaPodPreferences.startOnBoot(this)

        batterySaver.setOnCheckedChangeListener { _, isChecked ->
            PeaPodPreferences.setBatterySaver(this, isChecked)
        }
        startOnBoot.setOnCheckedChangeListener { _, isChecked ->
            PeaPodPreferences.setStartOnBoot(this, isChecked)
        }
    }
}
