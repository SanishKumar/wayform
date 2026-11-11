package app.wayform

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import kotlin.math.roundToInt

private enum class RoutePanel { START, ORIGIN, OPTIONS, CONDITIONS, CODE, NEEDS }

@Composable
fun RouteScreen(state: VenueState, model: VenueViewModel) {
    val catalog = requireNotNull(state.catalog)
    val destination = requireNotNull(state.destination)
    if (state.arrived) {
        Column(Modifier.fillMaxSize().safeDrawingPadding().verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Arrival confirmed", style = MaterialTheme.typography.labelLarge, color = Forest)
            Text(destination.name, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
            Text(destination.floorName, color = Muted)
            Button(onClick = model::backToPlaces) { Text("Back to places") }
        }
        return
    }
    val floors = remember(catalog) { catalog.raw.getJSONArray("floors").objects() }
    val route = state.route
    val steps = remember(route) { route?.getJSONArray("steps")?.objects().orEmpty() }
    val currentStep = steps.getOrNull(state.stepIndex)
    val start = catalog.destinations.firstOrNull { it.id == state.startId }
    val origin = state.checkInLabel ?: start?.name ?: "Check-in location"
    var floorId by remember(catalog.id, state.startId) { mutableStateOf(start?.floorId ?: floors.first().getString("id")) }
    var panel by remember { mutableStateOf<RoutePanel?>(null) }
    var showSteps by rememberSaveable(destination.id, state.active) { mutableStateOf(false) }
    var code by remember { mutableStateOf("") }
    val scanner = rememberLauncherForActivityResult(ScanContract()) { result -> result.contents?.let(model::checkIn) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(model::importConditions) }
    val speechReady by model.speechReady.collectAsStateWithLifecycle()
    LaunchedEffect(state.active, state.stepIndex, route) {
        val floor = if (state.active) currentStep?.optString("floorId")
            else route?.optJSONArray("path")?.optJSONObject(0)?.optString("floor")
        if (floors.any { it.getString("id") == floor }) floorId = requireNotNull(floor)
    }

    Column(Modifier.fillMaxSize().safeDrawingPadding()) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = model::backToPlaces) { WayIcon(R.drawable.ic_back, "Back to places") }
            Text(if (state.active) "Sample journey" else "Route preview", Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
            IconButton(onClick = { panel = RoutePanel.OPTIONS }, enabled = !state.busy) {
                WayIcon(R.drawable.ic_options, "Route options")
            }
        }
        key(state.active, destination.id) {
            val listState = rememberLazyListState()
            LaunchedEffect(state.stepIndex) {
                if (state.active) listState.scrollToItem(0)
            }
            LazyColumn(Modifier.weight(1f).fillMaxWidth().testTag(if (route != null) "route-ready" else "route-loading"),
                state = listState, contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                if (state.active && currentStep != null) item {
                    Surface(color = Forest, contentColor = Color.White, shape = MaterialTheme.shapes.large) {
                        Column(Modifier.fillMaxWidth().padding(18.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text("Step ${state.stepIndex + 1} of ${steps.size}", color = Lime, style = MaterialTheme.typography.labelLarge)
                            Text(currentStep.getString("instruction"), style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
                            Text("To ${destination.name}", style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                } else item {
                    Text(destination.name, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                    Text(destination.floorName, color = Muted)
                    TextButton(onClick = { panel = RoutePanel.ORIGIN }, enabled = !state.busy) {
                        WayIcon(R.drawable.ic_pin)
                        Spacer(Modifier.width(6.dp))
                        Text("From: $origin")
                    }
                }
                if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
                if (state.needs.hasRequirements) item { Text(state.needs.missingEvidence(), color = MaterialTheme.colorScheme.error) }
                state.error?.let { item { Text(it, color = MaterialTheme.colorScheme.error) } }
                state.notice?.let { item { Text(it, color = Forest) } }
                item {
                    Surface(color = Color.White, shape = MaterialTheme.shapes.large) {
                        Column(Modifier.fillMaxWidth()) {
                            LazyRow(contentPadding = PaddingValues(horizontal = 10.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                items(floors) { floor ->
                                    val name = floor.getString("name")
                                    FilterChip(selected = floorId == floor.getString("id"), onClick = { floorId = floor.getString("id") },
                                        label = { Text(name.substringBefore(" · ")) }, modifier = Modifier.semantics { contentDescription = name })
                                }
                            }
                            VenueMap(catalog, route, floorId)
                        }
                    }
                }
                if (route != null && !state.active) item {
                    Text("${route.getDouble("totalDistance").roundToInt()} m · ${steps.size} instructions",
                        style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                    Text("Sample map · Preview only", color = Muted)
                    TextButton(onClick = { showSteps = !showSteps }) { Text(if (showSteps) "Hide directions" else "Show directions") }
                }
                if (showSteps && !state.active) items(steps) { Text(it.getString("instruction")); HorizontalDivider() }
                if (state.active) item {
                    Text(state.policyLabel.orEmpty(), color = Muted, style = MaterialTheme.typography.bodySmall)
                    TextButton(onClick = model::readInstruction, enabled = speechReady) {
                        Text(if (speechReady) "Read instruction" else "Offline voice unavailable")
                    }
                }
            }
        }
        HorizontalDivider()
        Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp)) {
            if (state.active) {
                TextButton(onClick = { model.recover() }, modifier = Modifier.align(Alignment.CenterHorizontally)) { Text("I'm not here") }
                Button(onClick = model::nextInstruction, modifier = Modifier.fillMaxWidth()) {
                    Text(if (state.stepIndex == steps.lastIndex) "Mark arrived" else "Next instruction")
                }
            } else {
                Button(onClick = { panel = RoutePanel.START }, enabled = !state.busy && !state.needs.hasRequirements,
                    modifier = Modifier.fillMaxWidth()) { Text("Start sample journey") }
            }
        }
    }
    val dismiss = { panel = null }
    when (panel) {
        RoutePanel.START -> AlertDialog(onDismissRequest = dismiss, title = { Text("Confirm your starting place") }, text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(origin, style = MaterialTheme.typography.titleLarge)
                Text(state.policyLabel ?: "This demo simulates open paths for 10 minutes.")
                Text("Advance directions manually from this location.")
                TextButton(onClick = { panel = RoutePanel.ORIGIN }) { Text("Change starting place") }
                TextButton(onClick = { panel = RoutePanel.CONDITIONS }) { Text("Venue conditions") }
            }
        }, confirmButton = { Button(onClick = { dismiss(); model.confirmAndStart(simulateOpenPaths = state.policyLabel == null) }) { Text("I'm here · Start") } },
            dismissButton = { TextButton(onClick = dismiss) { Text("Cancel") } })
        RoutePanel.ORIGIN -> AlertDialog(onDismissRequest = dismiss, title = { Text("Starting place") }, text = {
            LazyColumn {
                item { TextButton(onClick = { dismiss(); scanner.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                    .setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false).setPrompt("Scan a venue check-in sign")) }) { Text("Scan QR sign") } }
                item { TextButton(onClick = { code = ""; panel = RoutePanel.CODE }) { Text("Enter code") }; HorizontalDivider() }
                items(catalog.destinations) { place -> TextButton(onClick = { dismiss(); model.preview(destination, start = place.id) }) {
                    Text("${place.name} · ${place.floorName}")
                } }
            }
        }, confirmButton = { TextButton(onClick = dismiss) { Text("Close") } })
        RoutePanel.OPTIONS -> AlertDialog(onDismissRequest = dismiss, title = { Text("Route options") }, text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                if (!state.active) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("Avoid stairs", Modifier.weight(1f))
                        Switch(state.avoidStairs, { dismiss(); model.preview(destination, avoidStairs = it) },
                            modifier = Modifier.semantics { contentDescription = "Avoid stairs" })
                    }
                    Text("Uses the venue’s authored access flags.", style = MaterialTheme.typography.bodySmall)
                }
                TextButton(onClick = { panel = RoutePanel.NEEDS }) { Text("Mobility requirements") }
                TextButton(onClick = { panel = RoutePanel.CONDITIONS }) { Text("Venue conditions") }
                state.policyLabel?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
            }
        }, confirmButton = { TextButton(onClick = dismiss) { Text("Done") } })
        RoutePanel.CONDITIONS -> AlertDialog(onDismissRequest = dismiss, title = { Text("Venue conditions") }, text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                Text("Choose a simulation for this sample venue or open a conditions file.")
                TextButton(onClick = { dismiss(); model.sampleConditions(false) }) { Text("All modeled paths open") }
                TextButton(onClick = { dismiss(); model.sampleConditions(true) }) { Text("All modeled lifts closed") }
                TextButton(onClick = { dismiss(); model.sampleConditions(false, short = true) }) { Text("Open for 30 seconds") }
                TextButton(onClick = { dismiss(); picker.launch(arrayOf("application/json", "text/plain", "application/octet-stream")) }) { Text("Import conditions file") }
            }
        }, confirmButton = { TextButton(onClick = dismiss) { Text("Close") } })
        RoutePanel.CODE -> AlertDialog(onDismissRequest = dismiss, title = { Text("Venue check-in code") }, text = {
            OutlinedTextField(code, { if (it.length <= 4096) code = it }, label = { Text("Check-in code") }, singleLine = true)
        }, confirmButton = { TextButton(onClick = { dismiss(); model.checkIn(code) }, enabled = code.isNotBlank()) { Text("Check code") } },
            dismissButton = { TextButton(onClick = dismiss) { Text("Cancel") } })
        RoutePanel.NEEDS -> MobilityNeedsDialog(state.needs, onSave = { model.saveNeeds(it); dismiss() }, onDismiss = dismiss)
        null -> Unit
    }
}
