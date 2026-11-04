package app.wayform

import java.io.ByteArrayOutputStream
import java.io.InputStream

fun InputStream.readBounded(limit: Int): ByteArray {
    val output = ByteArrayOutputStream()
    val buffer = ByteArray(8192)
    while (true) {
        val count = read(buffer, 0, minOf(buffer.size, limit + 1 - output.size()))
        if (count == -1) return output.toByteArray()
        output.write(buffer, 0, count)
        require(output.size() <= limit) { "The selected file is too large (limit ${limit / 1024} KiB)." }
    }
}
