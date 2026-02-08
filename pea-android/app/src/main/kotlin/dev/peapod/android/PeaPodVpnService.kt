package dev.peapod.android

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.IBinder
import android.os.ParcelFileDescriptor
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import kotlin.concurrent.thread

/**
 * VPN service for PeaPod traffic interception (.tasks/03-android §2.1, §2.4, §2.2).
 * Establishes tunnel (10.0.0.2/32, default route); runs as foreground with notification.
 * Local proxy on 127.0.0.1:3128 parses HTTP and calls core (§2.2.3–2.2.4). Tunnel packet read loop runs but does not yet redirect to proxy.
 */
class PeaPodVpnService : VpnService() {

    companion object {
        const val NOTIFICATION_CHANNEL_ID = "peapod_vpn"
        const val NOTIFICATION_ID = 1
        const val ACTION_DISCONNECT = "dev.peapod.android.DISCONNECT"
    }

    private var tunnelFd: ParcelFileDescriptor? = null
    private var coreHandle: Long = 0L

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_DISCONNECT -> {
                stopVpn()
                stopSelf()
                return START_NOT_STICKY
            }
        }
        if (tunnelFd != null) {
            return START_STICKY
        }
        val builder = Builder()
            .setSession(getString(R.string.app_name))
            .addAddress("10.0.0.2", 32)
            .addRoute("0.0.0.0", 0)
            .addDnsServer("8.8.8.8")
        tunnelFd = builder.establish()
        if (tunnelFd == null) {
            stopSelf()
            return START_NOT_STICKY
        }
        coreHandle = PeaCore.nativeCreate()
        LocalProxy.start(coreHandle, this)
        startTunnelReadLoop()
        startForeground(NOTIFICATION_ID, buildNotification(0))
        return START_STICKY
    }

    /** Read packets from tunnel (required so VPN doesn't stall). Full TCP redirect to proxy deferred. */
    private fun startTunnelReadLoop() {
        val fd = tunnelFd ?: return
        thread(name = "VpnTunnelRead") {
            val buf = ByteArray(32768)
            try {
                java.io.FileInputStream(fd.fileDescriptor).use { stream ->
                    while (stream.read(buf) >= 0) {
                        // TODO §2.2.1/2.2.2: parse IP/TCP and redirect to LocalProxy or relay
                    }
                }
            } catch (_: Exception) {}
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        stopVpn()
        super.onDestroy()
    }

    private fun stopVpn() {
        LocalProxy.stop()
        if (coreHandle != 0L) {
            PeaCore.nativeDestroy(coreHandle)
            coreHandle = 0L
        }
        tunnelFd?.close()
        tunnelFd = null
        stopForeground(STOP_FOREGROUND_REMOVE)
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                NOTIFICATION_CHANNEL_ID,
                getString(R.string.app_name),
                NotificationManager.IMPORTANCE_LOW
            ).apply { setShowBadge(false) }
            getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
        }
    }

    private fun buildNotification(peerCount: Int): android.app.Notification {
        val pendingOpen = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val pendingDisconnect = PendingIntent.getService(
            this, 0, Intent(this, PeaPodVpnService::class.java).setAction(ACTION_DISCONNECT),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val contentText = if (peerCount <= 0) getString(R.string.peapod_active)
        else getString(R.string.peapod_pod_devices, peerCount)
        return NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(contentText)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentIntent(pendingOpen)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, getString(R.string.disconnect), pendingDisconnect)
            .setOngoing(true)
            .build()
    }

    /** Update notification text (e.g. when peer count changes). Call from app when needed. */
    fun updateNotification(peerCount: Int) {
        NotificationManagerCompat.from(this).notify(NOTIFICATION_ID, buildNotification(peerCount))
    }
}
