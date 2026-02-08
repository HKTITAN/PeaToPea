package dev.peapod.android

import android.net.VpnService
import java.io.InputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import kotlin.concurrent.thread

/**
 * Local HTTP proxy in app (.tasks/03-android §2.2.3, §2.2.4).
 * Listens on 127.0.0.1:PROXY_PORT; parses request (method, Host, Range); calls PeaCore.nativeOnRequest.
 * On Fallback: protect socket, connect to origin, forward request/response. On Accelerate: fetch self-assigned chunks via WAN, pass to core, send reassembled response (§2.3); peer chunks require §4 local transport.
 */
object LocalProxy {

    const val PROXY_PORT = 3128
    private const val BUF_SIZE = 65536
    private const val MAX_HEADERS_LEN = 32768

    @Volatile
    private var serverSocket: ServerSocket? = null

    /** Start proxy in a background thread. coreHandle from PeaCore.nativeCreate(); vpnService for protect(). */
    fun start(
        coreHandle: Long,
        vpnService: VpnService,
    ) {
        if (coreHandle == 0L) return
        thread(name = "LocalProxy") {
            runServer(coreHandle, vpnService)
        }
    }

    /** Stop accepting new connections (call from service onDestroy). */
    fun stop() {
        try {
            serverSocket?.close()
        } catch (_: Exception) {}
        serverSocket = null
    }

    private fun runServer(coreHandle: Long, vpnService: VpnService) {
        try {
            val server = ServerSocket()
            server.reuseAddress = true
            server.bind(InetSocketAddress("127.0.0.1", PROXY_PORT))
            serverSocket = server
            while (serverSocket != null) {
                val client = try { server.accept() } catch (_: Exception) { break }
                thread {
                    try {
                        handleConnection(client, coreHandle, vpnService)
                    } catch (e: Exception) {
                        e.printStackTrace()
                    } finally {
                        try { client.close() } catch (_: Exception) {}
                    }
                }
            }
        } catch (e: Exception) {
            e.printStackTrace()
        } finally {
            serverSocket = null
        }
    }

    private fun handleConnection(
        client: Socket,
        coreHandle: Long,
        vpnService: VpnService,
    ) {
        val clientIn = client.getInputStream()
        val clientOut = client.getOutputStream()
        val (requestBytes, method, host, path, rangeStart, rangeEnd) = parseRequest(clientIn) ?: run {
            clientOut.write("HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n".toByteArray(StandardCharsets.US_ASCII))
            return
        }
        val url = buildUrl(host, path)
        val outBuf = ByteArray(65536)
        val action = PeaCore.nativeOnRequest(coreHandle, url, rangeStart, rangeEnd, outBuf)
        when (action) {
            0 -> {
                // Fallback: forward to origin (protect socket so it bypasses VPN)
                val origin = Socket()
                vpnService.protect(origin)
                val (hostOnly, port) = parseHostPort(host)
                origin.connect(InetSocketAddress(hostOnly, port))
                origin.getOutputStream().write(requestBytes)
                origin.getOutputStream().flush()
                thread {
                    try {
                        clientIn.copyTo(origin.getOutputStream())
                        origin.shutdownOutput()
                    } catch (_: Exception) {}
                }
                origin.getInputStream().copyTo(clientOut)
                clientOut.flush()
                origin.close()
            }
            1 -> {
                // Accelerate §2.3: parse assignment, fetch self chunks via WAN, pass to core, send reassembled response
                val acc = parseAccelerateResult(outBuf) ?: run {
                    clientOut.write("HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n".toByteArray(StandardCharsets.US_ASCII))
                    return
                }
                val selfId = PeaCore.nativeDeviceId(coreHandle) ?: run {
                    clientOut.write("HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n".toByteArray(StandardCharsets.US_ASCII))
                    return
                }
                val bodyBuf = ByteArray(acc.totalLength.coerceAtMost(Int.MAX_VALUE.toLong()).toInt())
                var complete = false
                for ((deviceId, start, end) in acc.assignments) {
                    if (!deviceId.contentEquals(selfId)) continue // peer chunks need §4 local transport
                    val payload = fetchChunkViaWan(vpnService, host, path, start, end) ?: continue
                    val hash = sha256(payload)
                    val result = PeaCore.nativeOnChunkReceived(coreHandle, acc.transferId, start, end, hash, payload, bodyBuf)
                    when (result) {
                        1 -> { complete = true; break }
                        -1 -> {
                            clientOut.write("HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n".toByteArray(StandardCharsets.US_ASCII))
                            return
                        }
                    }
                }
                if (!complete) {
                    clientOut.write("HTTP/1.1 504 Gateway Timeout\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".toByteArray(StandardCharsets.US_ASCII))
                    return
                }
                val (status, rangeHeader, bodySlice) = if (rangeEnd >= rangeStart && rangeStart >= 0) {
                    Triple(206, "Content-Range: bytes $rangeStart-$rangeEnd/${acc.totalLength}\r\n", bodyBuf)
                } else {
                    Triple(200, "", bodyBuf)
                }
                val headers = "HTTP/1.1 $status ${if (status == 206) "Partial Content" else "OK"}\r\nConnection: close\r\nContent-Length: ${bodySlice.size}\r\n$rangeHeader\r\n"
                clientOut.write(headers.toByteArray(StandardCharsets.US_ASCII))
                clientOut.write(bodySlice)
                clientOut.flush()
            }
            else -> {
                clientOut.write("HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n".toByteArray(StandardCharsets.US_ASCII))
            }
        }
    }

