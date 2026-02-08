package dev.peapod.android

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build

/**
 * If "Start on boot" is enabled, start the VPN service on device boot (.tasks/03-android §6.2.1).
 * VPN will only establish if the user had previously granted VPN consent.
 */
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        if (!PeaPodPreferences.startOnBoot(context)) return
        val serviceIntent = Intent(context, PeaPodVpnService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(serviceIntent)
        } else {
            context.startService(serviceIntent)
        }
    }
}
