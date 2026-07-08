package so.cynic.kyon.mirror.episodes

import org.junit.Assert.assertEquals
import org.junit.Test
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource

class EpisodeBuilderTest {
    private val builder = EpisodeBuilder()

    @Test
    fun groupsUsageEventsBySurfaceAndSortsByDuration() {
        val episodes = builder.from(
            listOf(
                usage("app.a", timestampMs = 1_000, durationMs = 10),
                usage("app.b", timestampMs = 2_000, durationMs = 90),
                usage("app.a", timestampMs = 3_000, durationMs = 40),
                notification("app.b", timestampMs = 4_000),
            ),
        )

        assertEquals(2, episodes.size)
        assertEquals("app.b", episodes[0].dominantSurface)
        assertEquals(90, episodes[0].totalDurationMs)
        assertEquals("app.a", episodes[1].dominantSurface)
        assertEquals(50, episodes[1].totalDurationMs)
        assertEquals(1_000, episodes[1].startedAtMs)
        assertEquals(3_000, episodes[1].endedAtMs)
        assertEquals(2, episodes[1].eventCount)
    }

    @Test
    fun ignoresNonUsageEvents() {
        val episodes = builder.from(
            listOf(
                notification("app.a", timestampMs = 1_000),
                ActivityEvent(
                    source = EventSource.MOBILE_SMS,
                    eventType = "sms_received",
                    target = "sms_sender:hash",
                    timestampMs = 2_000,
                ),
            ),
        )

        assertEquals(emptyList<BehaviorEpisode>(), episodes)
    }

    private fun usage(target: String, timestampMs: Long, durationMs: Long) = ActivityEvent(
        source = EventSource.MOBILE_USAGE,
        eventType = "app_foreground_daily",
        target = target,
        timestampMs = timestampMs,
        durationMs = durationMs,
    )

    private fun notification(target: String, timestampMs: Long) = ActivityEvent(
        source = EventSource.MOBILE_NOTIFICATION,
        eventType = "notification_posted",
        target = target,
        timestampMs = timestampMs,
    )
}
