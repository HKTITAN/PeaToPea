package dev.peapod.android

import java.net.DatagramPacket
import java.net.InetAddress
import java.net.MulticastSocket
import kotlin.concurrent.thread

/**
 * LAN discovery per .tasks/03-android §3.1 and 07: UDP multicast 239.255.60.60:45678,
 * periodic beacon (device_id, public_key, listen_port), receive and parse beacons/responses,
 * maintain peer list, call core on_peer_joined / on_peer_left. Advertises listen_port for local transport (§4).
 */
object Discovery {

    const val DISCOVERY_PORT = 45678
    const val LOCAL_TRANSPORT_PORT = 45679
    private const val MULTICAST_GROUP = "239.255.60.60"
    private const val BEACON_INTERVAL_MS = 4000L
    private const val PEER_TIMEOUT_MS = 16000L
    private const val TIMEOUT_CHECK_MS = 4000L
    private const val BEACON_FRAME_MAX = 256

    @Volatile
    private var socket: MulticastSocket? = null

    @Volatile
    private var running = false

    private data class PeerEntry(
        val deviceId: ByteArray,
        val publicKey: ByteArray,
        val addr: InetAddress,
        val port: Int,
        var lastSeen: Long
    ) {
        override fun equals(other: Any?) = other is PeerEntry && deviceId.contentEquals(other.deviceId)
        override fun hashCode() = deviceId.contentHashCode()
    }

    @Volatile
    private var peers = mutableMapOf<String, PeerEntry>()
    private val peersLock = Any()

    /** Optional: called when peer set changes (join or timeout). Use to update notification. */
    @Volatile
    var onPeerCountChanged: (() -> Unit)? = null

    /** Optional: called when a new peer is discovered (deviceId, publicKey, addr, port) so transport can connect. */
    @Volatile
    var onPeerDiscovered: ((ByteArray, ByteArray, java.net.InetAddress, Int) -> Unit)? = null

    /** Start discovery: bind multicast, send beacon loop, receive loop, timeout loop. Call when VPN/core is up. */
    fun start(coreHandle: Long, listenPort: Int) {
        if (coreHandle == 0L) return
        if (running) return
        running = true
        try {
            val s = MulticastSocket(DISCOVERY_PORT)
            s.reuseAddress = true
            s.soTimeout = 2000
            s.joinGroup(InetAddress.getByName(MULTICAST_GROUP))
            socket = s
            val group = InetAddress.getByName(MULTICAST_GROUP)
            val beaconBuf = ByteArray(BEACON_FRAME_MAX)
            thread(name = "DiscoveryBeacon") {
                while (running && socket != null) {
                    val n = PeaCore.nativeBeaconFrame(coreHandle, listenPort, beaconBuf)
                    if (n > 0) {
                        try {
                            socket?.send(DatagramPacket(beaconBuf, n, group, DISCOVERY_PORT))
                        } catch (_: Exception) {}
                    }
                    Thread.sleep(BEACON_INTERVAL_MS)
                }
            }
            thread(name = "DiscoveryRecv") {
                recvLoop(coreHandle, listenPort)
            }
            thread(name = "DiscoveryTimeout") {
                timeoutLoop(coreHandle)
            }
        } catch (e: Exception) {
            e.printStackTrace()
            running = false
        }
    }

    private fun recvLoop(coreHandle: Long, listenPort: Int) {
        val s = socket ?: return
        val buf = ByteArray(65536)
        val packet = DatagramPacket(buf, buf.size)
        val myId = PeaCore.nativeDeviceId(coreHandle) ?: return
        val responseFrame = ByteArray(BEACON_FRAME_MAX)
        val responseLen = buildDiscoveryResponseFrame(coreHandle, listenPort, responseFrame)
        while (running && socket != null) {
            try {
                s.receive(packet)
                val n = packet.length
                if (n < 5) continue
                val frame = packet.data.copyOfRange(0, n)
                val outDeviceId = ByteArray(16)
                val outPublicKey = ByteArray(32)
                val outListenPort = IntArray(1)
                val ok = PeaCore.nativeDecodeDiscoveryFrame(frame, outDeviceId, outPublicKey, outListenPort)
                if (ok != 0) continue
                if (outDeviceId.contentEquals(myId)) continue
                val from = packet.address
                val peerPort = outListenPort[0].and(0xFFFF)
                val idKey = outDeviceId.joinToString("") { "%02x".format(it) }
                val isNew = synchronized(peersLock) {
                    val prev = peers[idKey]
                    peers[idKey] = PeerEntry(outDeviceId, outPublicKey, from, peerPort, System.currentTimeMillis())
                    prev == null
                }
                if (isNew) {
                    PeaCore.nativePeerJoined(coreHandle, outDeviceId, outPublicKey)
                    onPeerCountChanged?.invoke()
                    onPeerDiscovered?.invoke(outDeviceId, outPublicKey, from, peerPort)
                }
                if (responseLen > 0) {
                    try {
                        s.send(DatagramPacket(responseFrame, responseLen, from, packet.port))
                    } catch (_: Exception) {}
                }
            } catch (_: java.net.SocketTimeoutException) {
            } catch (e: Exception) {
                if (running) e.printStackTrace()
            }
        }
    }

    private fun buildDiscoveryResponseFrame(coreHandle: Long, listenPort: Int, outBuf: ByteArray): Int {
        return PeaCore.nativeDiscoveryResponseFrame(coreHandle, listenPort, outBuf)
    }

    private fun timeoutLoop(coreHandle: Long) {
        while (running) {
            Thread.sleep(TIMEOUT_CHECK_MS)
            val now = System.currentTimeMillis()
            val timedOut = mutableListOf<ByteArray>()
            synchronized(peersLock) {
                peers.entries.removeIf { (_, entry) ->
                    if (now - entry.lastSeen >= PEER_TIMEOUT_MS) {
                        timedOut.add(entry.deviceId)
                        true
                    } else false
                }
            }
            for (deviceId in timedOut) {
                PeaCore.nativePeerLeft(coreHandle, deviceId, null)
                onPeerCountChanged?.invoke()
            }
        }
    }

    /** Stop discovery (call from service onDestroy). */
    fun stop() {
        running = false
        try {
            socket?.close()
        } catch (_: Exception) {}
        socket = null
        synchronized(peersLock) { peers.clear() }
    }

    /** Current peer count for notification. */
    fun peerCount(): Int = synchronized(peersLock) { peers.size }
}
