package app.wayform

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import app.wayform.core.MobileVenue
import app.wayform.core.TripStopReason
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
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
    val confirmed: Boolean = false,
    val active: Boolean = false,
    val stepIndex: Int = 0,
    val policyLabel: String? = null,
    val notice: String? = null,
)

class VenueViewModel(application: Application) : AndroidViewModel(application) {
    private val store = VenueStore(application)
    private val preferences = application.getSharedPreferences("wayform", 0)
    private val mutableState = MutableStateFlow(VenueState())
    val state = mutableState.asStateFlow()
    private var native: MobileVenue? = null
    private var generation = 0
    private var trip: TripGuard? = null
    private var policy: ByteArray? = null
    private var watch: Job? = null
    private var foreground = true
    private var planningTrip = false

    init { load() }
    fun load(id: String? = null) {
        stopTrip(TripStopReason.CONTEXT_CHANGED)
        policy = null
        val request = ++generation
        mutableState.value = mutableState.value.copy(busy = true, error = null)
        viewModelScope.launch {
            var loaded: MobileVenue? = null
            try {
                val result = withContext(Dispatchers.IO) {
                    store.seed(getApplication())
                    val venues = store.entries()
                    val selected = id ?: preferences.getString("venue", null)?.takeIf { saved -> venues.any { it.id == saved } } ?: "asterion-medical-center"
                    val bytes = store.bytes(selected)
                    loaded = MobileVenue(bytes)
                    val lifts = JSONObject(bytes.toString(Charsets.UTF_8)).getJSONArray("verticalConnectors").objects().filter { it.getString("kind") == "elevator" }.map { it.getString("id") }
                    Triple(venues, selected, VenueCatalog(JSONObject(loaded.catalog()), lifts))
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
        stopTrip(TripStopReason.CONTEXT_CHANGED)
        val request = ++generation
        preferences.edit().putBoolean("avoidStairs", avoidStairs).apply()
        mutableState.value = state.value.copy(destination = destination, startId = startId, avoidStairs = avoidStairs, route = null, busy = true, error = null, confirmed = false, active = false, notice = null, stepIndex = 0)
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
        stopTrip(TripStopReason.CANCELLED)
        mutableState.value = state.value.copy(destination = null, route = null, busy = false, error = null, active = false, confirmed = false, notice = null)
    }
    fun confirmStart() {
        if (!state.value.busy && foreground) mutableState.value = state.value.copy(confirmed = true, error = null, notice = "Starting place confirmed manually.")
    }
    fun sampleConditions(closeLifts: Boolean, short: Boolean = false) {
        val catalog = state.value.catalog ?: return
        applyPolicy(SampleConditions.create(catalog, closeLifts, if (short) 30_000 else 600_000), if (closeLifts) "Simulated: all lifts closed · 10 min" else if (short) "Simulated: open · 30 seconds" else "Simulated: open · 10 min")
    }
    fun applyPolicy(bytes: ByteArray, label: String) {
        val venue = native ?: return
        val start = state.value.startId ?: return
        stopTrip(TripStopReason.POLICY_CHANGED)
        policy = null
        val request = ++generation
        mutableState.value = state.value.copy(active = false, route = null, confirmed = false, policyLabel = null, busy = true, error = null, notice = null)
        viewModelScope.launch {
            try {
                require(bytes.size <= 64 * 1024) { "Conditions file must be 64 KiB or smaller." }
                withContext(Dispatchers.Default) {
                    val check = TripGuard(venue.startTrip(start, start, false, bytes, DeviceJourneyClock.wallMs(), DeviceJourneyClock.monotonicMs()))
                    check.use { it.read() }
                }
                if (request != generation) return@launch
                policy = bytes.copyOf()
                mutableState.value = state.value.copy(busy = false, policyLabel = label, notice = "Conditions loaded. Confirm your starting place before beginning.")
            } catch (error: Exception) {
                if (request == generation) mutableState.value = state.value.copy(busy = false, error = error.message ?: "Conditions are unavailable.")
            }
        }
    }
    fun startGuidance() {
        val venue = native ?: return
        val current = state.value
        val destination = current.destination ?: return
        val start = current.startId ?: return
        val conditions = policy ?: return
        if (!current.confirmed || !foreground || current.busy) return
        stopTrip(TripStopReason.CONTEXT_CHANGED)
        val request = ++generation
        planningTrip = true
        mutableState.value = current.copy(busy = true, route = null, error = null, notice = null, stepIndex = 0)
        viewModelScope.launch {
            var pending: TripGuard? = null
            try {
                withContext(Dispatchers.Default) { pending = TripGuard(venue.startTrip(start, destination.id, current.avoidStairs, conditions, DeviceJourneyClock.wallMs(), DeviceJourneyClock.monotonicMs())) }
                if (request != generation || !foreground) { pending?.close(); return@launch }
                val guard = requireNotNull(pending)
                val route = guard.read()
                trip = guard
                pending = null
                planningTrip = false
                mutableState.value = state.value.copy(busy = false, active = true, route = route)
                watch = viewModelScope.launch {
                    while (trip === guard) {
                        try {
                            delay(guard.delayMs())
                            if (trip !== guard) break
                            guard.read()
                        } catch (error: CancellationException) { throw error }
                        catch (error: Exception) { recover(error.message ?: "Conditions are no longer available."); break }
                    }
                }
            } catch (error: Exception) {
                pending?.close()
                if (request == generation) {
                    planningTrip = false
                    mutableState.value = state.value.copy(busy = false, active = false, route = null, confirmed = false, error = error.message ?: "Guidance is unavailable.")
                }
            }
        }
    }
    fun nextInstruction() {
        val guard = trip ?: return
        try {
            val route = guard.read()
            if (state.value.stepIndex + 1 >= route.getJSONArray("steps").length()) {
                recover("Journey complete. You marked your arrival.")
            } else mutableState.value = state.value.copy(route = route, stepIndex = state.value.stepIndex + 1)
        } catch (error: Exception) { recover(error.message ?: "Guidance is unavailable.") }
    }
    fun recover(message: String = "Guidance paused. Choose your current starting place and confirm it before restarting.") {
        generation++
        stopTrip(TripStopReason.CONTEXT_CHANGED)
        mutableState.value = state.value.copy(active = false, route = null, confirmed = false, busy = false, notice = message, error = null)
    }
    fun resume() { foreground = true }
    fun suspendGuidance() {
        foreground = false
        if (trip != null || planningTrip) recover("Guidance paused while the app was away. Confirm your location before restarting.")
        else mutableState.value = state.value.copy(confirmed = false)
    }
    private fun stopTrip(reason: TripStopReason) {
        watch?.cancel(); watch = null
        val previous = trip
        trip = null
        // Local withdrawal must complete even if the native handle cannot be released.
        runCatching { previous?.stop(reason) }
        planningTrip = false
    }
    override fun onCleared() { generation++; stopTrip(TripStopReason.CANCELLED); native?.close(); store.close() }
}
