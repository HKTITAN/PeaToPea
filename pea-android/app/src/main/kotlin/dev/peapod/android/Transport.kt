package dev.peapod.android

import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlin.concurrent.thread

/**
 * Local transport per .tasks/03-android §4: TCP server (45679), TCP client to discovered peers,
 * handshake (49 bytes: version + device_id + public_key), then length-prefixed encrypted frames.
 * Same wire format as Windows (pea-windows/transport.rs).
 */
object Transport {

    private const val HANDSHAKE_SIZE = 49
    private const val LEN_SIZE = 4
    private const val MAX_FRAME_LEN = 16 * 1024 * 1024
    private const val TICK_INTERVAL_MS = 1000L
    private const val OUTBUF_SIZE = 65536

    @Volatile
    private var serverSocket: ServerSocket? = null

    @Volatile
    private var running = false

    private data class PeerSender(
        val deviceId: ByteArray,
        val output: DataOutputStream,
        val sessionKey: ByteArray,
        val writeNonce: AtomicLong
    ) {
        override fun equals(other: Any?) = other is PeerSender && deviceId.contentEquals(other.deviceId)
        override fun hashCode() = deviceId.contentHashCode()
    }

    private val peerSenders = ConcurrentHashMap<String, PeerSender>()
    private val peerSendersLock = Object()

    @Volatile
    private var coreHandle: Long = 0L

    fun start(core: Long) {
        if (core == 0L) return
        if (running) return
        coreHandle = core
        running = true
        try {
            val server = ServerSocket(Discovery.LOCAL_TRANSPORT_PORT)
            server.reuseAddress = true
            serverSocket = server
            thread(name = "TransportAccept") { acceptLoop() }
            thread(name = "TransportTick") { tickLoop() }
        } catch (e: Exception) {
            e.printStackTrace()
            running = false
        }
    }

    fun stop() {
        running = false
        try { serverSocket?.close() } catch (_: Exception) {}
        serverSocket = null
        synchronized(peerSendersLock) {
            peerSenders.values.forEach { try { it.output.close() } catch (_: Exception) {} }
            peerSenders.clear()
        }
        coreHandle = 0L
    }

    /** Connect to a discovered peer (call from Discovery.onPeerDiscovered). */
    fun connectTo(deviceId: ByteArray, publicKey: ByteArray, addr: java.net.InetAddress, port: Int) {
        val idKey = deviceId.joinToString("") { "%02x".format(it) }
        if (peerSenders.containsKey(idKey)) return
        thread(name = "TransportConnect-$idKey") {
            try {
                val socket = Socket()
                socket.soTimeout = 30000
                socket.connect(InetSocketAddress(addr, port), 10000)
                val (peerId, sessionKey) = handshakeConnect(socket, deviceId, publicKey) ?: run {
                    socket.close()
                    return@thread
                }
                addPeerAndRunReadLoop(socket, peerId, sessionKey)
            } catch (_: Exception) {}
        }
    }

    private fun acceptLoop() {
        val server = serverSocket ?: return
        while (running && serverSocket != null) {
            try {
                val socket = server.accept()
                socket.soTimeout = 30000
                thread {
                    try {
                        val (peerId, sessionKey) = handshakeAccept(socket) ?: run {
                            socket.close()
                            return@thread
                        }
                        addPeerAndRunReadLoop(socket, peerId, sessionKey)
                    } catch (_: Exception) {
                        try { socket.close() } catch (_: Exception) {}
                    }
                }
            } catch (_: Exception) {
                if (running) break
            }
        }
    }

    private fun handshakeAccept(socket: Socket): Pair<ByteArray, ByteArray>? {
        val input = DataInputStream(socket.getInputStream())
        val output = DataOutputStream(socket.getOutputStream())
        val buf = ByteArray(HANDSHAKE_SIZE)
        input.readFully(buf)
        if (buf[0].toInt() and 0xFF != PeaCore.PROTOCOL_VERSION) return null
        val peerId = buf.copyOfRange(1, 17)
        val peerPublic = buf.copyOfRange(17, 49)
        val sessionKey = ByteArray(32)
        if (PeaCore.nativeSessionKey(coreHandle, peerPublic, sessionKey) != 0) return null
        val ourHandshake = ByteArray(HANDSHAKE_SIZE)
        if (PeaCore.nativeHandshakeBytes(coreHandle, ourHandshake) != 0) return null
        output.write(ourHandshake)
        output.flush()
        return peerId to sessionKey
    }

    private fun handshakeConnect(socket: Socket, expectDeviceId: ByteArray, expectPublicKey: ByteArray): Pair<ByteArray, ByteArray>? {
        val input = DataInputStream(socket.getInputStream())
        val output = DataOutputStream(socket.getOutputStream())
        val ourHandshake = ByteArray(HANDSHAKE_SIZE)
        if (PeaCore.nativeHandshakeBytes(coreHandle, ourHandshake) != 0) return null
        output.write(ourHandshake)
        output.flush()
        val buf = ByteArray(HANDSHAKE_SIZE)
        input.readFully(buf)
        if (buf[0].toInt() and 0xFF != PeaCore.PROTOCOL_VERSION) return null
        val peerId = buf.copyOfRange(1, 17)
        val peerPublic = buf.copyOfRange(17, 49)
        if (!peerId.contentEquals(expectDeviceId)) return null
        val sessionKey = ByteArray(32)
        if (PeaCore.nativeSessionKey(coreHandle, peerPublic, sessionKey) != 0) return null
        return peerId to sessionKey
    }

