package dev.peapod.android

import android.content.Context
import android.content.SharedPreferences

/** App preferences for §6.2 (battery saver, start on boot). */
object PeaPodPreferences {
    private const val PREFS_NAME = "peapod_settings"
    private const val KEY_BATTERY_SAVER = "battery_saver"
    private const val KEY_START_ON_BOOT = "start_on_boot"

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun batterySaver(context: Context): Boolean =
        prefs(context).getBoolean(KEY_BATTERY_SAVER, false)

    fun setBatterySaver(context: Context, value: Boolean) {
        prefs(context).edit().putBoolean(KEY_BATTERY_SAVER, value).apply()
    }

    fun startOnBoot(context: Context): Boolean =
        prefs(context).getBoolean(KEY_START_ON_BOOT, false)

    fun setStartOnBoot(context: Context, value: Boolean) {
        prefs(context).edit().putBoolean(KEY_START_ON_BOOT, value).apply()
    }
}
