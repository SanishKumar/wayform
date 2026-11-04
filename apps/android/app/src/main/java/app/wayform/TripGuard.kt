package app.wayform

import android.os.SystemClock
import app.wayform.core.MobileTripInterface
import app.wayform.core.TripStopReason
import org.json.JSONObject

interface JourneyClock {
    fun wallMs(): Double
    fun monotonicMs(): Double
}
object DeviceJourneyClock : JourneyClock {
    override fun wallMs() = System.currentTimeMillis().toDouble()
    override fun monotonicMs() = SystemClock.elapsedRealtime().toDouble()
}

/** Never caches a ready result. The UI must discard its copy when this becomes unavailable. */
class TripGuard(private val trip: MobileTripInterface, private val clock: JourneyClock = DeviceJourneyClock) : AutoCloseable {
    private var stopped = false
    fun read(): JSONObject {
        check(!stopped) { "This journey has ended. Confirm your location to start again." }
        val result = JSONObject(trip.read(clock.wallMs(), clock.monotonicMs()))
        check(result.getString("status") == "ready") { "Guidance paused: ${result.optString("reason", "conditions unavailable")}. Confirm your location and check the venue conditions." }
        return result.getJSONObject("route")
    }
    fun delayMs(): Long = trip.nextCheckDelay(clock.wallMs(), clock.monotonicMs()).toLong().coerceIn(1, 1000)
    fun stop(reason: TripStopReason) {
        if (!stopped) {
            stopped = true
            try { trip.invalidate(reason) } finally { (trip as? AutoCloseable)?.close() }
        }
    }
    override fun close() = stop(TripStopReason.CANCELLED)
}
