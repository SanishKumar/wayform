package app.wayform

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel

val Forest = Color(0xFF173D35)
val Paper = Color(0xFFF5F5F0)
val Lime = Color(0xFFE2F29A)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MaterialTheme(colorScheme = lightColorScheme(primary = Forest, secondary = Forest, background = Paper, surface = Paper, primaryContainer = Lime, secondaryContainer = Lime, onSecondaryContainer = Forest)) {
                Surface(Modifier.fillMaxSize(), color = Paper) { VenueScreen() }
            }
        }
    }
}

@Composable
fun VenueScreen(model: VenueViewModel = viewModel()) {
    val state by model.state.collectAsStateWithLifecycle()
    var query by rememberSaveable { mutableStateOf("") }
    var chooseVenue by remember { mutableStateOf(false) }
    val catalog = state.catalog
    LazyColumn(Modifier.fillMaxSize().safeDrawingPadding(), contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
        item {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text("WAYFORM", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Black, color = Forest)
                Text("OFFLINE", color = Forest, modifier = Modifier.background(Lime, RoundedCornerShape(20.dp)).padding(horizontal = 12.dp, vertical = 6.dp), style = MaterialTheme.typography.labelMedium)
            }
        }
        item { Text("A clearer way\nto get there.", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.SemiBold) }
        item {
            Card(colors = CardDefaults.cardColors(containerColor = Forest)) {
                Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("SAMPLE VENUE", style = MaterialTheme.typography.labelMedium, color = Lime)
                    Text(catalog?.name ?: "Opening your offline venues…", style = MaterialTheme.typography.titleLarge, color = Color.White)
                    Text("Synthetic maps for exploring the app. Physical access and operating conditions have not been surveyed.", style = MaterialTheme.typography.bodyMedium, color = Color.White.copy(alpha = .85f))
                    FilledTonalButton(onClick = { chooseVenue = true }, enabled = !state.busy) { Text("Change venue") }
                }
            }
        }
        if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        state.error?.let { message -> item { Text(message, color = MaterialTheme.colorScheme.error); Button(onClick = { model.load() }) { Text("Try again") } } }
        if (catalog != null && !state.busy) {
            item { OutlinedTextField(query, { query = it }, label = { Text("Find a room or place") }, modifier = Modifier.fillMaxWidth(), singleLine = true, shape = RoundedCornerShape(16.dp)) }
            val results = catalog.destinations.filter { it.matches(query) }
            item { Text("${results.size} ${if (results.size == 1) "place" else "places"} · stored on this device", style = MaterialTheme.typography.labelLarge) }
            items(results, key = { it.id }) { destination ->
                Column(Modifier.fillMaxWidth().background(Color.White, RoundedCornerShape(16.dp)).padding(18.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(destination.name, style = MaterialTheme.typography.titleMedium)
                    Text(destination.floorName, style = MaterialTheme.typography.bodyMedium, color = Forest)
                }
            }
            if (results.isEmpty()) item { Text("No matching places. Try a room name, service or floor.") }
        }
    }
    if (chooseVenue) AlertDialog(onDismissRequest = { chooseVenue = false }, title = { Text("Offline venues") }, text = {
        Column { state.venues.forEach { venue -> TextButton(onClick = { chooseVenue = false; query = ""; model.load(venue.id) }) { Text(venue.name) } } }
    }, confirmButton = { TextButton(onClick = { chooseVenue = false }) { Text("Close") } })
}
