package app.wayform

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import kotlin.math.roundToInt

@Composable
fun RouteScreen(state: VenueState, model: VenueViewModel) {
    val catalog = requireNotNull(state.catalog)
    val destination = requireNotNull(state.destination)
    val floors = catalog.raw.getJSONArray("floors").objects()
    val start = catalog.destinations.firstOrNull { it.id == state.startId }
    var floorId by remember(catalog.id, state.startId) { mutableStateOf(start?.floorId ?: floors.first().getString("id")) }
    var chooseStart by remember { mutableStateOf(false) }
    var chooseConditions by remember { mutableStateOf(false) }
    var enterCode by remember { mutableStateOf(false) }
    var code by remember { mutableStateOf("") }
    val scanner = rememberLauncherForActivityResult(ScanContract()) { result -> result.contents?.let(model::checkIn) }
    val speechReady by model.speechReady.collectAsStateWithLifecycle()
    val route = state.route
    LaunchedEffect(state.active, state.stepIndex) {
        if (state.active) route?.getJSONArray("steps")?.getJSONObject(state.stepIndex)?.optString("floorId")?.takeIf { id -> floors.any { it.getString("id") == id } }?.let { floorId = it }
    }
    LazyColumn(Modifier.fillMaxSize().safeDrawingPadding().testTag(if (route != null) "route-ready" else "route-loading"), contentPadding = PaddingValues(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        item { TextButton(onClick = model::backToPlaces) { Text("Back to places") } }
        item {
            Text(if (state.active) "GUIDANCE · SAMPLE" else "ROUTE PREVIEW · SAMPLE", style = MaterialTheme.typography.labelMedium, color = Forest)
            Text(destination.name, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
            Text(destination.floorName, style = MaterialTheme.typography.bodyMedium)
        }
        if (!state.active) item {
            OutlinedButton(onClick = { chooseStart = true }, modifier = Modifier.fillMaxWidth(), enabled = !state.busy) { Text("From: ${state.checkInLabel ?: start?.name ?: "Check-in location"}") }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = { scanner.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE).setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false).setPrompt("Scan a venue check-in sign")) }, enabled = !state.busy) { Text("Scan QR sign") }
                TextButton(onClick = { code = ""; enterCode = true }, enabled = !state.busy) { Text("Enter code") }
            }
        }
        if (!state.active) item {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(Modifier.weight(1f)) {
                    Text("Avoid stairs", style = MaterialTheme.typography.titleMedium)
                    Text("Uses authored access flags. Widths, slopes and surfaces are unverified.", style = MaterialTheme.typography.bodySmall)
                }
                Switch(state.avoidStairs, { model.preview(destination, avoidStairs = it) }, enabled = !state.busy, modifier = Modifier.semantics { contentDescription = "Avoid stairs" })
            }
        }
        if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        state.error?.let { item { Text(it, color = MaterialTheme.colorScheme.error) } }
        state.notice?.let { item { Text(it, style = MaterialTheme.typography.bodyMedium) } }
        item {
            OutlinedButton(onClick = { chooseConditions = true }, enabled = !state.busy) { Text("Venue conditions") }
            Text(state.policyLabel ?: "No operating conditions loaded", style = MaterialTheme.typography.bodySmall)
        }
        if (!state.active) item {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = model::confirmStart, enabled = !state.busy && !state.confirmed, modifier = Modifier.fillMaxWidth()) { Text(if (state.confirmed) "Starting place confirmed" else "Confirm this starting place") }
                Button(onClick = model::startGuidance, enabled = state.confirmed && state.policyLabel != null && !state.busy, modifier = Modifier.fillMaxWidth()) { Text("Start sample journey") }
            }
        }
        if (state.active && route != null) item {
            Card(colors = CardDefaults.cardColors(containerColor = Lime), modifier = Modifier.fillMaxWidth()) {
                Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    val steps = route.getJSONArray("steps")
                    Text("Step ${state.stepIndex + 1} of ${steps.length()}", style = MaterialTheme.typography.labelLarge)
                    Text(steps.getJSONObject(state.stepIndex).getString("instruction"), style = MaterialTheme.typography.titleLarge)
                    Text("Advance each step manually. Your movement is not being tracked.", style = MaterialTheme.typography.bodySmall)
                    Button(onClick = model::nextInstruction) { Text(if (state.stepIndex + 1 == steps.length()) "Mark arrived" else "Next instruction") }
                    OutlinedButton(onClick = model::readInstruction, enabled = speechReady) { Text(if (speechReady) "Read instruction" else "Offline voice unavailable") }
                    OutlinedButton(onClick = { model.recover() }) { Text("I'm not here") }
                }
            }
        }
        item {
            LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(floors) { floor -> FilterChip(selected = floorId == floor.getString("id"), onClick = { floorId = floor.getString("id") }, label = { Text(floor.getString("name")) }) }
            }
            VenueMap(catalog, route, floorId)
        }
        if (route != null) {
            item { Text("${route.getDouble("totalDistance").roundToInt()} m · ${route.getJSONArray("steps").length()} instructions", style = MaterialTheme.typography.titleMedium) }
            if (!state.active) item { Text("Preview only. Your location is not being tracked and operating conditions have not been checked.", style = MaterialTheme.typography.bodyMedium) }
            items(route.getJSONArray("steps").objects()) { step ->
                ElevatedCard(Modifier.fillMaxWidth()) { Text(step.getString("instruction"), modifier = Modifier.padding(16.dp)) }
            }
        }
    }
    if (chooseStart) AlertDialog(onDismissRequest = { chooseStart = false }, title = { Text("Choose a starting place") }, text = {
        LazyColumn { items(catalog.destinations) { place -> TextButton(onClick = { chooseStart = false; model.preview(destination, start = place.id) }) { Text("${place.name} · ${place.floorName}") } } }
    }, confirmButton = { TextButton(onClick = { chooseStart = false }) { Text("Close") } })
    if (chooseConditions) AlertDialog(onDismissRequest = { chooseConditions = false }, title = { Text("Sample conditions") }, text = {
        Column {
            Text("These controls simulate changes in this fictional venue. They are not reports from an operator.")
            TextButton(onClick = { chooseConditions = false; model.sampleConditions(false) }) { Text("All modeled paths open") }
            TextButton(onClick = { chooseConditions = false; model.sampleConditions(true) }) { Text("All modeled lifts closed") }
            TextButton(onClick = { chooseConditions = false; model.sampleConditions(false, short = true) }) { Text("Open for 30 seconds") }
        }
    }, confirmButton = { TextButton(onClick = { chooseConditions = false }) { Text("Close") } })
    if (enterCode) AlertDialog(onDismissRequest = { enterCode = false }, title = { Text("Venue check-in code") }, text = {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Paste the full code from a venue sign. A recognized code still needs your location confirmation.")
            OutlinedTextField(code, { if (it.length <= 4096) code = it }, label = { Text("Check-in code") }, singleLine = true)
        }
    }, confirmButton = { TextButton(onClick = { enterCode = false; model.checkIn(code) }, enabled = code.isNotBlank()) { Text("Check code") } }, dismissButton = { TextButton(onClick = { enterCode = false }) { Text("Cancel") } })
}
