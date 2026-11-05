package app.wayform

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import java.text.NumberFormat

@Composable
internal fun SurveyScreen(needs: MobilityNeeds, onSaveNeeds: (MobilityNeeds) -> Unit, onExit: () -> Unit, model: SurveyViewModel = viewModel()) {
    val state by model.state.collectAsStateWithLifecycle()
    var editNeeds by remember { mutableStateOf(false) }
    var choosing by remember { mutableStateOf<String?>(null) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> uri?.let(model::open) }
    val close = { model.clear(); onExit() }
    BackHandler(onBack = close)
    val bundle = state.bundle
    LazyColumn(Modifier.fillMaxSize().safeDrawingPadding().testTag("survey-review"), contentPadding = PaddingValues(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        item { TextButton(onClick = close) { Text("Back to places") } }
        item {
            Text("Survey routes", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
            Text("Check whether a path meets your requirements using measurements in a venue file.", style = MaterialTheme.typography.bodyMedium)
        }
        item {
            OutlinedButton(onClick = { picker.launch(arrayOf("application/json", "text/plain", "application/octet-stream")) }) { Text("Open survey file") }
            TextButton(onClick = { model.open(null) }) { Text("Try the clinic example") }
        }
        if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        state.error?.let { message -> item { Text(message, color = MaterialTheme.colorScheme.error) } }
        if (bundle != null) {
            item {
                Text(if (bundle.example) "Fictional clinic · recorded example" else "Imported survey · revision ${bundle.revision}", style = MaterialTheme.typography.labelLarge, color = Forest)
                Text(if (bundle.example) "The example is evaluated at its recorded time on 25 Oct 2026. Try a minimum width of 90 cm." else bundle.id)
            }
            item {
                OutlinedButton(onClick = { choosing = "From" }, enabled = !state.busy) { Text("From: ${state.start}") }
                OutlinedButton(onClick = { choosing = "To" }, enabled = !state.busy) { Text("To: ${state.end}") }
            }
            item {
                TextButton(onClick = { editNeeds = true }, enabled = !state.busy) { Text("Mobility requirements") }
                val selected = buildList {
                    if (needs.widthCm.isNotBlank()) add("Width ≥ ${needs.widthCm} cm")
                    if (needs.slopePercent.isNotBlank()) add("Slope ≤ ${needs.slopePercent}% in either direction")
                    if (needs.noSteps) add("Zero steps")
                }
                Text(selected.joinToString(" · ").ifEmpty { "Choose at least one requirement to check." })
                Button(onClick = { model.check(needs) }, enabled = needs.hasRequirements && state.start != state.end && !state.busy) { Text("Check route") }
            }
            val result = state.assessment
            if (result != null) {
                item {
                    Card(colors = CardDefaults.cardColors(containerColor = if (result.outcome == "routed") Lime else MaterialTheme.colorScheme.surfaceVariant)) {
                        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(when (result.outcome) {
                                "routed" -> "Route meets selected requirements"
                                "insufficient-evidence" -> "More evidence needed"
                                else -> "No eligible path"
                            }, style = MaterialTheme.typography.titleLarge)
                            result.distanceMeters?.let { Text("${NumberFormat.getNumberInstance().format(it)} m · ${result.legs.size} segments") }
                            Text("Evaluated at ${result.evaluatedAt}", style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
                items(result.legs) { Text(it) }
                items(result.reasons) { Text(it.replace('-', ' ').replaceFirstChar(Char::uppercase)) }
                if (result.evidence.isNotEmpty()) item { Text("Evidence records", style = MaterialTheme.typography.titleMedium) }
                items(result.evidence) { Text(it) }
            }
            item { Text("Planning only. This check uses reviewed field-survey records up to 30 days old. The file supplies its own evidence. Confirm current conditions with the venue before travel.", style = MaterialTheme.typography.bodySmall) }
        }
    }
    if (editNeeds) MobilityNeedsDialog(needs, onSave = { model.select(); onSaveNeeds(it); editNeeds = false }, onDismiss = { editNeeds = false })
    choosing?.let { label ->
        AlertDialog(onDismissRequest = { choosing = null }, title = { Text("Choose endpoint") }, text = {
            LazyColumn { items(bundle?.nodes.orEmpty()) { node ->
                TextButton(onClick = {
                    if (label == "From") model.select(start = node) else model.select(end = node)
                    choosing = null
                }) { Text(node) }
            } }
        }, confirmButton = { TextButton(onClick = { choosing = null }) { Text("Close") } })
    }
}
