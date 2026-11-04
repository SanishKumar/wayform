package app.wayform

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import org.junit.Rule
import org.junit.Test

class VenueScreenTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun searchAndVenueSelectionWorkOffline() {
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Find a room or place").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Change venue").performClick()
        compose.onNode(hasText("Asterion University Medical Center") and hasClickAction()).performClick()
        compose.waitUntil(30_000) { compose.onAllNodesWithText("Find a room or place").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Find a room or place").performTextInput("cardiology")
        compose.onNodeWithText("Heart & Vascular Clinic").assertIsDisplayed()
        compose.onNodeWithText("1 place · stored on this device").assertIsDisplayed()
    }
}
