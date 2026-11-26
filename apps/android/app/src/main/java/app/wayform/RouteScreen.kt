package app.wayform

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import org.json.JSONObject
import kotlin.math.roundToInt

private enum class RoutePanel { START, ORIGIN, OPTIONS, CONDITIONS, CODE, NEEDS }

@Composable
fun RouteScreen(state: VenueState, model: VenueViewModel) {
    val catalog = requireNotNull(state.catalog)
    val destination = requireNotNull(state.destination)
    if (state.arrived) {
        Column(Modifier.fillMaxSize().safeDrawingPadding().verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text("Arrival confirmed", style = MaterialTheme.typography.labelLarge, color = Lime)
            Text(destination.name, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
            Text(destination.floorName, color = Muted)
            Button(onClick = model::backToPlaces) { Text("Back to places") }
        }
        return
    }
    val floors = remember(catalog) { catalog.raw.getJSONArray("floors").objects() }
    val route = state.route
    val steps = remember(route) { route?.getJSONArray("steps")?.objects().orEmpty() }
    val currentStep = steps.getOrNull(state.stepIndex).takeIf { state.active }
    val stepNodeId = currentStep?.getString("nodeId")
    val start = catalog.destinations.firstOrNull { it.id == state.startId }
    val origin = state.checkInLabel ?: start?.name ?: "Check-in location"
    var floorId by remember(catalog.id, state.startId) { mutableStateOf(start?.floorId ?: floors.first().getString("id")) }
    var mapMode by rememberSaveable { mutableStateOf(MapViewMode.DEPTH) }
    var panel by remember { mutableStateOf<RoutePanel?>(null) }
    var showSteps by rememberSaveable(destination.id, state.active) { mutableStateOf(false) }
    var code by remember { mutableStateOf("") }
    val scanner = rememberLauncherForActivityResult(ScanContract()) { result -> result.contents?.let(model::checkIn) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(model::importConditions) }
    val speechReady by model.speechReady.collectAsStateWithLifecycle()
    LaunchedEffect(currentStep, route) {
        val path = route?.optJSONArray("path")?.objects().orEmpty()
        val floor = if (currentStep == null) path.firstOrNull()?.getString("floor")
            else path.firstOrNull { it.getString("id") == stepNodeId }?.getString("floor")
                ?: currentStep.getString("floorId")
        if (floors.any { it.getString("id") == floor }) floorId = requireNotNull(floor)
    }

    Column(Modifier.fillMaxSize().safeDrawingPadding(), horizontalAlignment = Alignment.CenterHorizontally) {
        Row(Modifier.widthIn(max = 680.dp).fillMaxWidth().padding(horizontal = 14.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            FilledTonalIconButton(onClick = model::backToPlaces) { WayIcon(R.drawable.ic_back, "Back to places") }
            Column(Modifier.weight(1f)) {
                Text(if (state.active) "Sample journey" else "Route preview", style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold)
                Text(catalog.name, style = MaterialTheme.typography.labelMedium, color = Muted, maxLines = 1,
                    overflow = TextOverflow.Ellipsis)
            }
            FilledTonalIconButton(onClick = { panel = RoutePanel.OPTIONS }, enabled = !state.busy) {
                WayIcon(R.drawable.ic_options, "Route options")
            }
        }

        BoxWithConstraints(Modifier.weight(1f).widthIn(max = 680.dp).fillMaxWidth()) {
            val mapShare = when {
                state.error != null || state.needs.hasRequirements || state.notice != null -> 0.43f
                LocalDensity.current.fontScale > 1.3f || showSteps -> 0.49f
                else -> 0.64f
            }
            val mapHeight = (maxHeight * mapShare).coerceAtMost(520.dp)
            val sideBySide = maxWidth >= 600.dp && maxHeight < 540.dp
            val map: @Composable (Modifier) -> Unit = { modifier ->
                RouteMapStage(catalog, route, floors, floorId, mapMode, stepNodeId, modifier,
                    onFloor = { floorId = it }, onMode = { mapMode = it })
            }
            val details: @Composable (Modifier) -> Unit = { modifier ->
                RouteDetails(state, destination, origin, currentStep, steps, showSteps, modifier,
                    onOrigin = { panel = RoutePanel.ORIGIN }, onToggleSteps = { showSteps = !showSteps })
            }
            if (sideBySide) {
                Row(Modifier.fillMaxSize().padding(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    map(Modifier.weight(1f).fillMaxHeight())
                    details(Modifier.weight(1f).fillMaxHeight())
                }
            } else {
                Column(Modifier.fillMaxSize().padding(horizontal = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    map(Modifier.fillMaxWidth().height(mapHeight))
                    details(Modifier.weight(1f).fillMaxWidth())
                }
            }
        }
        Surface(Modifier.widthIn(max = 680.dp).fillMaxWidth(), color = Paper) {
            Column(Modifier.padding(horizontal = 16.dp, vertical = 6.dp)) {
                if (state.active) {
                    Button(onClick = model::nextInstruction, modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp)) {
                        Text(if (state.stepIndex == steps.lastIndex) "Mark arrived" else "Next instruction")
                    }
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        TextButton(onClick = model::readInstruction, enabled = speechReady) {
                            Text(if (speechReady) "Read instruction" else "Offline voice unavailable")
                        }
                        TextButton(onClick = { model.lostLocation(); panel = RoutePanel.ORIGIN }) { Text("I'm not here") }
                    }
                } else if (state.startId == null) {
                    Button(onClick = { panel = RoutePanel.ORIGIN }, enabled = !state.busy,
                        modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp)) { Text("Choose current location") }
                } else {
                    Button(onClick = { panel = RoutePanel.START }, enabled = !state.busy && !state.needs.hasRequirements,
                        modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp)) { Text("Start sample journey") }
                }
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
        RoutePanel.ORIGIN -> OriginPicker(catalog, onDismiss = dismiss,
            onSelect = { place -> dismiss(); model.preview(destination, start = place.id) },
            onScan = {
                dismiss()
                scanner.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                    .setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false).setPrompt("Scan a venue check-in sign"))
            },
            onEnterCode = { code = ""; panel = RoutePanel.CODE })
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

@Composable
private fun RouteMapStage(
    catalog: VenueCatalog,
    route: JSONObject?,
    floors: List<JSONObject>,
    floorId: String,
    mode: MapViewMode,
    stepNodeId: String?,
    modifier: Modifier,
    onFloor: (String) -> Unit,
    onMode: (MapViewMode) -> Unit,
) {
    val shape = MaterialTheme.shapes.large
    Column(modifier.clip(shape).background(Paper).border(1.dp, MaterialTheme.colorScheme.outlineVariant, shape)) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Sample map", Modifier.weight(1f), color = Muted, style = MaterialTheme.typography.labelMedium)
            FilterChip(selected = mode == MapViewMode.DEPTH, onClick = { onMode(MapViewMode.DEPTH) },
                label = { Text("3D") }, modifier = Modifier.semantics { contentDescription = "3D cutaway" })
            Spacer(Modifier.width(6.dp))
            FilterChip(selected = mode == MapViewMode.PLAN, onClick = { onMode(MapViewMode.PLAN) },
                label = { Text("2D") }, modifier = Modifier.semantics { contentDescription = "2D floor plan" })
        }
        VenueMap(catalog, route, floorId, Modifier.weight(1f).fillMaxWidth(), mode = mode, stepNodeId = stepNodeId)
        LazyRow(contentPadding = PaddingValues(horizontal = 10.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            items(floors, key = { it.getString("id") }) { floor ->
                val id = floor.getString("id")
                val name = floor.getString("name")
                FilterChip(selected = floorId == id, onClick = { onFloor(id) },
                    label = { Text(name.substringBefore(" · ")) },
                    modifier = Modifier.semantics { contentDescription = name })
            }
        }
    }
}

@Composable
private fun RouteDetails(
    state: VenueState,
    destination: Destination,
    origin: String,
    step: JSONObject?,
    steps: List<JSONObject>,
    showSteps: Boolean,
    modifier: Modifier,
    onOrigin: () -> Unit,
    onToggleSteps: () -> Unit,
) {
    val listState = rememberLazyListState()
    LaunchedEffect(state.active, state.stepIndex) { listState.scrollToItem(0) }
    Surface(modifier, color = MaterialTheme.colorScheme.surface, shape = MaterialTheme.shapes.large) {
        LazyColumn(
            Modifier.fillMaxSize().testTag(if (state.route != null) "route-ready" else "route-loading"),
            state = listState,
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (step != null) item {
                DirectionCard(step, "Step ${state.stepIndex + 1} of ${steps.size}", destination.name)
            } else item {
                Text(destination.name, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
                Text(destination.floorName, color = Muted, style = MaterialTheme.typography.bodyMedium)
                if (state.startId != null) {
                    TextButton(onClick = onOrigin, enabled = !state.busy, contentPadding = PaddingValues(horizontal = 0.dp)) {
                        WayIcon(R.drawable.ic_pin)
                        Spacer(Modifier.width(5.dp))
                        Text("From: $origin")
                    }
                }
            }
            if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
            if (state.needs.hasRequirements) item { Text(state.needs.missingEvidence(), color = MaterialTheme.colorScheme.error) }
            state.error?.let { item { Text(it, color = MaterialTheme.colorScheme.error) } }
            state.notice?.let { item { Text(it, color = Lime) } }
            if (state.route != null && !state.active) item {
                HorizontalDivider()
                Row(Modifier.fillMaxWidth().padding(top = 4.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text("${state.route.getDouble("totalDistance").roundToInt()} m · ${steps.size} instructions",
                        Modifier.weight(1f), style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.SemiBold)
                    TextButton(onClick = onToggleSteps) { Text(if (showSteps) "Hide directions" else "Show directions") }
                }
            }
            if (showSteps && !state.active) items(steps) { segment ->
                Text(segment.getString("instruction"), Modifier.padding(vertical = 4.dp))
                HorizontalDivider()
            }
            if (state.active) state.policyLabel?.let { item {
                Text(it, color = Muted, style = MaterialTheme.typography.bodySmall)
            } }
        }
    }
}

@Composable
private fun DirectionCard(step: JSONObject, progress: String, destination: String) {
    val type = step.getString("type")
    val angle = when (type) {
        "turn_left" -> -90f
        "slight_left" -> -45f
        "slight_right" -> 45f
        "turn_right" -> 90f
        "u_turn" -> 180f
        else -> 0f
    }
    val icon = when (type) {
        "elevator", "escalator", "stairs", "ramp" -> R.drawable.ic_level_change
        "arrive" -> R.drawable.ic_pin
        else -> R.drawable.ic_direction
    }
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Box(Modifier.size(46.dp).clip(MaterialTheme.shapes.small)
            .background(MaterialTheme.colorScheme.primaryContainer), contentAlignment = Alignment.Center) {
            Box(Modifier.rotate(angle)) { WayIcon(icon, tint = MaterialTheme.colorScheme.onPrimaryContainer) }
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(progress, style = MaterialTheme.typography.labelLarge, color = Lime)
            Text(step.getString("instruction"), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
            Text("To $destination", style = MaterialTheme.typography.bodyMedium, color = Muted)
        }
    }
}

@Composable
private fun OriginPicker(catalog: VenueCatalog, onDismiss: () -> Unit, onSelect: (Destination) -> Unit,
    onScan: () -> Unit, onEnterCode: () -> Unit) {
    var query by rememberSaveable { mutableStateOf("") }
    val matches = catalog.destinations.filter { it.matches(query) }
    AlertDialog(onDismissRequest = onDismiss, title = { Text("Where are you now?") }, text = {
        Column {
            OutlinedTextField(query, { query = it }, modifier = Modifier.fillMaxWidth(),
                singleLine = true, label = { Text("Find a nearby place") })
            LazyColumn {
                if (query.isBlank()) item {
                    TextButton(onClick = onScan) { Text("Scan QR sign") }
                    TextButton(onClick = onEnterCode) { Text("Enter code") }
                    HorizontalDivider()
                }
                items(matches, key = { it.id }) { place ->
                    TextButton(onClick = { onSelect(place) }) { Text("${place.name} · ${place.floorName}") }
                }
                if (matches.isEmpty()) item { Text("No matching places. Try a room name or floor.") }
            }
        }
    }, confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } })
}
