package app.wayform

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel

@Composable
internal fun WayIcon(resource: Int, description: String? = null, tint: Color = LocalContentColor.current) {
    Icon(painterResource(resource), contentDescription = description, tint = tint)
}

class MainActivity : ComponentActivity() {
    private val venueModel by lazy { ViewModelProvider(this)[VenueViewModel::class.java] }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT),
        )
        setContent {
            WayformTheme { Surface(Modifier.fillMaxSize(), color = Paper) { VenueScreen(venueModel) } }
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
    val keyboardVisible = WindowInsets.ime.getBottom(LocalDensity.current) > 0
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
                VenueHome(
                    state, query, keyboardVisible, onQueryChange = { query = it },
                    onVenueClick = { chooseVenue = true }, onDestinationClick = { model.preview(it) },
                    onRetry = { model.load() },
                )
            }
        }
        if (!keyboardVisible) {
            HorizontalDivider()
            NavigationBar(
                modifier = Modifier.height(62.dp),
                containerColor = MaterialTheme.colorScheme.surface,
                tonalElevation = 0.dp,
                windowInsets = WindowInsets(0),
            ) {
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

@Composable
private fun VenueHome(
    state: VenueState,
    query: String,
    keyboardVisible: Boolean,
    onQueryChange: (String) -> Unit,
    onVenueClick: () -> Unit,
    onDestinationClick: (Destination) -> Unit,
    onRetry: () -> Unit,
) {
    val catalog = state.catalog
    val colors = MaterialTheme.colorScheme
    val keyboard = LocalSoftwareKeyboardController.current
    val results = remember(catalog, query) { catalog?.destinations.orEmpty().filter { it.matches(query) } }
    val listState = rememberLazyListState()
    LaunchedEffect(query, catalog?.id) { listState.scrollToItem(0) }

    BoxWithConstraints(Modifier.fillMaxSize().background(colors.background)) {
        val showMap = catalog != null && !state.busy && state.error == null &&
            query.isBlank() && !keyboardVisible && maxHeight >= 420.dp
        Column(Modifier.fillMaxSize()) {
            if (!keyboardVisible) {
                Row(
                    Modifier.fillMaxWidth().padding(start = 18.dp, end = 16.dp, top = 4.dp, bottom = 4.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Image(painterResource(R.drawable.ic_wayform), null, Modifier.size(34.dp).clip(RoundedCornerShape(11.dp)))
                    Column(Modifier.weight(1f)) {
                        Text("WAYFORM", style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold, color = colors.primary)
                        Text("Offline sample venue", style = MaterialTheme.typography.labelMedium, color = colors.onSurfaceVariant)
                    }
                    FilledTonalButton(
                        onClick = onVenueClick,
                        enabled = !state.busy,
                        contentPadding = PaddingValues(horizontal = 12.dp),
                    ) {
                        WayIcon(R.drawable.ic_venue, "Change venue")
                        Spacer(Modifier.width(5.dp))
                        Text("Venues")
                    }
                }
            }
            OutlinedTextField(
                value = query,
                onValueChange = onQueryChange,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
                label = { Text("Find a room or place") },
                singleLine = true,
                shape = RoundedCornerShape(24.dp),
                leadingIcon = { WayIcon(R.drawable.ic_search) },
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = { onQueryChange("") }) { WayIcon(R.drawable.ic_close, "Clear search") }
                    }
                },
                colors = OutlinedTextFieldDefaults.colors(
                    focusedContainerColor = colors.surfaceVariant,
                    unfocusedContainerColor = colors.surfaceVariant,
                    unfocusedBorderColor = Color.Transparent,
                ),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { keyboard?.hide() }),
            )
            if (!keyboardVisible) {
                Text(
                    catalog?.name ?: "Opening venues…",
                    Modifier.padding(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 4.dp),
                    style = MaterialTheme.typography.titleMedium,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            if (showMap && catalog != null) {
                VenueOverview(catalog, Modifier.fillMaxWidth().weight(1.18f))
            }
            Surface(
                modifier = Modifier.fillMaxWidth().weight(1f).padding(top = 4.dp),
                color = colors.surface,
                shape = RoundedCornerShape(topStart = 26.dp, topEnd = 26.dp),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize().testTag("venue-list"),
                    state = listState,
                    contentPadding = PaddingValues(start = 18.dp, end = 18.dp, top = 12.dp, bottom = 12.dp),
                ) {
                    if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
                    state.error?.let { message ->
                        item {
                            Text(message, color = colors.error)
                            TextButton(onClick = onRetry) { Text("Try again") }
                        }
                    }
                    if (catalog != null && !state.busy) {
                        item {
                            Text(
                                if (query.isBlank()) "Places" else "Search results · ${results.size}",
                                Modifier.padding(horizontal = 2.dp, vertical = 4.dp),
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.SemiBold,
                            )
                        }
                        items(results, key = { it.id }) { destination ->
                            ListItem(
                                headlineContent = { Text(destination.name, fontWeight = FontWeight.Medium) },
                                supportingContent = { Text(destination.floorName) },
                                leadingContent = {
                                    Surface(color = colors.surfaceVariant, shape = CircleShape) {
                                        Box(Modifier.size(40.dp), contentAlignment = Alignment.Center) { WayIcon(R.drawable.ic_pin) }
                                    }
                                },
                                trailingContent = { WayIcon(R.drawable.ic_right) },
                                colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                                modifier = Modifier.clip(RoundedCornerShape(14.dp)).clickable {
                                    keyboard?.hide()
                                    onDestinationClick(destination)
                                },
                            )
                            HorizontalDivider()
                        }
                        if (results.isEmpty()) item {
                            Text(
                                "No matching places. Try a room, service or floor.",
                                Modifier.padding(4.dp),
                                color = colors.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun VenueOverview(catalog: VenueCatalog, modifier: Modifier) {
    val floor = remember(catalog) {
        val startFloor = catalog.destinations.firstOrNull { it.id == catalog.startId }?.floorId
        val floors = catalog.raw.getJSONArray("floors").objects()
        floors.firstOrNull { it.getString("id") == startFloor } ?: floors.first()
    }
    Box(modifier.padding(horizontal = 12.dp).clip(RoundedCornerShape(24.dp))) {
        VenueMap(catalog, null, floor.getString("id"), Modifier.fillMaxSize(), gesturesEnabled = false)
        Surface(
            Modifier.align(Alignment.BottomStart).padding(12.dp),
            color = MaterialTheme.colorScheme.surface,
            shape = CircleShape,
        ) {
            Text(
                floor.getString("name").substringBefore(" · "),
                Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                style = MaterialTheme.typography.labelMedium,
            )
        }
    }
}
