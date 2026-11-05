package app.wayform

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import app.wayform.core.MobileException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

internal data class SurveyState(
    val bundle: SurveyBundle? = null,
    val start: String = "",
    val end: String = "",
    val busy: Boolean = false,
    val assessment: SurveyAssessment? = null,
    val error: String? = null,
)

internal class SurveyViewModel(application: Application) : AndroidViewModel(application) {
    private val stateFlow = MutableStateFlow(SurveyState())
    val state = stateFlow.asStateFlow()
    private var job: Job? = null

    fun clear() {
        job?.cancel()
        stateFlow.value = SurveyState()
    }

    fun open(uri: Uri?) {
        clear()
        stateFlow.value = SurveyState(busy = true)
        val app = getApplication<Application>()
        job = viewModelScope.launch {
            try {
                val bundle = withContext(Dispatchers.IO) {
                    val stream = if (uri == null) app.assets.open("survey/clinic-entrance.json")
                        else requireNotNull(app.contentResolver.openInputStream(uri)) { "Could not open the survey file." }
                    SurveyBundle.open(stream.use { it.readBounded(surveyByteLimit) }, example = uri == null)
                }
                stateFlow.value = SurveyState(bundle, bundle.nodes.first(), bundle.nodes.last())
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                stateFlow.value = SurveyState(error = surveyError(error))
            }
        }
    }

    fun select(start: String = state.value.start, end: String = state.value.end) {
        job?.cancel()
        stateFlow.value = state.value.copy(start = start, end = end, assessment = null, error = null, busy = false)
    }

    fun check(needs: MobilityNeeds) {
        val input = state.value
        val bundle = input.bundle ?: return
        if (input.busy) return
        stateFlow.value = input.copy(assessment = null, error = null, busy = true)
        job = viewModelScope.launch {
            try {
                val assessment = withContext(Dispatchers.Default) { bundle.assess(input.start, input.end, needs) }
                stateFlow.value = input.copy(assessment = assessment, error = null)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (error: Exception) {
                stateFlow.value = input.copy(assessment = null, error = surveyError(error))
            }
        }
    }

    private fun surveyError(error: Exception) =
        if (error is MobileException.Failed) error.detail else error.message ?: "Could not read the survey."
}
