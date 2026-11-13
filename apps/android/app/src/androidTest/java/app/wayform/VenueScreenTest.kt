package app.wayform

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.isDialog
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextClearance
import org.junit.Rule
import org.junit.Test
import androidx.lifecycle.Lifecycle
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import androidx.lifecycle.ViewModelProvider
import android.net.Uri
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull

class VenueScreenTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun findCardiology() {
        val model = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
        InstrumentationRegistry.getInstrumentation().targetContext.getSharedPreferences("wayform", 0).edit().remove("widthCm").remove("slopePercent").remove("noSteps").commit()
        compose.waitUntil(30_000) { model.state.value.catalog != null && !model.state.value.busy }
        compose.onNodeWithContentDescription("Change venue").performClick()
        compose.onNode(hasText("Asterion University Medical Center") and hasClickAction() and hasAnyAncestor(isDialog())).performClick()
        compose.waitUntil(30_000) { model.state.value.catalog?.id == "asterion-medical-center" && !model.state.value.busy }
        compose.onNodeWithText("Find a room or place").performTextClearance()
        compose.onNodeWithText("Find a room or place").performTextInput("cardiology")
        compose.onNodeWithText("Find a room or place").performImeAction()
        compose.onNodeWithTag("venue-list").performScrollToNode(hasText("Heart & Vascular Clinic"))
    }
    @Test fun searchAndVenueSelectionWorkOffline() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").assertIsDisplayed()
        compose.onNodeWithTag("venue-list").performScrollToNode(hasText("Search results · 1"))
        compose.onNodeWithText("Search results · 1").assertIsDisplayed()
    }
    @Test fun destinationOpensNativeFloorMapAndDirections() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithContentDescription("Route options").performClick()
        compose.onNodeWithContentDescription("Avoid stairs").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithTag("route-ready").performScrollToNode(hasText("Show directions"))
        compose.onNodeWithText("Show directions").performClick()
        compose.onNodeWithText("Hide directions").assertIsDisplayed()
    }
    private fun startSampleJourney() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Start sample journey").performClick()
        compose.onNodeWithText("This demo simulates open paths for 10 minutes.").assertIsDisplayed()
        compose.onNodeWithText("I'm here · Start").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Next instruction").fetchSemanticsNodes().isNotEmpty() }
    }
    private fun assertConfirmationRequired() {
        compose.onNodeWithText("Next instruction").assertDoesNotExist()
        val model = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
        assertFalse(model.state.value.confirmed)
        assertFalse(model.state.value.active)
        compose.onNodeWithText("Start sample journey").performClick()
        compose.onNodeWithText("Confirm your starting place").assertIsDisplayed()
        compose.onNodeWithText("Cancel").performClick()
    }
    @Test fun locationRecoveryPlansFromTheSelectedPlaceAndKeepsTheDestination() {
        startSampleJourney()
        val model = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
        val old = model.state.value
        compose.onNodeWithText("I'm not here").performClick()
        compose.onNodeWithText("Where are you now?").assertIsDisplayed()
        compose.onNodeWithText("Close").performClick()
        compose.onNodeWithText("Start sample journey").assertDoesNotExist()
        compose.runOnIdle { model.confirmAndStart(simulateOpenPaths = false) }
        assertNull(model.state.value.startId)
        assertNull(model.state.value.route)
        assertFalse(model.state.value.active)
        compose.activityRule.scenario.recreate()
        compose.onNodeWithText("Choose current location").performClick()
        compose.onNodeWithText("Find a nearby place").performTextInput("Garden Café")
        compose.onNodeWithText("Garden Café · Ground · Diagnostics & Outpatients").performClick()
        compose.waitUntil(30_000) { model.state.value.route != null }
        assertConfirmationRequired()
        compose.onNodeWithText("Start sample journey").performClick()
        compose.onNodeWithText("Garden Café").assertIsDisplayed()
        compose.onNodeWithText("I'm here · Start").performClick()
        compose.waitUntil(30_000) { model.state.value.active }
        val resumed = model.state.value
        org.junit.Assert.assertEquals(old.destination?.id, resumed.destination?.id)
        org.junit.Assert.assertEquals(old.policyLabel, resumed.policyLabel)
        org.junit.Assert.assertNotEquals(old.startId, resumed.startId)
        org.junit.Assert.assertEquals(resumed.startId, resumed.route!!.getJSONArray("path").getJSONObject(0).getString("id"))
    }
    @Test fun markingArrivalEndsGuidanceAndReturnsToTheSearchResults() {
        startSampleJourney()
        val model = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
        val count = requireNotNull(model.state.value.route).getJSONArray("steps").length()
        repeat(count - 1) { compose.onNodeWithText("Next instruction").performClick() }
        compose.onNodeWithText("Mark arrived").performClick()
        compose.onNodeWithText("Arrival confirmed").assertIsDisplayed()
        assertFalse(model.state.value.active)
        compose.onNodeWithText("Back to places").performClick()
        compose.onNodeWithText("Heart & Vascular Clinic").assertIsDisplayed()
    }
    @Test fun confirmingTheStartingPlaceKeepsTheChosenVenueConditions() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").performClick()
        val model = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
        compose.waitUntil(30_000) { !model.state.value.busy }
        compose.onNodeWithContentDescription("Route options").performClick()
        compose.onNodeWithText("Venue conditions").performClick()
        compose.onNodeWithText("All modeled lifts closed").performClick()
        compose.waitUntil(30_000) { model.state.value.policyLabel?.contains("lifts closed") == true }
        compose.onNodeWithText("Start sample journey").performClick()
        compose.onNodeWithText("Simulated: all lifts closed · 10 min").assertIsDisplayed()
        compose.onNodeWithText("I'm here · Start").performClick()
        compose.waitUntil(30_000) { !model.state.value.busy }
        org.junit.Assert.assertEquals("Simulated: all lifts closed · 10 min", model.state.value.policyLabel)
    }
    @Test fun backgroundingRequiresNewLocationConfirmation() {
        startSampleJourney()
        compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
        compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        assertConfirmationRequired()
    }
    @Test fun recognizedCheckInStillRequiresConfirmationAndForeignCodesFail() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("From:", substring = true).performClick()
        compose.onNodeWithText("Enter code").performClick()
        compose.onNodeWithText("Check-in code").performTextInput("not-a-venue-code")
        compose.onNodeWithText("Check code").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithText("This code is not a recognized check-in", substring = true).fetchSemanticsNodes().isNotEmpty() }
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val packageJson = context.assets.open("venues/asterion-medical-center.json").use { JSONObject(it.readBytes().toString(Charsets.UTF_8)) }
        val payload = packageJson.getJSONArray("localizationAnchors").objects().first { it.getString("kind") == "qr" }.getString("payload")
        compose.onNodeWithText("Choose current location").performClick()
        compose.onNodeWithText("Enter code").performClick()
        compose.onNodeWithText("Check-in code").performTextInput(payload)
        compose.onNodeWithText("Check code").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Code recognized.", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("From: Central Concourse", substring = true).assertIsDisplayed()
        assertConfirmationRequired()
    }
    @Test fun hardRequirementsWithdrawGuidanceAndSurviveRecreation() {
        startSampleJourney()
        compose.onNodeWithContentDescription("Route options").performClick()
        compose.onNodeWithText("Mobility requirements").performClick()
        compose.onNodeWithText("Minimum clear width (cm)").performTextInput("90")
        compose.onNodeWithText("Save requirements").performClick()
        compose.onNodeWithTag("route-loading").performScrollToNode(hasText("No verified route:", substring = true))
        compose.onNodeWithText("No verified route:", substring = true).assertIsDisplayed()
        compose.onNodeWithText("Start sample journey").assertIsNotEnabled()
        compose.activityRule.scenario.recreate()
        compose.onNodeWithTag("route-loading").performScrollToNode(hasText("No verified route:", substring = true))
        compose.onNodeWithText("No verified route:", substring = true).assertIsDisplayed()
    }
    @Test fun invalidImportedConditionsWithdrawTheCurrentJourney() {
        startSampleJourney()
        lateinit var model: VenueViewModel
        compose.runOnIdle { model = ViewModelProvider(compose.activity)[VenueViewModel::class.java] }
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val file = java.io.File(context.cacheDir, "invalid-conditions.json")
        for (bytes in listOf("{\"schemaVersion\":\"0.1.0\",\"schemaVersion\":\"0.1.0\"}".toByteArray(), ByteArray(64 * 1024 + 1) { 32 })) {
            file.writeBytes(bytes)
            compose.runOnIdle { model.importConditions(Uri.fromFile(file)) }
            compose.waitUntil(30_000) { !model.state.value.busy && model.state.value.error != null }
            assertFalse(model.state.value.active)
            assertFalse(model.state.value.confirmed)
            assertNull(model.state.value.route)
            assertNull(model.state.value.policyLabel)
        }
        file.delete()
    }
    @Test fun switchingVenueWithdrawsGuidanceBeforeTheNewPackageLoads() {
        startSampleJourney()
        lateinit var model: VenueViewModel
        compose.runOnIdle {
            model = ViewModelProvider(compose.activity)[VenueViewModel::class.java]
            model.load("harbor-exchange")
            assertFalse(model.state.value.active)
            assertFalse(model.state.value.confirmed)
            assertNull(model.state.value.route)
        }
        compose.waitUntil(30_000) { !model.state.value.busy && model.state.value.catalog?.id == "harbor-exchange" }
    }
}
