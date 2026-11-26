package app.wayform

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import java.text.NumberFormat

@Composable
internal fun SurveyScreen(needs: MobilityNeeds, onSaveNeeds: (MobilityNeeds) -> Unit, onExit: () -> Unit, model: SurveyViewModel = viewModel()) {
    val state by model.state.collectAsStateWithLifecycle()
    val colors = MaterialTheme.colorScheme
    val bundle = state.bundle
    val profile = if (bundle?.example == true) state.exampleNeeds else needs
    var editNeeds by remember { mutableStateOf(false) }
    var choosing by remember { mutableStateOf<String?>(null) }
    var evidence by remember(state.assessment) { mutableStateOf(false) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(model::open) }
    BackHandler { model.clear(); onExit() }

    LazyColumn(
        Modifier.fillMaxSize().testTag("survey-review"),
        contentPadding = PaddingValues(start = 18.dp, end = 18.dp, top = 14.dp, bottom = 20.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (bundle == null) {
            item {
                Text("Survey routes", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.SemiBold)
                Text("Check whether a route meets your width, slope or step requirements.",
                    Modifier.padding(top = 6.dp), color = colors.onSurfaceVariant)
            }
            item {
                Surface(color = colors.surface, shape = MaterialTheme.shapes.extraLarge) {
                    Column(Modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                        ClinicRouteSchematic()
                        Column(Modifier.padding(horizontal = 6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Text("Clinic entrance", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                            Text("Compare two routes using fictional doorway measurements.",
                                color = colors.onSurfaceVariant, style = MaterialTheme.typography.bodyMedium)
                        }
                        Button(
                            onClick = { model.open(null) },
                            enabled = !state.busy,
                            modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp),
                            shape = MaterialTheme.shapes.medium,
                        ) {
                            Text("Try the clinic example", Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
                            WayIcon(R.drawable.ic_right)
                        }
                    }
                }
            }
            item {
                Surface(
                    onClick = { picker.launch(arrayOf("application/json", "text/plain", "application/octet-stream")) },
                    enabled = !state.busy, color = colors.surface, shape = MaterialTheme.shapes.large,
                ) {
                    Row(Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(horizontal = 16.dp, vertical = 8.dp),
                        horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        WayIcon(R.drawable.ic_survey)
                        Text("Open survey file", Modifier.weight(1f), fontWeight = FontWeight.Medium)
                        WayIcon(R.drawable.ic_right)
                    }
                }
            }
        } else {
            item {
                Text(if (bundle.example) "Fictional survey · 25 Oct 2026" else "Imported survey · Revision ${bundle.revision}",
                    color = colors.onSurfaceVariant, style = MaterialTheme.typography.labelMedium)
                Text(if (bundle.example) "Clinic entrance" else bundle.id, Modifier.padding(top = 4.dp),
                    style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.SemiBold)
            }
            item {
                Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    SurveyEndpoint("From", state.start, enabled = !state.busy,
                        modifier = Modifier.weight(1f).fillMaxHeight()) { choosing = "From" }
                    SurveyEndpoint("To", state.end, enabled = !state.busy,
                        modifier = Modifier.weight(1f).fillMaxHeight()) { choosing = "To" }
                }
            }
            item {
                val summary = buildList {
                    if (profile.widthCm.isNotBlank() && (!bundle.example || profile.widthCm !in listOf("75", "90", "120"))) add("Width ≥ ${profile.widthCm} cm")
                    if (profile.slopePercent.isNotBlank()) add("Slope ≤ ${profile.slopePercent}%")
                    if (profile.noSteps) add("Zero steps")
                }.joinToString(" · ")
                Surface(color = colors.surface, shape = MaterialTheme.shapes.large) {
                    Column(Modifier.fillMaxWidth().padding(start = 12.dp, end = 12.dp, top = 14.dp, bottom = 4.dp)) {
                        if (bundle.example) {
                            Text("Minimum doorway width", Modifier.padding(start = 4.dp), style = MaterialTheme.typography.titleMedium)
                            Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min).padding(top = 10.dp),
                                horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                for (width in listOf("75", "90", "120")) {
                                    val selected = profile.widthCm == width
                                    Surface(
                                        selected = selected,
                                        onClick = { model.setExampleNeeds(profile.copy(widthCm = width)) },
                                        modifier = Modifier.weight(1f).fillMaxHeight(),
                                        shape = MaterialTheme.shapes.small,
                                        color = if (selected) colors.primary else colors.surfaceContainerHigh,
                                        contentColor = if (selected) colors.onPrimary else colors.onSurface,
                                    ) {
                                        Box(Modifier.heightIn(min = 48.dp).padding(horizontal = 8.dp, vertical = 8.dp),
                                            contentAlignment = Alignment.Center) {
                                            Text("$width cm", style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.SemiBold)
                                        }
                                    }
                                }
                            }
                        }
                        if (summary.isNotEmpty()) Text(summary, Modifier.padding(start = 4.dp, top = 6.dp),
                            color = colors.onSurfaceVariant, style = MaterialTheme.typography.bodyMedium)
                        TextButton(
                            onClick = { editNeeds = true }, enabled = !state.busy,
                            colors = ButtonDefaults.textButtonColors(contentColor = colors.onSurfaceVariant),
                            contentPadding = PaddingValues(horizontal = 4.dp),
                        ) {
                            Text(if (bundle.example) "Edit example requirements" else "Mobility requirements")
                        }
                        if (!bundle.example) {
                            if (!profile.hasRequirements) Text("Set a requirement to check this route.", color = colors.onSurfaceVariant)
                            Button(onClick = { model.check(profile) },
                                enabled = profile.hasRequirements && state.start != state.end && !state.busy,
                                modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp), shape = MaterialTheme.shapes.medium) {
                                Text(if (state.assessment == null) "Check route" else "Check again")
                            }
                        }
                        if (state.start == state.end) Text("Choose two different places.", color = colors.onSurfaceVariant)
                    }
                }
            }
            state.assessment?.let { result ->
                item {
                    val routed = result.outcome == "routed"
                    Surface(color = colors.surfaceContainerHigh, contentColor = colors.onSurface, shape = MaterialTheme.shapes.extraLarge) {
                        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 16.dp)
                            .semantics { liveRegion = LiveRegionMode.Polite },
                            verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(when (result.outcome) {
                                "routed" -> "Route meets selected requirements"
                                "insufficient-evidence" -> "More evidence needed"
                                else -> "No eligible path"
                            }, style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.SemiBold,
                                color = if (routed) colors.primary else colors.onSurface)
                            result.distanceMeters?.let {
                                val count = result.legs.size
                                Text("${NumberFormat.getNumberInstance().format(it)} m · $count ${if (count == 1) "segment" else "segments"}",
                                    style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.SemiBold)
                            }
                            if (routed) {
                                val places = result.legs.take(3).map { it.from }
                                val middle = if (result.legs.size > 3) listOf("…") else emptyList()
                                Text((places + middle + state.end).joinToString(" → "), color = colors.onSurfaceVariant)
                            } else {
                                val reason = result.reasons.joinToString(", ") { it.replace('-', ' ') }.replaceFirstChar(Char::uppercase)
                                Text(reason.ifEmpty { "The survey has no qualifying path between these places." }, color = colors.onSurfaceVariant)
                            }
                            FilledTonalButton(onClick = { evidence = !evidence }, shape = MaterialTheme.shapes.small) {
                                Text(if (evidence) "Hide details" else "View route details")
                            }
                        }
                    }
                }
                if (evidence) {
                    item { Text("Evaluated at ${result.evaluatedAt}", style = MaterialTheme.typography.bodySmall) }
                    items(result.legs) { Text("${it.from} → ${it.to} · ${it.edgeId}") }
                    if (result.evidence.isNotEmpty()) item { Text("Evidence records", fontWeight = FontWeight.SemiBold) }
                    items(result.evidence) { Text(it) }
                }
            }
            item {
                Text(if (bundle.example) "Example settings apply only to this fictional survey." else "Planning assessment. Confirm current conditions with the venue before travel.",
                    color = colors.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
                TextButton(onClick = model::clear, contentPadding = PaddingValues(horizontal = 0.dp)) { Text("Change survey") }
            }
        }
        if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        state.error?.let { item { Text(it, color = MaterialTheme.colorScheme.error) } }
    }
    if (editNeeds) MobilityNeedsDialog(profile, onSave = {
        if (bundle?.example == true) model.setExampleNeeds(it) else { model.select(); onSaveNeeds(it) }
        editNeeds = false
    }, onDismiss = { editNeeds = false })
    choosing?.let { label -> AlertDialog(onDismissRequest = { choosing = null }, title = { Text("Choose endpoint") }, text = {
        LazyColumn { items(bundle?.nodes.orEmpty()) { node ->
            TextButton(onClick = {
                if (label == "From") model.select(start = node) else model.select(end = node)
                choosing = null
            }) { Text(node) }
        } }
    }, confirmButton = { TextButton(onClick = { choosing = null }) { Text("Close") } }) }
}