    private fun addPeerAndRunReadLoop(socket: Socket, peerId: ByteArray, sessionKey: ByteArray) {
        val idKey = peerId.joinToString("") { "%02x".format(it) }
        val sender = PeerSender(peerId, DataOutputStream(socket.getOutputStream()), sessionKey, AtomicLong(0))
        synchronized(peerSendersLock) {
            peerSenders[idKey]?.let { try { it.output.close() } catch (_: Exception) {} }
            peerSenders[idKey] = sender
        }
        runReadLoop(socket, peerId, sessionKey)
        synchronized(peerSendersLock) { peerSenders.remove(idKey) }
        try { socket.close() } catch (_: Exception) {}
        PeaCore.nativePeerLeft(coreHandle, peerId, null)
    }

    private fun runReadLoop(socket: Socket, peerId: ByteArray, sessionKey: ByteArray) {
        val input = DataInputStream(socket.getInputStream())
        val outBuf = ByteArray(OUTBUF_SIZE)
        var readNonce = 0L
        val idKey = peerId.joinToString("") { "%02x".format(it) }
        try {
            while (running) {
                val lenBuf = ByteArray(4)
                input.readFully(lenBuf)
                val len = ByteBuffer.wrap(lenBuf).order(ByteOrder.LITTLE_ENDIAN).int and 0x7FFF_FFFF
                if (len <= 0 || len > MAX_FRAME_LEN) break
                val cipher = ByteArray(len)
                input.readFully(cipher)
                val plainBuf = ByteArray(len + 1024)
                val plainLen = PeaCore.nativeDecryptWire(sessionKey, readNonce, cipher, plainBuf)
                if (plainLen <= 0) break
                readNonce++
                val plain = plainBuf.copyOfRange(0, plainLen)
                val resultLen = PeaCore.nativeOnMessageReceived(coreHandle, peerId, plain, outBuf)
                if (resultLen < 0) continue
                parseAndSendOutbound(outBuf, resultLen, idKey)
            }
        } catch (_: Exception) {}
    }

    /** Parse out_buf from on_message_received: 4 body_len, body?, then 4 count, each (16 peer_id, 4 len, payload). Send each payload to the peer (encrypted). */
    private fun parseAndSendOutbound(buf: ByteArray, len: Int, excludeIdKey: String) {
        if (len < 4) return
        var off = 0
        val bodyLen = ByteBuffer.wrap(buf, 0, 4).order(ByteOrder.LITTLE_ENDIAN).int and 0x7FFF_FFFF
        off += 4
        if (bodyLen > 0 && off + bodyLen <= len) off += bodyLen
        if (off + 4 > len) return
        val count = ByteBuffer.wrap(buf, off, 4).order(ByteOrder.LITTLE_ENDIAN).int and 0x7FFF_FFFF
        off += 4
        repeat(count) {
            if (off + 16 + 4 > len) return@repeat
            val peerId = buf.copyOfRange(off, off + 16)
            off += 16
            val payloadLen = ByteBuffer.wrap(buf, off, 4).order(ByteOrder.LITTLE_ENDIAN).int and 0x7FFF_FFFF
            off += 4
            if (off + payloadLen > len) return@repeat
            val payload = buf.copyOfRange(off, off + payloadLen)
            off += payloadLen
            sendToPeer(peerId, payload)
        }
    }

    private fun sendToPeer(peerId: ByteArray, plain: ByteArray) {
        val idKey = peerId.joinToString("") { "%02x".format(it) }
        val sender = peerSenders[idKey] ?: return
        val cipherBuf = ByteArray(plain.size + 16)
        val n = PeaCore.nativeEncryptWire(sender.sessionKey, sender.writeNonce.getAndIncrement(), plain, cipherBuf)
        if (n <= 0) return
        try {
            synchronized(sender.output) {
                val lenBuf = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putInt(n).array()
                sender.output.write(lenBuf)
                sender.output.write(cipherBuf, 0, n)
                sender.output.flush()
            }
        } catch (_: Exception) {}
    }

    private fun tickLoop() {
        val outBuf = ByteArray(OUTBUF_SIZE)
        while (running && coreHandle != 0L) {
            Thread.sleep(TICK_INTERVAL_MS)
            val n = PeaCore.nativeTick(coreHandle, outBuf)
            if (n > 0) parseOutboundActions(outBuf, n)
        }
    }

    /** Parse tick output: 4 count LE, then each (16 peer_id, 4 len LE, payload). */
    private fun parseOutboundActions(buf: ByteArray, len: Int) {
        if (len < 4) return
        var off = 0
        val count = ByteBuffer.wrap(buf, 0, 4).order(ByteOrder.LITTLE_ENDIAN).int and 0x7FFF_FFFF
        off += 4
        repeat(count) {
            if (off + 16 + 4 > len) return@repeat
            val peerId = buf.copyOfRange(off, off + 16)
            off += 16
            val payloadLen = ByteBuffer.wrap(buf, off, 4).order(ByteOrder.LITTLE_ENDIAN).int and 0x7FFF_FFFF
            off += 4
            if (off + payloadLen > len) return@repeat
            val payload = buf.copyOfRange(off, off + payloadLen)
            off += payloadLen
            sendToPeer(peerId, payload)
        }
    }
}
