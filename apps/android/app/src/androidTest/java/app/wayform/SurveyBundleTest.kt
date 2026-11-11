package app.wayform

import androidx.test.platform.app.InstrumentationRegistry
import java.time.Instant
import org.junit.Assert.*
import org.junit.Test

class SurveyBundleTest {
    private fun sampleBytes() = InstrumentationRegistry.getInstrumentation().targetContext.assets
        .open("survey/clinic-entrance.json").use { it.readBytes() }
    private fun checkWidth(bundle: SurveyBundle, cm: String, now: Instant = exampleTime) =
        bundle.assess("Plaza entrance", "Reception", MobilityNeeds(widthCm = cm), now)

    @Test fun widthChangesTheNativeRouteAndExplainsUnavailablePaths() {
        val bundle = SurveyBundle.open(sampleBytes(), example = true)
        assertEquals(12.0, checkWidth(bundle, "75").distanceMeters!!, 0.0)
        val detour = checkWidth(bundle, "90")
        assertEquals(18.0, detour.distanceMeters!!, 0.0)
        assertEquals(listOf(SurveyLeg("Plaza entrance", "Side lobby", "side-door"), SurveyLeg("Side lobby", "Reception", "lobby-corridor")), detour.legs)
        assertEquals(listOf("example-widths"), detour.evidence)
        assertEquals("infeasible", checkWidth(bundle, "120").outcome)
        val noSteps = bundle.assess(bundle.nodes.first(), bundle.nodes.last(), MobilityNeeds(noSteps = true))
        assertEquals("insufficient-evidence", noSteps.outcome)
        assertTrue(noSteps.legs.isEmpty())
        assertTrue(noSteps.reasons.isNotEmpty())
    }

    @Test fun importedBytesUseTheCheckTimeAndCannotSupplyTheirOwnClock() {
        val bundle = SurveyBundle.open(sampleBytes())
        assertEquals("routed", checkWidth(bundle, "90").outcome)
        val expired = checkWidth(bundle, "90", Instant.parse("2026-11-01T00:00:00Z"))
        assertEquals("insufficient-evidence", expired.outcome)
        assertTrue("stale-evidence" in expired.reasons)
        val duplicate = sampleBytes().decodeToString().replaceFirst("\"nodes\":", "\"nodes\": [], \"nodes\":")
        assertTrue(runCatching { SurveyBundle.open(duplicate.toByteArray()) }.isFailure)
        val ambiguous = sampleBytes().decodeToString().replaceFirst("\"revision\": \"1\"", "\"revision\": \"1\", \"revision\": \"2\"")
        assertTrue(runCatching { SurveyBundle.open(ambiguous.toByteArray()) }.isFailure)
        assertTrue(runCatching { SurveyBundle.open(ByteArray(surveyByteLimit + 1)) }.isFailure)
    }
}
