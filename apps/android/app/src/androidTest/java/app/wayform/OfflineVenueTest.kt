package app.wayform

import androidx.test.platform.app.InstrumentationRegistry
import app.wayform.core.MobileVenue
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class OfflineVenueTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    @Test fun nativeBindingsReadBundledVenueAndRejectCorruption() {
        val bytes = context.assets.open("venues/asterion-medical-center.json").use { it.readBytes() }
        MobileVenue(bytes).use { venue ->
            val catalog = VenueCatalog(JSONObject(venue.catalog()))
            assertTrue(catalog.destinations.any { it.matches("cardiology") })
            val route = JSONObject(venue.preview(catalog.startId, "poi:poi-cardiology", true))
            assertTrue(route.getBoolean("found"))
            assertEquals(catalog.hash, route.getJSONObject("receipt").getString("packageHash"))
        }
        assertThrows(Exception::class.java) { MobileVenue("{}".toByteArray()).close() }
    }

    @Test fun packagesSurviveDatabaseReopenAndFailedImport() {
        VenueStore(context).use { store ->
            store.seed(context)
            val before = store.entries()
            assertEquals(4, before.size)
            assertThrows(Exception::class.java) { store.save("{}".toByteArray()) }
            assertEquals(before, store.entries())
        }
        VenueStore(context).use { store ->
            store.entries().forEach { entry ->
                MobileVenue(store.bytes(entry.id)).use { venue ->
                    assertEquals(entry.hash, JSONObject(venue.catalog()).getString("packageHash"))
                }
            }
        }
    }
}
