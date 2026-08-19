package so.cynic.kyon.mirror.identity

import so.cynic.kyon.mirror.episodes.BehaviorEpisode

data class IdentitySnapshot(
    val generatedAtMs: Long,
    val topSurfaces: List<String>,
    val nightUsageEpisodes: Int,
    val confidence: Double,
)

class IdentitySnapshotBuilder {
    fun build(episodes: List<BehaviorEpisode>): IdentitySnapshot {
        val topSurfaces = episodes
            .sortedByDescending { it.eventCount }
            .take(5)
            .map { it.dominantSurface }

        return IdentitySnapshot(
            generatedAtMs = System.currentTimeMillis(),
            topSurfaces = topSurfaces,
            nightUsageEpisodes = 0,
            confidence = if (episodes.size >= 10) 0.3 else 0.1,
        )
    }
}
