package app.wayform

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasClickAction
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

class VenueScreenTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private fun findCardiology() {
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Find a room or place").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Change venue").performClick()
        compose.onNode(hasText("Asterion University Medical Center") and hasClickAction()).performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Find a room or place").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Find a room or place").performTextClearance()
        compose.onNodeWithText("Find a room or place").performTextInput("cardiology")
        compose.onNodeWithText("Find a room or place").performImeAction()
        compose.onNodeWithTag("venue-list").performScrollToNode(hasText("Heart & Vascular Clinic"))
    }
    @Test fun searchAndVenueSelectionWorkOffline() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").assertIsDisplayed()
        compose.onNodeWithTag("venue-list").performScrollToNode(hasText("1 place · stored on this device"))
        compose.onNodeWithText("1 place · stored on this device").assertIsDisplayed()
    }
    @Test fun destinationOpensNativeFloorMapAndDirections() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithContentDescription("Avoid stairs").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithTag("route-ready").performScrollToNode(hasText("Preview only.", substring = true))
        compose.onNodeWithText("Preview only.", substring = true).assertIsDisplayed()
    }
    private fun startSampleJourney() {
        findCardiology()
        compose.onNodeWithText("Heart & Vascular Clinic").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithTag("route-ready").performScrollToNode(hasText("Venue conditions"))
        compose.onNodeWithText("Venue conditions").performClick()
        compose.onNodeWithText("All modeled paths open").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Simulated: open · 10 min").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithTag("route-loading").performScrollToNode(hasText("Confirm this starting place"))
        compose.onNodeWithText("Confirm this starting place").performClick()
        compose.onNodeWithText("Start sample journey").performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithTag("route-ready").fetchSemanticsNodes().isNotEmpty() }
    }
    @Test fun locationRecoveryClearsTheLiveJourney() {
        startSampleJourney()
        compose.onNodeWithTag("route-ready").performScrollToNode(hasText("I'm not here"))
        compose.onNodeWithText("I'm not here").performClick()
        compose.onNodeWithTag("route-loading").performScrollToNode(hasText("Start sample journey"))
        compose.onNodeWithText("Start sample journey").assertIsNotEnabled()
    }
    @Test fun backgroundingRequiresNewLocationConfirmation() {
        startSampleJourney()
        compose.activityRule.scenario.moveToState(Lifecycle.State.CREATED)
        compose.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        compose.onNodeWithTag("route-loading").performScrollToNode(hasText("Start sample journey"))
        compose.onNodeWithText("Start sample journey").assertIsNotEnabled()
    }
}
