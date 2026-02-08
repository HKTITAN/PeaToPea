package dev.peapod.android

import android.os.Bundle
import android.widget.ArrayAdapter
import android.widget.Spinner
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SwitchCompat

/**
 * Settings screen (.tasks/03-android §6.2, §7.1.2): battery saver, start on boot, battery threshold.
 */
class SettingsActivity : AppCompatActivity() {

    private val thresholdValues = intArrayOf(5, 10, 15, 20, 25, 30)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        val batterySaver = findViewById<SwitchCompat>(R.id.settings_battery_saver)
        val startOnBoot = findViewById<SwitchCompat>(R.id.settings_start_on_boot)
        val thresholdSpinner = findViewById<Spinner>(R.id.settings_battery_threshold)

        batterySaver.isChecked = PeaPodPreferences.batterySaver(this)
        startOnBoot.isChecked = PeaPodPreferences.startOnBoot(this)

        thresholdSpinner.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_item, resources.getStringArray(R.array.battery_threshold_options))
            .also { it.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item) }
        val currentThreshold = PeaPodPreferences.batteryThresholdPercent(this)
        val idx = thresholdValues.indexOfFirst { it >= currentThreshold }.coerceIn(0, thresholdValues.size - 1)
        thresholdSpinner.setSelection(idx)

        batterySaver.setOnCheckedChangeListener { _, isChecked ->
            PeaPodPreferences.setBatterySaver(this, isChecked)
        }
        startOnBoot.setOnCheckedChangeListener { _, isChecked ->
            PeaPodPreferences.setStartOnBoot(this, isChecked)
        }
        thresholdSpinner.setOnItemSelectedListener(object : android.widget.AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent: android.widget.AdapterView<*>?, view: android.view.View?, position: Int, id: Long) {
                PeaPodPreferences.setBatteryThresholdPercent(this@SettingsActivity, thresholdValues[position])
            }
            override fun onNothingSelected(parent: android.widget.AdapterView<*>?) {}
        })
    }
}
