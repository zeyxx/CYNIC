package so.cynic.kyon.application.observability

import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.ports.ActivityEventStore

enum class ExportFormat {
    JSON,
    CSV,
}

data class ExportPayload(
    val mimeType: String,
    val body: String,
)

class ExportActivityEvents(
    private val eventStore: ActivityEventStore,
) {
    suspend fun execute(limit: Int = 100, format: ExportFormat): ExportPayload {
        val events = eventStore.recent(limit)
        return when (format) {
            ExportFormat.JSON -> ExportPayload(
                mimeType = "application/json",
                body = exportJson(events),
            )

            ExportFormat.CSV -> ExportPayload(
                mimeType = "text/csv",
                body = exportCsv(events),
            )
        }
    }

    private fun exportJson(events: List<ActivityEvent>): String = buildString {
        append("{")
        append("\"exportedAtMs\":${System.currentTimeMillis()},")
        append("\"count\":${events.size},")
        append("\"items\":[")
        events.forEachIndexed { index, event ->
            if (index > 0) append(",")
            append(jsonEvent(event))
        }
        append("]}")
    }

    private fun jsonEvent(event: ActivityEvent): String = buildString {
        append("{")
        append("\"id\":")
        append(event.id ?: "null")
        append(",\"source\":\"")
        append(escapeJson(event.source.name))
        append("\",\"eventType\":\"")
        append(escapeJson(event.eventType))
        append("\",\"target\":\"")
        append(escapeJson(event.target))
        append("\",\"timestampMs\":")
        append(event.timestampMs)
        append(",\"durationMs\":")
        append(event.durationMs ?: "null")
        append(",\"privacy\":\"")
        append(event.privacy.name)
        append("\",\"syncEligible\":")
        append(event.syncEligible)
        append(",\"contextJson\":\"")
        append(escapeJson(event.contextJson))
        append("\"}")
    }

    private fun exportCsv(events: List<ActivityEvent>): String = buildString {
        append("id,source,eventType,target,timestamp_ms,duration_ms,privacy,sync_eligible,context_json\n")
        events.forEach { event ->
            append(csvValue(event.id))
            append(",")
            append(csvValue(event.source))
            append(",")
            append(csvValue(event.eventType))
            append(",")
            append(csvValue(event.target))
            append(",")
            append(csvValue(event.timestampMs))
            append(",")
            append(csvValue(event.durationMs))
            append(",")
            append(csvValue(event.privacy))
            append(",")
            append(csvValue(event.syncEligible))
            append(",")
            append(csvValue(event.contextJson))
            append('\n')
        }
    }

    private fun csvValue(value: Any?): String = buildString {
        val raw = value?.toString().orEmpty()
        val escaped = raw.replace("\"", "\"\"")
        append("\"")
        append(escaped)
        append("\"")
    }

    private fun escapeJson(value: String): String = value
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}
