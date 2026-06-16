package so.cynic.kyon.mirror.identity

import so.cynic.kyon.mirror.episodes.BehaviorEpisode
import java.util.Calendar

data class IdentitySnapshot(
    val generatedAtMs: Long,
    val topSurfaces: List<String>,
    val nightUsageEpisodes: Int,
    val confidence: Double,
)

class IdentitySnapshotBuilder {
    fun build(episodes: List<BehaviorEpisode>): IdentitySnapshot {
        val sortedByDuration = episodes
            .sortedByDescending { it.totalDurationMs }

        val topSurfaces = sortedByDuration
            .take(5)
            .map { it.dominantSurface }
            .ifEmpty { listOf("aucun signal") }

        val totalDurationMs = episodes.sumOf { it.totalDurationMs }
        val nightUsageEpisodes = episodes.count { episode ->
            isNightHour(episode.startedAtMs) || isNightHour(episode.endedAtMs)
        }
        val confidence = when {
            totalDurationMs >= 6 * 60L * 60L * 1000L -> 0.92
            totalDurationMs >= 60L * 60L * 1000L -> 0.7
            totalDurationMs >= 10L * 60L * 1000L -> 0.4
            episodes.isNotEmpty() -> 0.2
            else -> 0.0
        }

        return IdentitySnapshot(
            generatedAtMs = System.currentTimeMillis(),
            topSurfaces = topSurfaces,
            nightUsageEpisodes = nightUsageEpisodes,
            confidence = confidence,
        )
    }

    private fun isNightHour(timestampMs: Long): Boolean {
        val hour = Calendar.getInstance().apply { timeInMillis = timestampMs }.get(Calendar.HOUR_OF_DAY)
        return hour < 6 || hour >= 22
    }
}
