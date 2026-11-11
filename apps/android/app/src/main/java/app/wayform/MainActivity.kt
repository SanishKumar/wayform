package app.wayform

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Image
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel

val Forest = Color(0xFF173D35)
val Paper = Color(0xFFF7F8F4)
val Lime = Color(0xFFE2F29A)
val Muted = Color(0xFF52635B)

@Composable
internal fun WayIcon(resource: Int, description: String? = null) {
    Icon(painterResource(resource), contentDescription = description)
}

class MainActivity : ComponentActivity() {
    private val venueModel by lazy { ViewModelProvider(this)[VenueViewModel::class.java] }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MaterialTheme(colorScheme = lightColorScheme(
                primary = Forest, onPrimary = Color.White, secondary = Forest,
                background = Paper, surface = Paper, surfaceVariant = Color(0xFFE7EDE6),
                onSurface = Color(0xFF172D24), onSurfaceVariant = Muted,
                primaryContainer = Lime, onPrimaryContainer = Forest,
                secondaryContainer = Lime, onSecondaryContainer = Forest,
            )) { Surface(Modifier.fillMaxSize()) { VenueScreen(venueModel) } }
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
    val surveyModel: SurveyViewModel = viewModel()
    val catalog = state.catalog
    val keyboard = LocalSoftwareKeyboardController.current
    if (catalog != null && state.destination != null && !survey) {
        BackHandler { model.backToPlaces() }
        RouteScreen(state, model)
        return
    }

    Column(Modifier.fillMaxSize().safeDrawingPadding().imePadding()) {
        Box(Modifier.weight(1f)) {
            if (survey) {
                SurveyScreen(state.needs, model::saveNeeds, onExit = { survey = false }, model = surveyModel)
            } else {
                LazyColumn(Modifier.fillMaxSize().testTag("venue-list"), contentPadding = PaddingValues(16.dp)) {
                    item {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Image(painterResource(R.drawable.ic_wayform), null, Modifier.size(28.dp).clip(RoundedCornerShape(7.dp)))
                            Text("WAYFORM", color = Forest, style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.Bold)
                        }
                        Text("Where to?", Modifier.padding(top = 18.dp, bottom = 10.dp),
                            style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold)
                        OutlinedTextField(query, { query = it }, modifier = Modifier.fillMaxWidth(),
                            label = { Text("Find a room or place") }, singleLine = true,
                            shape = RoundedCornerShape(18.dp), leadingIcon = { WayIcon(R.drawable.ic_search) },
                            trailingIcon = { if (query.isNotEmpty()) IconButton(onClick = { query = "" }) { WayIcon(R.drawable.ic_close, "Clear search") } },
                            colors = OutlinedTextFieldDefaults.colors(unfocusedContainerColor = Color.White, focusedContainerColor = Color.White),
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                            keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }))
                    }
                    item {
                        ListItem(
                            modifier = Modifier.fillMaxWidth().clickable(enabled = !state.busy, onClickLabel = "Change venue") { chooseVenue = true },
                            overlineContent = { Text("Offline sample venue") },
                            headlineContent = { Text(catalog?.name ?: "Opening venues…", fontWeight = FontWeight.Medium) },
                            trailingContent = { WayIcon(R.drawable.ic_right, "Change venue") },
                            colors = ListItemDefaults.colors(containerColor = Paper),
                        )
                        HorizontalDivider()
                    }
                    if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
                    state.error?.let { message -> item {
                        Text(message, color = MaterialTheme.colorScheme.error)
                        TextButton(onClick = { model.load() }) { Text("Try again") }
                    } }
                    if (catalog != null && !state.busy) {
                        val results = catalog.destinations.filter { it.matches(query) }
                        item {
                            val heading = if (query.isBlank()) "Places" else "Search results"
                            Text("$heading · ${results.size}", Modifier.padding(vertical = 14.dp),
                                style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                        }
                        items(results, key = { it.id }) { destination ->
                            ListItem(headlineContent = { Text(destination.name, fontWeight = FontWeight.Medium) },
                                supportingContent = { Text(destination.floorName, color = Muted) },
                                leadingContent = { WayIcon(R.drawable.ic_pin) },
                                trailingContent = { WayIcon(R.drawable.ic_right) },
                                colors = ListItemDefaults.colors(containerColor = Color.White),
                                modifier = Modifier.clickable { keyboard?.hide(); model.preview(destination) })
                            HorizontalDivider(color = Paper)
                        }
                        if (results.isEmpty()) item { Text("No matching places. Try a room, service or floor.") }
                    }
                }
            }
        }
        if (WindowInsets.ime.getBottom(LocalDensity.current) == 0) {
            NavigationBar(containerColor = Color.White, windowInsets = WindowInsets(0)) {
                NavigationBarItem(selected = !survey, onClick = { if (survey) surveyModel.clear(); survey = false },
                    icon = { WayIcon(R.drawable.ic_pin) }, label = { Text("Places") })
                NavigationBarItem(selected = survey, onClick = { survey = true },
                    icon = { WayIcon(R.drawable.ic_survey) }, label = { Text("Survey routes") })
            }
        }
    }
    if (chooseVenue) AlertDialog(onDismissRequest = { chooseVenue = false }, title = { Text("Choose a venue") }, text = {
        LazyColumn { items(state.venues) { venue ->
            TextButton(onClick = { chooseVenue = false; query = ""; model.load(venue.id) }) { Text(venue.name) }
        } }
    }, confirmButton = { TextButton(onClick = { chooseVenue = false }) { Text("Close") } })
}
