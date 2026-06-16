package so.cynic.kyon.mirror.episodes

import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource

data class BehaviorEpisode(
    val startedAtMs: Long,
    val endedAtMs: Long,
    val dominantSurface: String,
    val eventCount: Int,
    val totalDurationMs: Long,
)

class EpisodeBuilder {
    /**
     * Groups usage events by package name and produces one [BehaviorEpisode] per surface.
     * [BehaviorEpisode.totalDurationMs] is the sum of all [ActivityEvent.durationMs] for that
     * surface — this is what the usage stats API actually reports.
     */
    fun from(events: List<ActivityEvent>): List<BehaviorEpisode> {
        return events
            .filter { it.source == EventSource.MOBILE_USAGE }
            .groupBy { it.target }
            .mapNotNull { (target, grouped) ->
                val timestamps = grouped.map { it.timestampMs }
                BehaviorEpisode(
                    startedAtMs = timestamps.minOrNull() ?: return@mapNotNull null,
                    endedAtMs = timestamps.maxOrNull() ?: return@mapNotNull null,
                    dominantSurface = target,
                    eventCount = grouped.size,
                    totalDurationMs = grouped.sumOf { it.durationMs ?: 0L },
                )
            }
            .sortedByDescending { it.totalDurationMs }
    }
}
