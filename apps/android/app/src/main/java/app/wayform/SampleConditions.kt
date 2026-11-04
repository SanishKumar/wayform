package app.wayform

import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import java.time.format.DateTimeFormatterBuilder
import java.util.Locale
import java.util.UUID

/** Explicit simulations for the bundled synthetic venues, never operator observations. */
object SampleConditions {
    fun create(catalog: VenueCatalog, closeLifts: Boolean, lifetimeMs: Long, nowMs: Long = System.currentTimeMillis()): ByteArray {
        val format = DateTimeFormatterBuilder().appendInstant(3).toFormatter(Locale.ROOT)
        fun timestamp(ms: Long) = format.format(Instant.ofEpochMilli(ms))
        val closures = JSONArray()
        if (closeLifts) catalog.liftIds.forEach { id -> closures.put(JSONObject().put("id", "sample-close-$id").put("target", JSONObject().put("type", "source").put("id", id)).put("reason", "Simulated lift closure")) }
        return JSONObject().put("schemaVersion", "0.1.0").put("id", "sample-${UUID.randomUUID()}")
            .put("buildingId", catalog.id).put("packageHash", catalog.hash)
            .put("validFrom", timestamp(nowMs - 1000)).put("validUntil", timestamp(nowMs + lifetimeMs))
            .put("closures", closures).toString().toByteArray()
    }
}
