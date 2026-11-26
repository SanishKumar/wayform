package app.wayform

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
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

    LazyColumn(Modifier.fillMaxSize().testTag("survey-review"), contentPadding = PaddingValues(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        if (bundle == null) {
            item {
                Text("Survey routes", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold)
                Text("Check whether a route meets your width, slope or step requirements.",
                    Modifier.padding(top = 4.dp), color = colors.onSurfaceVariant)
            }
            item {
                Surface(color = colors.surfaceVariant, contentColor = colors.onSurface, shape = MaterialTheme.shapes.extraLarge) {
                    Column(Modifier.fillMaxWidth().padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Surface(color = colors.secondaryContainer, contentColor = colors.primary, shape = CircleShape) {
                            Box(Modifier.size(48.dp), contentAlignment = Alignment.Center) {
                                WayIcon(R.drawable.ic_survey)
                            }
                        }
                        Text("Clinic entrance", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                        Text("Compare two entrances using fictional doorway measurements.", color = colors.onSurfaceVariant)
                        Button(onClick = { model.open(null) }, enabled = !state.busy, modifier = Modifier.fillMaxWidth()) {
                            Text("Try the clinic example")
                        }
                    }
                }
            }
            item {
                OutlinedButton(onClick = { picker.launch(arrayOf("application/json", "text/plain", "application/octet-stream")) },
                    enabled = !state.busy, modifier = Modifier.fillMaxWidth()) { Text("Open survey file") }
            }
        } else {
            item {
                Text(if (bundle.example) "Clinic entrance" else bundle.id, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
                Text(if (bundle.example) "Fictional survey · 25 Oct 2026" else "Imported survey · Revision ${bundle.revision}",
                    Modifier.padding(top = 4.dp), color = colors.onSurfaceVariant)
            }
            item {
                Surface(color = colors.surfaceVariant, shape = MaterialTheme.shapes.large) {
                    Column(Modifier.fillMaxWidth()) {
                        SurveyEndpoint("From", state.start, enabled = !state.busy, onClick = { choosing = "From" })
                        HorizontalDivider(Modifier.padding(horizontal = 16.dp), color = colors.outlineVariant)
                        SurveyEndpoint("To", state.end, enabled = !state.busy, onClick = { choosing = "To" })
                    }
                }
            }
            if (bundle.example) item {
                Text("Minimum doorway width", style = MaterialTheme.typography.titleMedium)
                LazyRow(Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(listOf("75", "90", "120")) { width ->
                        FilterChip(selected = profile.widthCm == width, onClick = { model.setExampleNeeds(profile.copy(widthCm = width)) },
                            label = { Text("$width cm") }, colors = FilterChipDefaults.filterChipColors(
                                selectedContainerColor = colors.primary, selectedLabelColor = colors.onPrimary,
                            ))
                    }
                }
            }
            item {
                val summary = buildList {
                    if (profile.widthCm.isNotBlank() && (!bundle.example || profile.widthCm !in listOf("75", "90", "120"))) add("Width ≥ ${profile.widthCm} cm")
                    if (profile.slopePercent.isNotBlank()) add("Slope ≤ ${profile.slopePercent}%")
                    if (profile.noSteps) add("Zero steps")
                }.joinToString(" · ")
                if (summary.isNotEmpty()) Text(summary, color = colors.onSurfaceVariant)
                TextButton(onClick = { editNeeds = true }, enabled = !state.busy) {
                    Text(if (bundle.example) "Edit example requirements" else "Mobility requirements")
                }
                if (!bundle.example) {
                    if (!profile.hasRequirements) Text("Set a requirement to check this route.", color = colors.onSurfaceVariant)
                    Button(onClick = { model.check(profile) }, enabled = profile.hasRequirements && state.start != state.end && !state.busy,
                        modifier = Modifier.fillMaxWidth()) {
                        Text(if (state.assessment == null) "Check route" else "Check again")
                    }
                }
                if (state.start == state.end) Text("Choose two different places.", color = colors.onSurfaceVariant)
            }
            state.assessment?.let { result ->
                item {
                    val routed = result.outcome == "routed"
                    Surface(color = colors.surfaceVariant, contentColor = colors.onSurface, shape = MaterialTheme.shapes.extraLarge) {
                        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 16.dp)
                            .semantics { liveRegion = LiveRegionMode.Polite },
                            verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(when (result.outcome) {
                                "routed" -> "Route meets selected requirements"
                                "insufficient-evidence" -> "More evidence needed"
                                else -> "No eligible path"
                            }, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold,
                                color = if (routed) colors.primary else colors.onSurface)
                            result.distanceMeters?.let {
                                val count = result.legs.size
                                Text("${NumberFormat.getNumberInstance().format(it)} m · $count ${if (count == 1) "segment" else "segments"}",
                                    style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
                            }
                            if (routed) {
                                val places = result.legs.take(3).map { it.from }
                                val middle = if (result.legs.size > 3) listOf("…") else emptyList()
                                Text((places + middle + state.end).joinToString(" → "), color = colors.onSurfaceVariant)
                            } else {
                                val reason = result.reasons.joinToString(", ") { it.replace('-', ' ') }.replaceFirstChar(Char::uppercase)
                                Text(reason.ifEmpty { "The survey has no qualifying path between these places." }, color = colors.onSurfaceVariant)
                            }
                            TextButton(onClick = { evidence = !evidence }, contentPadding = PaddingValues(horizontal = 0.dp)) {
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
                TextButton(onClick = model::clear) { Text("Change survey") }
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
private fun SurveyEndpoint(label: String, place: String, enabled: Boolean, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val content = if (enabled) colors.onSurface else colors.onSurface.copy(alpha = 0.38f)
    Surface(onClick = onClick, enabled = enabled, color = Color.Transparent, contentColor = content) {
        Row(Modifier.fillMaxWidth().heightIn(min = 56.dp).padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
            WayIcon(R.drawable.ic_pin)
            Column(Modifier.weight(1f)) {
                Text(label, color = if (enabled) colors.onSurfaceVariant else content, style = MaterialTheme.typography.labelMedium)
                Text(place, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            }
            WayIcon(R.drawable.ic_right)
        }
    }
}
