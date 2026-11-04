package app.wayform

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.speech.tts.TextToSpeech
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.Locale

class OfflineSpeech(context: Context) : AutoCloseable {
    private var closed = false
    private var engine: TextToSpeech? = null
    private val mutableReady = MutableStateFlow(false)
    val ready = mutableReady.asStateFlow()
    init {
        engine = TextToSpeech(context.applicationContext) { status ->
            Handler(Looper.getMainLooper()).post {
                if (!closed && status == TextToSpeech.SUCCESS) {
                    val voice = engine?.voices?.filter { !it.isNetworkConnectionRequired && it.locale.language == Locale.ENGLISH.language && TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED !in it.features.orEmpty() }?.sortedBy { it.name }?.firstOrNull()
                    mutableReady.value = voice != null && engine?.setVoice(voice) == TextToSpeech.SUCCESS
                }
            }
        }
    }
    fun speak(text: String): Boolean {
        if (!ready.value || closed) return false
        stop()
        return engine?.speak(text.take(TextToSpeech.getMaxSpeechInputLength()), TextToSpeech.QUEUE_FLUSH, null, "wayform-instruction") == TextToSpeech.SUCCESS
    }
    fun stop() { engine?.stop() }
    override fun close() { closed = true; mutableReady.value = false; engine?.stop(); engine?.shutdown(); engine = null }
}
