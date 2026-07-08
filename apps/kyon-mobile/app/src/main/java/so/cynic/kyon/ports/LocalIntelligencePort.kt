package so.cynic.kyon.ports

data class SemanticAnalysis(
    val toxicityScore: Float, // 0.0 to 1.0
    val intent: String,       // e.g., "INFORMATION", "SOCIAL_HOOK", "COMMERCIAL_SPAM", "URGENT"
    val isDistracting: Boolean
)

/**
 * Sovereign AI Port
 * Defines how KYON evaluates the cognitive impact of stimuli locally.
 * To be backed by Gemini Nano (AICore) or Llama.cpp in the future.
 */
interface LocalIntelligencePort {
    suspend fun analyzeText(title: String, body: String): SemanticAnalysis
}
