package app.wayform

import android.net.Uri
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class SurveyScreenTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun clinicExampleRechecksWhenTheWidthChanges() {
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Find a room or place").fetchSemanticsNodes().isNotEmpty() }
        val venue = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
        val initial = venue.state.value.needs
        try {
            val saved = MobilityNeeds(widthCm = "83", noSteps = true)
            compose.runOnIdle { venue.saveNeeds(saved) }
            compose.onNodeWithText("Survey routes").performClick()
            compose.onNodeWithText("Try the clinic example").performClick()
            fun expect(text: String) {
                compose.waitUntil(30_000) {
                    ViewModelProvider(compose.activity)[SurveyViewModel::class.java].state.value.assessment != null
                }
                compose.onNodeWithTag("survey-review").performScrollToNode(hasText(text))
                compose.onNodeWithText(text).assertIsDisplayed()
            }
            expect("18 m · 2 segments")
            compose.onNodeWithTag("survey-review").performScrollToNode(hasText("75 cm"))
            compose.onNodeWithText("75 cm").performClick()
            expect("12 m · 1 segment")
            compose.onNodeWithTag("survey-review").performScrollToNode(hasText("120 cm"))
            compose.onNodeWithText("120 cm").performClick()
            expect("No eligible path")
            assertEquals(saved, venue.state.value.needs)
        } finally {
            compose.runOnIdle { venue.saveNeeds(initial) }
        }
    }

    @Test fun replacingAFileDropsThePreviousAssessmentEvenOnFailure() {
        val model = ViewModelProvider(compose.activity)[SurveyViewModel::class.java]
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val file = File(context.cacheDir, "survey-import-test.json")
        try {
            file.writeBytes(context.assets.open("survey/clinic-entrance.json").use { it.readBytes() })
            compose.runOnIdle { model.open(Uri.fromFile(file)) }
            compose.waitUntil(30_000) { model.state.value.bundle != null }
            assertFalse(model.state.value.bundle!!.example)
            compose.runOnIdle { model.check(MobilityNeeds(widthCm = "90")) }
            compose.waitUntil(30_000) { model.state.value.assessment != null }
            file.writeText("{}")
            compose.runOnIdle {
                model.open(Uri.fromFile(file))
                assertNull(model.state.value.assessment)
            }
            compose.waitUntil(30_000) { model.state.value.error != null }
            assertNull(model.state.value.bundle)
            assertNull(model.state.value.assessment)
        } finally {
            file.delete()
        }
    }
}