    /** Layout: 16 transfer_id, 8 total_length LE, 4 num LE, then num*(16 device_id, 8 start LE, 8 end LE). */
    private fun parseAccelerateResult(buf: ByteArray): AccelerateResult? {
        if (buf.size < 28) return null
        val bb = ByteBuffer.wrap(buf).order(java.nio.ByteOrder.LITTLE_ENDIAN)
        val totalLength = bb.getLong(16)
        val num = bb.getInt(24) and 0x7FFF_FFFF
        if (28 + num * 32 > buf.size) return null
        val assignments = (0 until num).map { i ->
            val base = 28 + i * 32
            val deviceId = buf.copyOfRange(base, base + 16)
            val start = bb.getLong(base + 16)
            val end = bb.getLong(base + 24)
            Triple(deviceId, start, end)
        }
        return AccelerateResult(buf.copyOfRange(0, 16), totalLength, assignments)
    }

    private data class AccelerateResult(
        val transferId: ByteArray,
        val totalLength: Long,
        val assignments: List<Triple<ByteArray, Long, Long>>,
    ) {
        override fun equals(other: Any?) = other is AccelerateResult && transferId.contentEquals(other.transferId) && totalLength == other.totalLength && assignments.size == other.assignments.size
        override fun hashCode() = transferId.contentHashCode() + 31 * totalLength.hashCode()
    }

    private fun sha256(input: ByteArray): ByteArray =
        MessageDigest.getInstance("SHA-256").digest(input)

    /** Fetch one range from origin via HTTP; socket protected so it bypasses VPN. Returns body bytes or null. */
    private fun fetchChunkViaWan(vpnService: VpnService, host: String, path: String, start: Long, end: Long): ByteArray? {
        val (hostOnly, port) = parseHostPort(host)
        return try {
            val socket = Socket()
            vpnService.protect(socket)
            socket.soTimeout = 30_000
            socket.connect(InetSocketAddress(hostOnly, port), 10_000)
            val request = "GET $path HTTP/1.1\r\nHost: $host\r\nRange: bytes=$start-$end\r\nConnection: close\r\n\r\n"
            socket.getOutputStream().write(request.toByteArray(StandardCharsets.US_ASCII))
            socket.getOutputStream().flush()
            val input = socket.getInputStream()
            val buf = ByteArray(8192)
            var n = 0
            while (n < buf.size) {
                val r = input.read(buf, n, buf.size - n)
                if (r <= 0) break
                n += r
                val s = String(buf, 0, n, StandardCharsets.US_ASCII)
                val headerEnd = s.indexOf("\r\n\r\n")
                if (headerEnd != -1) {
                    val bodyStart = headerEnd + 4
                    val bodyBytes = java.io.ByteArrayOutputStream()
                    bodyBytes.write(buf, bodyStart, n - bodyStart)
                    while (true) {
                        val more = input.read(buf)
                        if (more <= 0) break
                        bodyBytes.write(buf, 0, more)
                    }
                    socket.close()
                    return bodyBytes.toByteArray()
                }
            }
            socket.close()
            null
        } catch (_: Exception) {
            null
        }
    }

    /** Parse first line and headers; return (full request bytes, method, host, path, rangeStart, rangeEnd). */
    private fun parseRequest(input: InputStream): RequestParse? {
        val buf = ByteArray(MAX_HEADERS_LEN)
        var n = 0
        while (n < buf.size) {
            val r = input.read(buf, n, buf.size - n)
            if (r <= 0) return null
            n += r
            val s = String(buf, 0, n, StandardCharsets.US_ASCII)
            val end = s.indexOf("\r\n\r\n")
            if (end != -1) {
                val headerLen = end + 4
                val firstLine = s.lineSequence().firstOrNull() ?: return null
                val parts = firstLine.split(" ", limit = 3)
                if (parts.size < 3) return null
                val method = parts[0]
                val path = parts[1]
                var host: String? = null
                var rangeStart = 0L
                var rangeEnd = 0L
                for (line in s.lines().drop(1)) {
                    if (line.isEmpty()) break
                    val colon = line.indexOf(':')
                    if (colon <= 0) continue
                    val key = line.substring(0, colon).trim().lowercase()
                    val value = line.substring(colon + 1).trim()
                    when (key) {
                        "host" -> host = value
                        "range" -> parseRange(value)?.let { (a, b) -> rangeStart = a; rangeEnd = b }
                    }
                }
                if (host == null) return null
                return RequestParse(
                    requestBytes = buf.copyOf(n),
                    method = method,
                    host = host,
                    path = path,
                    rangeStart = rangeStart,
                    rangeEnd = rangeEnd,
                )
            }
        }
        return null
    }

    private data class RequestParse(
        val requestBytes: ByteArray,
        val method: String,
        val host: String,
        val path: String,
        val rangeStart: Long,
        val rangeEnd: Long,
    )

    private fun parseRange(s: String): Pair<Long, Long>? {
        val v = s.trim().removePrefix("bytes=") ?: return null
        val (a, b) = v.split("-", limit = 2).map { it.trim() }
        val start = a.toLongOrNull() ?: return null
        val end = if (b.isEmpty()) return null else b.toLongOrNull() ?: return null
        if (end < start) return null
        return start to end
    }

    private fun buildUrl(host: String, path: String): String {
        val pathStr = path.trim()
        return if (pathStr.startsWith("http://") || pathStr.startsWith("https://")) pathStr
        else "http://$host$pathStr"
    }

    private fun parseHostPort(host: String): Pair<String, Int> {
        val bracket = host.indexOf(']')
        val colon = host.indexOf(':', if (bracket >= 0) bracket else 0)
        return if (colon >= 0) {
            host.substring(0, colon) to host.substring(colon + 1).toIntOrNull() ?: 80
        } else {
            host to 80
        }
    }
}
