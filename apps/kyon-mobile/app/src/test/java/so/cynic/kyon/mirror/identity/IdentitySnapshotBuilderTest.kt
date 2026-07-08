package so.cynic.kyon.mirror.identity

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import so.cynic.kyon.mirror.episodes.BehaviorEpisode
import java.util.Calendar
import java.util.TimeZone

class IdentitySnapshotBuilderTest {
    private val builder = IdentitySnapshotBuilder()

    @Test
    fun emptyEpisodesProduceLowConfidenceFallback() {
        val snapshot = builder.build(emptyList())

        assertEquals(listOf("aucun signal"), snapshot.topSurfaces)
        assertEquals(0, snapshot.nightUsageEpisodes)
        assertEquals(0.0, snapshot.confidence, 0.0)
    }

    @Test
    fun ranksTopSurfacesAndComputesConfidenceBuckets() {
        val snapshot = builder.build(
            listOf(
                episode("app.low", durationMs = 10 * MINUTE),
                episode("app.high", durationMs = 50 * MINUTE),
            ),
        )

        assertEquals(listOf("app.high", "app.low"), snapshot.topSurfaces)
        assertEquals(0.7, snapshot.confidence, 0.0)
        assertTrue(snapshot.generatedAtMs > 0)
    }

    @Test
    fun countsNightEpisodes() {
        val snapshot = builder.build(
            listOf(
                episode("app.day", startedAtMs = atHour(14), endedAtMs = atHour(15)),
                episode("app.night", startedAtMs = atHour(23), endedAtMs = atHour(23)),
            ),
        )

        assertEquals(1, snapshot.nightUsageEpisodes)
    }

    private fun episode(
        surface: String,
        startedAtMs: Long = atHour(12),
        endedAtMs: Long = atHour(13),
        durationMs: Long = 30 * MINUTE,
    ) = BehaviorEpisode(
        startedAtMs = startedAtMs,
        endedAtMs = endedAtMs,
        dominantSurface = surface,
        eventCount = 1,
        totalDurationMs = durationMs,
    )

    private fun atHour(hour: Int): Long =
        Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(2026, Calendar.JUNE, 24, hour, 0, 0)
        }.timeInMillis

    private companion object {
        const val MINUTE = 60L * 1000L
    }
}
