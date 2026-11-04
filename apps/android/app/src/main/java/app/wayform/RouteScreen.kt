package app.wayform

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
import kotlin.math.roundToInt

@Composable
fun RouteScreen(state: VenueState, model: VenueViewModel) {
    val catalog = requireNotNull(state.catalog)
    val destination = requireNotNull(state.destination)
    val floors = catalog.raw.getJSONArray("floors").objects()
    val start = catalog.destinations.firstOrNull { it.id == state.startId }
    var floorId by remember(catalog.id, state.startId) { mutableStateOf(start?.floorId ?: floors.first().getString("id")) }
    var chooseStart by remember { mutableStateOf(false) }
    val route = state.route
    LazyColumn(Modifier.fillMaxSize().safeDrawingPadding().testTag(if (route != null) "route-ready" else "route-loading"), contentPadding = PaddingValues(20.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        item { TextButton(onClick = model::backToPlaces) { Text("Back to places") } }
        item {
            Text("ROUTE PREVIEW · SAMPLE", style = MaterialTheme.typography.labelMedium, color = Forest)
            Text(destination.name, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.SemiBold)
            Text(destination.floorName, style = MaterialTheme.typography.bodyMedium)
        }
        item { OutlinedButton(onClick = { chooseStart = true }, modifier = Modifier.fillMaxWidth(), enabled = !state.busy) { Text("From: ${start?.name ?: "Check-in location"}") } }
        item {
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
        item {
            LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(floors) { floor -> FilterChip(selected = floorId == floor.getString("id"), onClick = { floorId = floor.getString("id") }, label = { Text(floor.getString("name")) }) }
            }
            VenueMap(catalog, route, floorId)
        }
        if (route != null) {
            item { Text("${route.getDouble("totalDistance").roundToInt()} m · ${route.getJSONArray("steps").length()} instructions", style = MaterialTheme.typography.titleMedium) }
            item { Text("Preview only. Your location is not being tracked and operating conditions have not been checked.", style = MaterialTheme.typography.bodyMedium) }
            items(route.getJSONArray("steps").objects()) { step ->
                ElevatedCard(Modifier.fillMaxWidth()) { Text(step.getString("instruction"), modifier = Modifier.padding(16.dp)) }
            }
        }
    }
    if (chooseStart) AlertDialog(onDismissRequest = { chooseStart = false }, title = { Text("Choose a starting place") }, text = {
        LazyColumn { items(catalog.destinations) { place -> TextButton(onClick = { chooseStart = false; model.preview(destination, start = place.id) }) { Text("${place.name} · ${place.floorName}") } } }
    }, confirmButton = { TextButton(onClick = { chooseStart = false }) { Text("Close") } })
}
