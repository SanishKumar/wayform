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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
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
    val keyboardVisible = WindowInsets.ime.getBottom(LocalDensity.current) > 0
    if (state.catalog != null && state.destination != null && !survey) {
        BackHandler { model.backToPlaces() }
        RouteScreen(state, model)
        return
    }

    Column(Modifier.fillMaxSize().imePadding(), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.fillMaxWidth().weight(1f)) {
            if (survey) {
                Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))) {
                    SurveyScreen(state.needs, model::saveNeeds, onExit = { survey = false }, model = surveyModel)
                }
            } else {
                VenueHome(
                    state = state,
                    query = query,
                    keyboardVisible = keyboardVisible,
                    onQueryChange = { query = it },
                    onVenueClick = { chooseVenue = true },
                    onDestinationClick = { model.preview(it) },
                    onRetry = { model.load() },
                )
            }
        }
        if (!keyboardVisible) {
            HomeDock(
                survey = survey,
                onPlaces = {
                    if (survey) surveyModel.clear()
                    survey = false
                },
                onSurvey = { survey = true },
            )
        }
    }

    if (chooseVenue) {
        AlertDialog(
            onDismissRequest = { chooseVenue = false },
            title = { Text("Choose a venue") },
            text = {
                LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    items(state.venues, key = { it.id }) { venue ->
                        TextButton(
                            onClick = {
                                chooseVenue = false
                                query = ""
                                model.load(venue.id)
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) { Text(venue.name, Modifier.weight(1f)) }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { chooseVenue = false }) { Text("Close") } },
        )
    }
}

@Composable
private fun HomeDock(survey: Boolean, onPlaces: () -> Unit, onSurvey: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    Row(
        Modifier.navigationBarsPadding().windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Horizontal))
            .padding(horizontal = 20.dp, vertical = 10.dp).widthIn(max = 376.dp).fillMaxWidth().selectableGroup(),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        HomeTab("Places", R.drawable.ic_pin, !survey, Modifier.weight(1f), onPlaces)
        HomeTab("Survey routes", R.drawable.ic_survey, survey, Modifier.weight(1f), onSurvey)
    }
}

