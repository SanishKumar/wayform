package app.wayform

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.ViewModelProvider

val Forest = Color(0xFF173D35)
val Paper = Color(0xFFF5F5F0)
val Lime = Color(0xFFE2F29A)

class MainActivity : ComponentActivity() {
    private val venueModel by lazy { ViewModelProvider(this)[VenueViewModel::class.java] }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MaterialTheme(colorScheme = lightColorScheme(primary = Forest, secondary = Forest, background = Paper, surface = Paper, primaryContainer = Lime, secondaryContainer = Lime, onSecondaryContainer = Forest)) {
                Surface(Modifier.fillMaxSize(), color = Paper) { VenueScreen(venueModel) }
            }
        }
    }
    override fun onResume() { super.onResume(); venueModel.resume() }
    override fun onPause() { venueModel.suspendGuidance(); super.onPause() }
}

@Composable
fun VenueScreen(model: VenueViewModel = viewModel()) {
    val state by model.state.collectAsStateWithLifecycle()
    var query by rememberSaveable { mutableStateOf("") }
    var chooseVenue by remember { mutableStateOf(false) }
    var survey by rememberSaveable { mutableStateOf(false) }
    val catalog = state.catalog
    val keyboard = LocalSoftwareKeyboardController.current
    if (survey) {
        SurveyScreen(state.needs, model::saveNeeds, onExit = { survey = false })
        return
    }
    if (catalog != null && state.destination != null) {
        BackHandler { model.backToPlaces() }
        RouteScreen(state, model)
        return
    }
    LazyColumn(Modifier.fillMaxSize().safeDrawingPadding().testTag("venue-list"), contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
        item {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text("WAYFORM", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Black, color = Forest)
                Text("OFFLINE", color = Forest, modifier = Modifier.background(Lime, RoundedCornerShape(20.dp)).padding(horizontal = 12.dp, vertical = 6.dp), style = MaterialTheme.typography.labelMedium)
            }
        }
        item { Text("Find a destination", style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.SemiBold) }
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
        item { OutlinedButton(onClick = { survey = true }, enabled = !state.busy) { Text("Survey routes") } }
        if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
        state.error?.let { message -> item { Text(message, color = MaterialTheme.colorScheme.error); Button(onClick = { model.load() }) { Text("Try again") } } }
        if (catalog != null && !state.busy) {
            item { OutlinedTextField(query, { query = it }, label = { Text("Find a room or place") }, modifier = Modifier.fillMaxWidth(), singleLine = true, shape = RoundedCornerShape(16.dp), keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search), keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() })) }
            val results = catalog.destinations.filter { it.matches(query) }
            item { Text("${results.size} ${if (results.size == 1) "place" else "places"} · stored on this device", style = MaterialTheme.typography.labelLarge) }
            items(results, key = { it.id }) { destination ->
                Column(Modifier.fillMaxWidth().background(Color.White, RoundedCornerShape(16.dp)).clickable { model.preview(destination) }.padding(18.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
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
