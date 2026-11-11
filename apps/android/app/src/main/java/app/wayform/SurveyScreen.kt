package app.wayform

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import java.text.NumberFormat

@Composable
internal fun SurveyScreen(needs: MobilityNeeds, onSaveNeeds: (MobilityNeeds) -> Unit, onExit: () -> Unit, model: SurveyViewModel = viewModel()) {
    val state by model.state.collectAsStateWithLifecycle()
    val bundle = state.bundle
    val profile = if (bundle?.example == true) state.exampleNeeds else needs
    var editNeeds by remember { mutableStateOf(false) }
    var choosing by remember { mutableStateOf<String?>(null) }
    var evidence by remember(state.assessment) { mutableStateOf(false) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(model::open) }
    BackHandler { model.clear(); onExit() }

    LazyColumn(Modifier.fillMaxSize().testTag("survey-review"), contentPadding = PaddingValues(18.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        if (bundle == null) {
            item {
                Text("Survey routes", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold)
                Text("Find a path that fits your width, slope or step requirements.", Modifier.padding(top = 6.dp), color = Muted)
            }
            item {
                Surface(color = Forest, contentColor = Color.White, shape = MaterialTheme.shapes.large) {
                    Column(Modifier.fillMaxWidth().padding(20.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Text("Clinic entrance", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                        Text("Compare two entrances using fictional doorway measurements.")
                        FilledTonalButton(onClick = { model.open(null) }, enabled = !state.busy) { Text("Try the clinic example") }
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
                Text(if (bundle.example) "Fictional survey · 25 Oct 2026" else "Imported survey · Revision ${bundle.revision}", color = Muted)
            }
            item {
                Surface(color = Color.White, shape = MaterialTheme.shapes.large) {
                    Column(Modifier.fillMaxWidth().padding(horizontal = 10.dp)) {
                        TextButton(onClick = { choosing = "From" }, enabled = !state.busy) { Text("From: ${state.start}") }
                        HorizontalDivider()
                        TextButton(onClick = { choosing = "To" }, enabled = !state.busy) { Text("To: ${state.end}") }
                    }
                }
            }
            if (bundle.example) item {
                Text("Minimum doorway width", style = MaterialTheme.typography.titleMedium)
                LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(listOf("75", "90", "120")) { width ->
                        FilterChip(selected = profile.widthCm == width, onClick = { model.setExampleNeeds(profile.copy(widthCm = width)) },
                            label = { Text("$width cm") })
                    }
                }
            }
            item {
                val summary = buildList {
                    if (profile.widthCm.isNotBlank() && (!bundle.example || profile.widthCm !in listOf("75", "90", "120"))) add("Width ≥ ${profile.widthCm} cm")
                    if (profile.slopePercent.isNotBlank()) add("Slope ≤ ${profile.slopePercent}%")
                    if (profile.noSteps) add("Zero steps")
                }.joinToString(" · ")
                if (summary.isNotEmpty()) Text(summary, color = Muted)
                TextButton(onClick = { editNeeds = true }, enabled = !state.busy) {
                    Text(if (bundle.example) "Edit example requirements" else "Mobility requirements")
                }
                if (!bundle.example) {
                    if (!profile.hasRequirements) Text("Set a requirement to check this route.", color = Muted)
                    Button(onClick = { model.check(profile) }, enabled = profile.hasRequirements && state.start != state.end && !state.busy) {
                        Text(if (state.assessment == null) "Check route" else "Check again")
                    }
                }
                if (state.start == state.end) Text("Choose two different places.", color = Muted)
            }
            state.assessment?.let { result ->
                item {
                    val routed = result.outcome == "routed"
                    Surface(color = if (routed) Lime else MaterialTheme.colorScheme.surfaceVariant, shape = MaterialTheme.shapes.large) {
                        Column(Modifier.fillMaxWidth().padding(18.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(when (result.outcome) {
                                "routed" -> "Route meets selected requirements"
                                "insufficient-evidence" -> "More evidence needed"
                                else -> "No eligible path"
                            }, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                            result.distanceMeters?.let {
                                val count = result.legs.size
                                Text("${NumberFormat.getNumberInstance().format(it)} m · $count ${if (count == 1) "segment" else "segments"}", style = MaterialTheme.typography.headlineSmall)
                            }
                            if (routed) {
                                val places = result.legs.take(3).map { it.from }
                                val middle = if (result.legs.size > 3) listOf("…") else emptyList()
                                Text((places + middle + state.end).joinToString(" → "))
                            }
                            else Text(result.reasons.joinToString(", ") { it.replace('-', ' ') }.replaceFirstChar(Char::uppercase).ifEmpty { "The survey has no qualifying path between these places." })
                        }
                    }
                }
                item { TextButton(onClick = { evidence = !evidence }) { Text(if (evidence) "Hide details" else "View route details") } }
                if (evidence) {
                    item { Text("Evaluated at ${result.evaluatedAt}", style = MaterialTheme.typography.bodySmall) }
                    items(result.legs) { Text("${it.from} → ${it.to} · ${it.edgeId}") }
                    if (result.evidence.isNotEmpty()) item { Text("Evidence records", fontWeight = FontWeight.SemiBold) }
                    items(result.evidence) { Text(it) }
                }
            }
            item {
                Text(if (bundle.example) "Example settings apply only to this fictional survey." else "Planning assessment. Confirm current conditions with the venue before travel.", color = Muted, style = MaterialTheme.typography.bodySmall)
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
