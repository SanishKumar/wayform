package app.wayform

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import app.wayform.core.MobileVenue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

data class VenueState(
    val busy: Boolean = true,
    val venues: List<VenueEntry> = emptyList(),
    val catalog: VenueCatalog? = null,
    val error: String? = null,
    val destination: Destination? = null,
    val startId: String? = null,
    val avoidStairs: Boolean = false,
    val route: JSONObject? = null,
)

class VenueViewModel(application: Application) : AndroidViewModel(application) {
    private val store = VenueStore(application)
    private val preferences = application.getSharedPreferences("wayform", 0)
    private val mutableState = MutableStateFlow(VenueState())
    val state = mutableState.asStateFlow()
    private var native: MobileVenue? = null
    private var generation = 0

    init { load() }
    fun load(id: String? = null) {
        val request = ++generation
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            var loaded: MobileVenue? = null
            try {
                val result = withContext(Dispatchers.IO) {
                    store.seed(getApplication())
                    val venues = store.entries()
                    val selected = id ?: preferences.getString("venue", null)?.takeIf { saved -> venues.any { it.id == saved } } ?: "asterion-medical-center"
                    loaded = MobileVenue(store.bytes(selected))
                    Triple(venues, selected, VenueCatalog(JSONObject(loaded.catalog())))
                }
                if (request != generation) { loaded?.close(); return@launch }
                native?.close()
                native = loaded
                preferences.edit().putString("venue", result.second).apply()
                mutableState.value = VenueState(false, result.first, result.third, startId = result.third.startId, avoidStairs = preferences.getBoolean("avoidStairs", false))
            } catch (error: Exception) {
                loaded?.close()
                if (request == generation) mutableState.value = mutableState.value.copy(busy = false, error = error.message ?: "Could not open the venue.")
            }
        }
    }
    fun preview(destination: Destination, start: String? = state.value.startId, avoidStairs: Boolean = state.value.avoidStairs) {
        val venue = native ?: return
        val startId = start ?: return
        val request = ++generation
        preferences.edit().putBoolean("avoidStairs", avoidStairs).apply()
        mutableState.value = state.value.copy(destination = destination, startId = startId, avoidStairs = avoidStairs, route = null, busy = true, error = null)
        viewModelScope.launch {
            try {
                val route = withContext(Dispatchers.Default) { JSONObject(venue.preview(startId, destination.id, avoidStairs)) }
                if (request != generation) return@launch
                check(route.getBoolean("found")) { route.optString("error", "No route is available with these options.") }
                mutableState.value = state.value.copy(busy = false, route = route)
            } catch (error: Exception) {
                if (request == generation) mutableState.value = state.value.copy(busy = false, route = null, error = error.message ?: "Could not plan this route.")
            }
        }
    }
    fun backToPlaces() {
        generation++
        mutableState.value = state.value.copy(destination = null, route = null, busy = false, error = null)
    }
    override fun onCleared() { generation++; native?.close(); store.close() }
}
