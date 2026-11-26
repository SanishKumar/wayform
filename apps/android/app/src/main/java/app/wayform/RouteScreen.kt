package app.wayform

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
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

    val density = LocalDensity.current
    var headerHeight by remember { mutableIntStateOf(0) }
    var dockHeight by remember { mutableIntStateOf(0) }
    var dockWidth by remember { mutableIntStateOf(0) }
    BoxWithConstraints(Modifier.fillMaxSize().background(Paper)) {
        val wide = maxWidth >= 600.dp && maxHeight < 540.dp
        val dockMaxWidth = if (wide) (maxWidth * .48f).coerceAtMost(480.dp) else 500.dp
        val detailsHeight = maxHeight * when {
            wide -> .44f
            showSteps -> .44f
            density.fontScale > 1.3f -> .40f
            else -> .32f
        }
        val framing = with(density) {
            PaddingValues(
                top = headerHeight.toDp(),
                end = if (wide) dockWidth.toDp() else 0.dp,
                bottom = if (!wide) dockHeight.toDp() else WindowInsets.safeDrawing.getBottom(this).toDp(),
            )
        }
        VenueMap(catalog, route, floorId, Modifier.matchParentSize(), mode = mapMode, stepNodeId = stepNodeId,
            framing = framing, focusRoute = true)

        RouteHeader(catalog.name, state.active,
            Modifier.align(Alignment.TopCenter).widthIn(max = 680.dp).fillMaxWidth()
                .onSizeChanged { headerHeight = it.height }
                .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
                .padding(horizontal = 16.dp, vertical = 8.dp),
            canOpenOptions = !state.busy, onBack = model::backToPlaces, onOptions = { panel = RoutePanel.OPTIONS })

        Column(
            Modifier.align(if (wide) Alignment.BottomEnd else Alignment.BottomCenter)
                .widthIn(max = dockMaxWidth).fillMaxWidth()
                .onSizeChanged { dockHeight = it.height; dockWidth = it.width }
                .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Bottom + WindowInsetsSides.Horizontal))
                .padding(horizontal = 16.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FloorSwitch(floors, floorId, Modifier.weight(1f), onFloor = { floorId = it })
                ModeSwitch(mapMode, onMode = { mapMode = it })
            }

            RouteDetails(state, destination, origin, currentStep, steps, showSteps, Modifier.heightIn(max = detailsHeight),
                onOrigin = { panel = RoutePanel.ORIGIN })

            when {
                state.active -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    RouteControl(
                        text = if (speechReady) "Listen" else "Voice unavailable",
                        icon = R.drawable.ic_audio,
                        modifier = Modifier.weight(1f),
                        enabled = speechReady,
                        description = if (speechReady) "Read instruction" else "Offline voice unavailable",
                        onClick = model::readInstruction,
                    )
                    RouteControl("I'm not here", R.drawable.ic_pin, Modifier.weight(1f), onClick = {
                        model.lostLocation()
                        panel = RoutePanel.ORIGIN
                    })
                    val last = state.stepIndex == steps.lastIndex
                    RouteAction(if (last) "Arrived" else "Next", if (last) "Mark arrived" else "Next instruction",
                        onClick = model::nextInstruction)
                }
                state.startId == null -> Button(
                    onClick = { panel = RoutePanel.ORIGIN }, enabled = !state.busy,
                    modifier = Modifier.fillMaxWidth().heightIn(min = 54.dp), shape = MaterialTheme.shapes.medium,
                ) { Text("Choose current location") }
                else -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (route != null) RouteSummary(route.getDouble("totalDistance").roundToInt(), steps.size,
                        showSteps, Modifier.weight(1f), onClick = { showSteps = !showSteps })
                    else Spacer(Modifier.weight(1f))
                    RouteAction("Start", "Start sample journey", enabled = !state.busy && !state.needs.hasRequirements,
                        onClick = { panel = RoutePanel.START })
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
private fun RouteHeader(
    venue: String,
    active: Boolean,
    modifier: Modifier,
    canOpenOptions: Boolean,
    onBack: () -> Unit,
    onOptions: () -> Unit,
) {
    Row(modifier, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilledTonalIconButton(
            onClick = onBack, modifier = Modifier.size(50.dp), shape = MaterialTheme.shapes.medium,
            colors = IconButtonDefaults.filledTonalIconButtonColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        ) { WayIcon(R.drawable.ic_back, "Back to places") }
        Surface(Modifier.weight(1f), shape = MaterialTheme.shapes.medium,
            color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = .94f)) {
            Column(Modifier.padding(horizontal = 14.dp, vertical = 8.dp)) {
                Text(if (active) "Sample journey" else "Route preview", style = MaterialTheme.typography.titleSmall)
                Text(venue, style = MaterialTheme.typography.labelMedium, color = Muted, maxLines = 1,
                    overflow = TextOverflow.Ellipsis)
            }
        }
        FilledTonalIconButton(
            onClick = onOptions, enabled = canOpenOptions, modifier = Modifier.size(50.dp),
            shape = MaterialTheme.shapes.medium,
            colors = IconButtonDefaults.filledTonalIconButtonColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        ) { WayIcon(R.drawable.ic_options, "Route options") }
    }
}

@Composable
private fun FloorSwitch(floors: List<JSONObject>, floorId: String, modifier: Modifier, onFloor: (String) -> Unit) {
    Surface(modifier, shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = .93f)) {
        LazyRow(contentPadding = PaddingValues(4.dp), horizontalArrangement = Arrangement.spacedBy(1.dp)) {
            items(floors, key = { it.getString("id") }) { floor ->
                val id = floor.getString("id")
                val name = floor.getString("name")
                val label = name.substringBefore(" · ").removePrefix("Level ")
                MapSelection(label, name, floorId == id, onClick = { onFloor(id) })
            }
        }
    }
}

