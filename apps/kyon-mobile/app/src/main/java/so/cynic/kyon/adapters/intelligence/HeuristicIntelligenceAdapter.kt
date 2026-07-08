package so.cynic.kyon.adapters.intelligence

import so.cynic.kyon.ports.LocalIntelligencePort
import so.cynic.kyon.ports.SemanticAnalysis

/**
 * Placeholder Sovereign LLM Adapter.
 * Uses advanced heuristics and pattern matching until the on-device LLM (Gemini Nano / MLC) is fully loaded.
 */
class HeuristicIntelligenceAdapter : LocalIntelligencePort {
    override suspend fun analyzeText(title: String, body: String): SemanticAnalysis {
        val combined = "$title $body".lowercase()
        
        // Simulating semantic embeddings / attention mechanisms
        val doomscrollHooks = listOf(
            // English
            "you won't believe", "see what", "just posted", "missed you", "trending", "react", "tagged you", "story", "memories", "who viewed",
            // French
            "tu ne devineras jamais", "regarde ce que", "vient de poster", "vu", "en train d'écrire", "nouveau message", "t'a identifié", "a réagi", "nouvelle story", "souvenirs", "tendances", "qui a regardé"
        )
        val urgentHooks = listOf(
            // English
            "urgent", "emergency", "hospital", "important", "now", "help",
            // French
            "urgence", "urgent", "hôpital", "important", "maintenant", "au secours", "aide", "grave", "accident"
        )
        val spamHooks = listOf(
            // English
            "sale", "promo", "buy now", "discount", "offer", "limited time",
            // French
            "soldes", "promo", "acheter", "offre", "réduction", "durée limitée", "gratuit", "profitez-en"
        )
        
        var isDistracting = false
        var intent = "INFORMATION"
        var toxicity = 0.0f
        
        for (hook in doomscrollHooks) {
            if (combined.contains(hook)) {
                isDistracting = true
                intent = "SOCIAL_HOOK"
                toxicity += 0.4f
            }
        }
        
        for (hook in spamHooks) {
            if (combined.contains(hook)) {
                isDistracting = true
                intent = "COMMERCIAL_SPAM"
                toxicity += 0.6f
            }
        }
        
        for (hook in urgentHooks) {
            if (combined.contains(hook)) {
                intent = "URGENT"
                toxicity = 0.0f // Urgent is necessary, not toxic
                isDistracting = false
            }
        }
        
        return SemanticAnalysis(
            toxicityScore = toxicity.coerceIn(0.0f, 1.0f),
            intent = intent,
            isDistracting = isDistracting
        )
    }
}
