package so.cynic.kyon.mirror.usage

import android.app.usage.UsageStatsManager
import android.content.Context
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventPrivacy
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.sensing.Sensor

class UsageSnapshotCollector(private val context: Context) : Sensor {
    override suspend fun snapshot(): List<ActivityEvent> {
        val manager = context.getSystemService(Context.USAGE_STATS_SERVICE) as UsageStatsManager
        val now = System.currentTimeMillis()
        val stats = manager.queryUsageStats(
            UsageStatsManager.INTERVAL_DAILY,
            now - LOOKBACK_MS,
            now,
        )

        return stats
            .filter { it.totalTimeInForeground > 0 }
            .map {
                ActivityEvent(
                    source = EventSource.MOBILE_USAGE,
                    eventType = "app_foreground_daily",
                    target = it.packageName,
                    timestampMs = now,
                    durationMs = it.totalTimeInForeground,
                    privacy = EventPrivacy.METADATA_ONLY,
                    contextJson = """{"content_captured":false}""",
                    syncEligible = true,
                )
            }
    }

    private companion object {
        const val LOOKBACK_MS = 24L * 60L * 60L * 1000L
    }
}
