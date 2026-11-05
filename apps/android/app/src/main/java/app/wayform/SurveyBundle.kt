package app.wayform

import android.util.JsonReader
import app.wayform.core.planMobility
import java.time.Instant
import java.time.format.DateTimeFormatterBuilder
import org.json.JSONArray
import org.json.JSONObject

private val millisecondUtc = DateTimeFormatterBuilder().appendInstant(3).toFormatter()
internal val exampleTime: Instant = Instant.parse("2026-10-25T12:00:00Z")
internal const val surveyByteLimit = 1024 * 1024

private fun jsonProfile(needs: MobilityNeeds): ByteArray {
    require(needs.validationError() == null) { "Check the mobility requirements." }
    val hard = JSONObject()
    hard.put("minClearWidthMeters", needs.widthCm.toDoubleOrNull()?.div(100) ?: JSONObject.NULL)
    val slope = needs.slopePercent.toDoubleOrNull() ?: JSONObject.NULL
    hard.put("maxUphillPercent", slope).put("maxDownhillPercent", slope)
    hard.put("maxCrossSlopePercent", JSONObject.NULL)
    hard.put("maxStepCount", if (needs.noSteps) 0 else JSONObject.NULL)
    hard.put("allowedSurfaces", JSONObject.NULL).put("allowedDoorOperations", JSONObject.NULL)
    return JSONObject().apply {
        put("id", "android-survey-review")
        put("allowRestricted", false)
        put("respectLegacyExclusions", true)
        put("hard", hard)
        put("preferences", JSONObject().put("avoidKinds", JSONArray()).put("penaltyMeters", 0))
        put("evidencePolicy", JSONObject().apply {
            put("acceptedSources", JSONArray().put("field-survey"))
            put("requireReviewed", true)
            put("maxAgeSeconds", 30 * 86400)
            put("maxTransformErrorMeters", 0.25)
        })
    }.toString().toByteArray()
}

private fun evaluate(bytes: ByteArray, start: String, end: String, needs: MobilityNeeds, time: Instant): JSONObject {
    val context = JSONObject().apply {
        put("evaluatedAt", millisecondUtc.format(time))
        put("policies", JSONArray())
        put("trustedIssuerIds", JSONArray())
        put("minimumPolicyRevisions", JSONObject())
    }
    return JSONObject(planMobility(bytes, jsonProfile(needs), context.toString().toByteArray(), start, end))
}

// Read just enough to address a native query. Only the native loader can accept a file.
private fun firstNode(bytes: ByteArray): String {
    JsonReader(bytes.inputStream().bufferedReader()).use { reader ->
        reader.beginObject()
        while (reader.hasNext()) {
            if (reader.nextName() != "nodes") { reader.skipValue(); continue }
            reader.beginArray()
            require(reader.hasNext()) { "The survey needs at least two endpoints." }
            reader.beginObject()
            while (reader.hasNext()) {
                if (reader.nextName() == "id") return reader.nextString()
                reader.skipValue()
            }
            break
        }
        error("The file must contain a mobility bundle with named nodes.")
    }
}

internal class SurveyBundle private constructor(
    private val bytes: ByteArray,
    private val document: JSONObject,
    val example: Boolean,
) {
    val id: String = document.getString("id")
    val revision: String = document.getString("revision")
    val nodes = document.getJSONArray("nodes").objects().map { it.getString("id") }
    private val edges = document.getJSONArray("edges").objects().associateBy { it.getString("id") }

    fun assess(start: String, end: String, needs: MobilityNeeds, now: Instant = Instant.now()): SurveyAssessment {
        require(start != end) { "Choose different endpoints." }
        require(needs.hasRequirements) { "Choose a width, slope or step requirement first." }
        val time = if (example) exampleTime else now
        val result = evaluate(bytes, start, end, needs, time)
        val outcome = result.getString("outcome")
        if (outcome != "routed") {
            require(outcome == "insufficient-evidence" || outcome == "infeasible")
            val reasons = result.getJSONObject("diagnostic").getJSONArray("reasonCodes").strings()
            return SurveyAssessment(outcome, millisecondUtc.format(time), reasons = reasons)
        }
        val assessment = result.getJSONObject("assessment")
        val receipt = assessment.getJSONObject("receipt")
        val legs = receipt.getJSONArray("edgeIds").strings().map { edgeId ->
            val edge = edges.getValue(edgeId)
            "${edge.getString("from")} → ${edge.getString("to")} · $edgeId"
        }
        return SurveyAssessment(outcome, receipt.getString("evaluatedAt"), legs = legs,
            distanceMeters = receipt.getDouble("distanceMeters"), evidence = assessment.getJSONArray("evidenceIds").strings())
    }

    companion object {
        fun open(bytes: ByteArray, example: Boolean = false): SurveyBundle {
            require(bytes.size <= surveyByteLimit) { "Survey files must be 1 MiB or smaller." }
            val first = firstNode(bytes)
            evaluate(bytes, first, first, MobilityNeeds(), if (example) exampleTime else Instant.now())
            val bundle = SurveyBundle(bytes, JSONObject(bytes.toString(Charsets.UTF_8)), example)
            require(bundle.nodes.size >= 2) { "The survey needs at least two endpoints." }
            return bundle
        }
    }
}

private fun JSONArray.strings(): List<String> = (0 until length()).map(::getString)

internal data class SurveyAssessment(
    val outcome: String,
    val evaluatedAt: String,
    val legs: List<String> = emptyList(),
    val distanceMeters: Double? = null,
    val evidence: List<String> = emptyList(),
    val reasons: List<String> = emptyList(),
)