@Composable
private fun ClinicRouteSchematic() {
    val colors = MaterialTheme.colorScheme
    Surface(color = colors.surfaceContainerHigh, shape = MaterialTheme.shapes.medium) {
        Canvas(Modifier.fillMaxWidth().height(110.dp)) {
            val entry = Offset(size.width * .21f, size.height * .73f)
            val lobby = Offset(entry.x, size.height * .26f)
            val reception = Offset(size.width * .79f, entry.y)
            val line = 2.dp.toPx()
            drawLine(colors.outline, entry, reception, strokeWidth = line, cap = StrokeCap.Round)
            drawLine(colors.primary, entry, lobby, strokeWidth = line, cap = StrokeCap.Round)
            drawLine(colors.primary, lobby, reception, strokeWidth = line, cap = StrokeCap.Round)
            for (node in listOf(entry, lobby, reception)) {
                drawCircle(colors.primary.copy(alpha = .14f), 12.dp.toPx(), node)
                drawCircle(colors.primary, 4.dp.toPx(), node)
            }
        }
    }
}

@Composable
private fun SurveyEndpoint(label: String, place: String, enabled: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val content = if (enabled) colors.onSurface else colors.onSurface.copy(alpha = 0.38f)
    Surface(onClick = onClick, enabled = enabled, color = colors.surface, contentColor = content,
        shape = MaterialTheme.shapes.medium, modifier = modifier) {
        Column(Modifier.heightIn(min = 78.dp).padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Surface(color = colors.surfaceContainerHigh, shape = CircleShape) {
                    Box(Modifier.size(28.dp), contentAlignment = Alignment.Center) { WayIcon(R.drawable.ic_pin) }
                }
                Spacer(Modifier.weight(1f))
                WayIcon(R.drawable.ic_right)
            }
            Text(label, color = if (enabled) colors.onSurfaceVariant else content, style = MaterialTheme.typography.labelMedium)
            Text(place, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        }
    }
}
