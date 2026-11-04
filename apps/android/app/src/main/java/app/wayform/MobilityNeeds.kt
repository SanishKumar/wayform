package app.wayform

data class MobilityNeeds(val widthCm: String = "", val slopePercent: String = "", val noSteps: Boolean = false) {
    val hasRequirements: Boolean get() = widthCm.isNotBlank() || slopePercent.isNotBlank() || noSteps
    fun validationError(): String? {
        if (widthCm.isNotBlank() && widthCm.toDoubleOrNull()?.let { it.isFinite() && it in 20.0..500.0 } != true) return "Enter a clear width between 20 and 500 cm, or leave it blank."
        if (slopePercent.isNotBlank() && slopePercent.toDoubleOrNull()?.let { it.isFinite() && it in 0.0..100.0 } != true) return "Enter a slope between 0 and 100%, or leave it blank."
        return null
    }
    fun missingEvidence(): String = "No verified route: this venue has no surveyed ${buildList { if (widthCm.isNotBlank()) add("clear-width"); if (slopePercent.isNotBlank()) add("slope"); if (noSteps) add("step-count") }.joinToString(", ")} evidence. Your requirements remain enabled. Unknown does not mean inaccessible."
}
