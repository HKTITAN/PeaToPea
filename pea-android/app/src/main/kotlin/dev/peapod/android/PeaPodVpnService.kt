package dev.peapod.android

import android.content.Intent
import android.net.VpnService
import android.os.IBinder

/**
 * VPN service for PeaPod traffic interception (§2.1).
 * Placeholder: establish() and packet handling will be implemented per .tasks/03-android.md.
 */
class PeaPodVpnService : VpnService() {

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // TODO: start foreground with notification; call establish(); run packet loop
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? {
        return super.onBind(intent)
    }
}
