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
            compose.runOnIdle { venue.saveNeeds(MobilityNeeds(widthCm = "90")) }
            compose.onNodeWithText("Survey routes").performClick()
            compose.onNodeWithText("Try the clinic example").performClick()
            compose.waitUntil(30_000) { compose.onAllNodesWithText("Check route").fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithText("Check route").performScrollTo().performClick()
            fun scrollTo(text: String) = compose.onNodeWithTag("survey-review").performScrollToNode(hasText(text))
            scrollTo("Route meets selected requirements")
            compose.onNodeWithText("18 m · 2 segments").assertIsDisplayed()
            scrollTo("Mobility requirements")
            compose.onNodeWithText("Mobility requirements").performClick()
            compose.onNodeWithText("Minimum clear width (cm)").performTextReplacement("120")
            compose.onNodeWithText("Save requirements").performClick()
            compose.onNodeWithText("Route meets selected requirements").assertDoesNotExist()
            compose.onNodeWithText("Check route").performScrollTo().performClick()
            scrollTo("No eligible path")
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