@Composable
private fun HomeTab(text: String, icon: Int, selected: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val shape = RoundedCornerShape(19.dp)
    val accent = if (selected) colors.primary else colors.onSurfaceVariant
    Surface(
        modifier = modifier.clip(shape).selectable(selected = selected, role = Role.Tab, onClick = onClick),
        shape = shape,
        color = if (selected) colors.surfaceContainerHighest else colors.surface,
    ) {
        Row(
            Modifier.heightIn(min = 52.dp).padding(horizontal = 10.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
        ) {
            WayIcon(icon, tint = accent)
            Text(text, color = if (selected) colors.onSurface else accent,
                style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.SemiBold)
        }
    }
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

    BoxWithConstraints(Modifier.fillMaxSize()) {
        val floor = remember(catalog) { catalog?.let(::overviewFloor) }
        val showMap = floor != null && !state.busy && state.error == null &&
            query.isBlank() && !keyboardVisible && maxHeight >= (420 * LocalDensity.current.fontScale.coerceIn(1f, 1.4f)).dp
        if (showMap && catalog != null) {
            val mapHeight = (maxHeight * .73f).coerceAtMost(640.dp)
            VenueMap(
                catalog = catalog,
                route = null,
                floorId = floor.getString("id"),
                modifier = Modifier.fillMaxWidth().height(mapHeight),
                gesturesEnabled = false,
                framing = PaddingValues(top = 126.dp, bottom = 72.dp),
            )
            Box(Modifier.fillMaxWidth().height(mapHeight).background(Brush.verticalGradient(
                0f to Paper.copy(alpha = .70f),
                .19f to Color.Transparent,
                .71f to Color.Transparent,
                1f to Paper,
            )))
        }
        Column(
            Modifier.align(Alignment.TopCenter).fillMaxHeight().widthIn(max = 560.dp).fillMaxWidth()
                .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
                .padding(horizontal = 18.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            if (!keyboardVisible) HomeHeader(state.busy, onVenueClick)
            HomeSearch(query, onQueryChange, onSearch = { keyboard?.hide() })
            if (showMap && catalog != null) {
                Box(Modifier.weight(1.42f).fillMaxWidth(), contentAlignment = Alignment.BottomStart) {
                    VenueMapCaption(catalog.name, floor.getString("name").substringBefore(" · "))
                }
            } else if (!keyboardVisible && catalog != null && !state.busy) {
                Text(catalog.name, color = colors.onSurfaceVariant, style = MaterialTheme.typography.labelLarge,
                    maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
            Surface(
                modifier = Modifier.fillMaxWidth().weight(1f),
                color = colors.surface,
                shape = RoundedCornerShape(26.dp),
            ) {
                LazyColumn(
                    Modifier.fillMaxSize().testTag("venue-list"),
                    state = listState,
                    contentPadding = PaddingValues(12.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    if (state.busy) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
                    state.error?.let { message ->
                        item {
                            Text(message, color = colors.error, modifier = Modifier.padding(4.dp))
                            TextButton(onClick = onRetry) { Text("Try again") }
                        }
                    }
                    if (catalog != null && !state.busy) {
                        item {
                            Text(
                                if (query.isBlank()) "Destinations" else "Search results · ${results.size}",
                                Modifier.padding(horizontal = 4.dp, vertical = 5.dp),
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.SemiBold,
                            )
                        }
                        items(results, key = { it.id }) { destination ->
                            DestinationCard(destination) {
                                keyboard?.hide()
                                onDestinationClick(destination)
                            }
                        }
                        if (results.isEmpty()) item {
                            Text("No matching places. Try a room, service or floor.",
                                Modifier.padding(6.dp), color = colors.onSurfaceVariant)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun HomeHeader(busy: Boolean, onVenueClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    Row(
        Modifier.fillMaxWidth().padding(top = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Surface(shape = RoundedCornerShape(17.dp), color = colors.surface) {
            Row(
                Modifier.heightIn(min = 46.dp).padding(start = 7.dp, end = 13.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(7.dp),
            ) {
                Image(painterResource(R.drawable.ic_wayform), null, Modifier.size(30.dp).clip(RoundedCornerShape(9.dp)))
                Text("WAYFORM", style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.Bold)
            }
        }
        Spacer(Modifier.weight(1f))
        Surface(
            onClick = onVenueClick, enabled = !busy, shape = RoundedCornerShape(17.dp),
            color = colors.surfaceContainerHigh,
            modifier = Modifier.semantics { contentDescription = "Change venue" },
        ) {
            Row(
                Modifier.heightIn(min = 46.dp).padding(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(7.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                WayIcon(R.drawable.ic_venue)
                Text("Venues", style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.SemiBold)
            }
        }
    }
}

@Composable
private fun HomeSearch(query: String, onQueryChange: (String) -> Unit, onSearch: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    TextField(
        value = query,
        onValueChange = onQueryChange,
        modifier = Modifier.fillMaxWidth(),
        label = { Text("Find a room or place") },
        singleLine = true,
        shape = RoundedCornerShape(18.dp),
        leadingIcon = { WayIcon(R.drawable.ic_search, tint = colors.primary) },
        trailingIcon = {
            if (query.isNotEmpty()) {
                IconButton(onClick = { onQueryChange("") }) { WayIcon(R.drawable.ic_close, "Clear search") }
            }
        },
        colors = TextFieldDefaults.colors(
            focusedContainerColor = colors.surfaceContainerHigh,
            unfocusedContainerColor = colors.surfaceContainerHigh,
            focusedIndicatorColor = Color.Transparent,
            unfocusedIndicatorColor = Color.Transparent,
            focusedLabelColor = colors.onSurfaceVariant,
        ),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { onSearch() }),
    )
}

@Composable
private fun VenueMapCaption(name: String, floor: String) {
    val colors = MaterialTheme.colorScheme
    Surface(color = colors.surface.copy(alpha = .96f), shape = RoundedCornerShape(20.dp)) {
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Column(Modifier.weight(1f)) {
                Text("Sample venue", color = colors.onSurfaceVariant, style = MaterialTheme.typography.labelSmall)
                Text(name, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            }
            Surface(color = colors.surfaceContainerHighest, shape = RoundedCornerShape(12.dp)) {
                Text(floor, Modifier.padding(horizontal = 9.dp, vertical = 6.dp),
                    color = colors.onSurface, style = MaterialTheme.typography.labelMedium)
            }
        }
    }
}

@Composable
private fun DestinationCard(destination: Destination, onClick: () -> Unit) {
    val colors = MaterialTheme.colorScheme
    val shape = RoundedCornerShape(15.dp)
    Row(
        Modifier.fillMaxWidth().clip(shape).background(colors.surfaceContainerHigh)
            .clickable(role = Role.Button, onClick = onClick)
            .heightIn(min = 58.dp).padding(horizontal = 9.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier.size(36.dp).clip(RoundedCornerShape(11.dp)).background(colors.surfaceContainerHighest),
            contentAlignment = Alignment.Center,
        ) { WayIcon(R.drawable.ic_pin, tint = colors.onSurfaceVariant) }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(destination.name, fontWeight = FontWeight.SemiBold, style = MaterialTheme.typography.bodyMedium)
            Text(destination.floorName, color = colors.onSurfaceVariant, style = MaterialTheme.typography.bodySmall)
        }
        WayIcon(R.drawable.ic_right, tint = colors.onSurfaceVariant)
    }
}

private fun overviewFloor(catalog: VenueCatalog) = catalog.raw.getJSONArray("floors").objects().let { floors ->
    val startFloor = catalog.destinations.firstOrNull { it.id == catalog.startId }?.floorId
    floors.firstOrNull { it.getString("id") == startFloor } ?: floors.first()
}
