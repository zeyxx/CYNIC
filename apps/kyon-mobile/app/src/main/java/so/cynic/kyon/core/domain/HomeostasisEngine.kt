package so.cynic.kyon.core.domain

import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.OrganismStateDao
import so.cynic.kyon.core.storage.OrganismStateEntity
import kotlin.math.max
import kotlin.math.min

class HomeostasisEngine(private val stateDao: OrganismStateDao) {

    companion object {
        const val DECAY_RATE_PER_HOUR = 0.1f
        const val THRESHOLD_LOCKDOWN = 0.8f
        const val THRESHOLD_STRICT = 0.5f
    }
    
    suspend fun getCurrentState(): OrganismStateEntity? {
        return stateDao.getHomeostasis()
    }

    suspend fun processEvent(event: ActivityEvent) {
        val nowMs = System.currentTimeMillis()
        
        // 1. Retrieve current state or initialize baseline
        val currentState = stateDao.getHomeostasis() ?: OrganismStateEntity(
            focusPressure = 0.0f,
            shieldLevel = "PERMISSIVE",
            lastVerdict = null,
            updatedAtMs = nowMs
        )

        // 2. Apply chronological decay (Organism cools down over time)
        val hoursElapsed = (nowMs - currentState.updatedAtMs) / (1000f * 60f * 60f)
        var pressure = max(0.0f, currentState.focusPressure - (hoursElapsed * DECAY_RATE_PER_HOUR))

        // 3. Compute the metabolic impact of the new event
        val pressureDelta = calculatePressureDelta(event)
        pressure = min(1.0f, pressure + pressureDelta)

        // 4. Determine Shield reaction based on new pressure
        val newShieldLevel = when {
            pressure >= THRESHOLD_LOCKDOWN -> "LOCKDOWN"
            pressure >= THRESHOLD_STRICT -> "STRICT"
            else -> "PERMISSIVE"
        }

        // 5. Update SSoT locally
        val newState = currentState.copy(
            focusPressure = pressure,
            shieldLevel = newShieldLevel,
            updatedAtMs = nowMs
        )
        
        stateDao.updateHomeostasis(newState)
    }

    private fun calculatePressureDelta(event: ActivityEvent): Float {
        return when (event.source) {
            EventSource.MOBILE_NOTIFICATION -> 0.05f // Interruptions add pressure
            EventSource.MOBILE_CALL -> 0.10f         // Calls are highly disruptive
            EventSource.MOBILE_SMS -> 0.02f
            EventSource.MOBILE_USAGE -> {
                // Le calcul de la pression métabolique est désormais sous-traité au 
                // noyau cognitif Rust (kyon-core) pour des raisons de performance 
                // et d'unification avec le reste de l'écosystème CYNIC.
                val duration = event.durationMs ?: 0L
                KyonRustBridge.calculateSessionPressure(duration)
            }
            EventSource.USER_REPORT -> -0.2f // The user actively reclaiming control cools down the system
        }
    }
}