@Composable
private fun ModeSwitch(mode: MapViewMode, onMode: (MapViewMode) -> Unit) {
    Surface(shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = .93f)) {
        Row(Modifier.padding(4.dp), horizontalArrangement = Arrangement.spacedBy(1.dp)) {
            MapSelection("3D", "3D cutaway", mode == MapViewMode.DEPTH, onClick = { onMode(MapViewMode.DEPTH) })
            MapSelection("2D", "2D floor plan", mode == MapViewMode.PLAN, onClick = { onMode(MapViewMode.PLAN) })
        }
    }
}

@Composable
private fun MapSelection(label: String, description: String, selected: Boolean, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val fill = if (selected) colors.primary else Color.Transparent
    val textColor = if (selected) colors.onPrimary else colors.onSurfaceVariant
    Box(Modifier.clip(MaterialTheme.shapes.small).background(fill)
        .selectable(selected = selected, onClick = onClick, role = Role.Tab)
        .semantics { contentDescription = description }
        .defaultMinSize(minWidth = 44.dp, minHeight = 44.dp).padding(horizontal = 8.dp),
        contentAlignment = Alignment.Center) {
        Text(label, color = textColor, style = MaterialTheme.typography.labelLarge, maxLines = 1)
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
) {
    val listState = rememberLazyListState()
    LaunchedEffect(state.active, state.stepIndex) { listState.scrollToItem(0) }
    LazyColumn(
        modifier.fillMaxWidth().testTag(if (state.route != null) "route-ready" else "route-loading"),
        state = listState,
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        item {
            Surface(shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surfaceContainerHigh,
                modifier = Modifier.fillMaxWidth()) {
                Column(Modifier.padding(horizontal = 14.dp, vertical = 12.dp)) {
                    if (step != null) {
                        DirectionCard(step, "Step ${state.stepIndex + 1} of ${steps.size}", destination.name)
                        state.policyLabel?.let { label ->
                            Text(label, Modifier.padding(top = 8.dp), color = Muted, style = MaterialTheme.typography.labelMedium)
                        }
                    } else {
                        Text(destination.name, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                        Text(destination.floorName, color = Muted, style = MaterialTheme.typography.bodyMedium)
                        if (state.startId != null) {
                            TextButton(onClick = onOrigin, enabled = !state.busy, contentPadding = PaddingValues(horizontal = 0.dp)) {
                                WayIcon(R.drawable.ic_pin, tint = Lime)
                                Spacer(Modifier.width(5.dp))
                                Text("From: $origin")
                            }
                        }
                    }
                }
            }
        }
        if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        val message = state.error ?: state.needs.takeIf { it.hasRequirements }?.missingEvidence() ?: state.notice
        if (message != null) item {
            Surface(shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surface, modifier = Modifier.fillMaxWidth()) {
                Text(message, Modifier.padding(12.dp), color = if (state.error != null || state.needs.hasRequirements)
                    MaterialTheme.colorScheme.error else Lime)
            }
        }
        if (showSteps && !state.active) items(steps) { segment ->
            Surface(shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surface, modifier = Modifier.fillMaxWidth()) {
                Text(segment.getString("instruction"), Modifier.padding(12.dp))
            }
        }
    }
}

@Composable
private fun RouteSummary(distance: Int, stepCount: Int, showSteps: Boolean, modifier: Modifier, onClick: () -> Unit) {
    Surface(onClick = onClick, modifier = modifier.heightIn(min = 76.dp),
        shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surfaceContainerHigh) {
        Row(Modifier.padding(horizontal = 14.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("$distance m", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
                    Text("$stepCount instructions", Modifier.padding(bottom = 3.dp),
                        style = MaterialTheme.typography.labelMedium, color = Muted)
                }
                Text(if (showSteps) "Hide directions" else "Show directions",
                    style = MaterialTheme.typography.labelMedium, color = Lime)
            }
            WayIcon(if (showSteps) R.drawable.ic_close else R.drawable.ic_right)
        }
    }
}

@Composable
private fun RouteControl(
    text: String,
    icon: Int,
    modifier: Modifier,
    enabled: Boolean = true,
    description: String = text,
    onClick: () -> Unit,
) {
    Surface(onClick = onClick, enabled = enabled, modifier = modifier.heightIn(min = 76.dp)
        .semantics { if (description != text) contentDescription = description },
        shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surfaceContainerHigh,
        contentColor = if (enabled) MaterialTheme.colorScheme.onSurface else Muted) {
        Column(Modifier.padding(horizontal = 8.dp, vertical = 10.dp), verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally) {
            WayIcon(icon)
            Spacer(Modifier.height(4.dp))
            Text(text, style = MaterialTheme.typography.labelMedium, textAlign = TextAlign.Center)
        }
    }
}

@Composable
private fun RouteAction(label: String, description: String, enabled: Boolean = true, onClick: () -> Unit) {
    Button(onClick = onClick, enabled = enabled, shape = MaterialTheme.shapes.large,
        modifier = Modifier.widthIn(min = 82.dp).heightIn(min = 76.dp).semantics { contentDescription = description },
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 10.dp)) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            WayIcon(R.drawable.ic_right)
            Text(label, style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.Bold)
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
