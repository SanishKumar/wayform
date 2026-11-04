package app.wayform

import androidx.test.platform.app.InstrumentationRegistry
import app.wayform.core.MobileVenue
import app.wayform.core.TripStopReason
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class TripGuardTest {
    private class Clock(var wall: Double = 10_000.0, var mono: Double = 100.0) : JourneyClock {
        override fun wallMs() = wall
        override fun monotonicMs() = mono
    }
    private fun bytes() = InstrumentationRegistry.getInstrumentation().targetContext.assets.open("venues/asterion-medical-center.json").use { it.readBytes() }
    private fun catalog(venue: MobileVenue, bytes: ByteArray): VenueCatalog {
        val lifts = JSONObject(bytes.toString(Charsets.UTF_8)).getJSONArray("verticalConnectors").objects().filter { it.getString("kind") == "elevator" }.map { it.getString("id") }
        return VenueCatalog(JSONObject(venue.catalog()), lifts)
    }

    @Test fun expiryAndClockRollbackWithdrawAllGuidance() {
        val bytes = bytes()
        MobileVenue(bytes).use { venue ->
            val catalog = catalog(venue, bytes)
            for (expiredWall in listOf(11_000.0, 8_000.0)) {
                val clock = Clock()
                val policy = SampleConditions.create(catalog, false, 1000, 10_000)
                TripGuard(venue.startTrip(catalog.startId, "poi:poi-cardiology", true, policy, clock.wall, clock.mono), clock).use { guard ->
                    assertTrue(guard.read().getBoolean("found"))
                    clock.wall = expiredWall; clock.mono = 1100.0
                    assertThrows(IllegalStateException::class.java) { guard.read() }
                    clock.wall = 10_001.0; clock.mono = 101.0
                    assertThrows(IllegalStateException::class.java) { guard.read() }
                }
            }
        }
    }
    @Test fun explicitInvalidationAndClosedLiftsCannotExposeAStepFreeTrip() {
        val bytes = bytes()
        MobileVenue(bytes).use { venue ->
            val catalog = catalog(venue, bytes)
            val policy = SampleConditions.create(catalog, false, 1000, 10_000)
            val guard = TripGuard(venue.startTrip(catalog.startId,"poi:poi-cardiology",true,policy,10_000.0,100.0),Clock())
            guard.stop(TripStopReason.CONTEXT_CHANGED)
            assertThrows(IllegalStateException::class.java) { guard.read() }
            val closed = SampleConditions.create(catalog, true, 1000, 10_000)
            assertThrows(Exception::class.java) { venue.startTrip(catalog.startId,"poi:poi-cardiology",true,closed,10_000.0,100.0).close() }
        }
    }
}
