package so.cynic.kyon.application.mirror

import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.ports.ActivityEventStore
import so.cynic.kyon.ports.LocalIntelligencePort

class RecordNotificationEvent(
    private val eventStore: ActivityEventStore,
    private val intelligencePort: LocalIntelligencePort,
) {
    suspend fun execute(
        packageName: String,
        postedAtMs: Long,
        isOngoing: Boolean,
        notificationId: Int,
        title: String,
        text: String,
    ) {
        val analysis = intelligencePort.analyzeText(title, text)
        
        // Transform the semantic output into contextJson for the ActivityEvent
        val contextJson = """{
            "content_captured":true,
            "is_ongoing":$isOngoing,
            "id":$notificationId,
            "intent":"${analysis.intent}",
            "toxicity":${analysis.toxicityScore},
            "is_distracting":${analysis.isDistracting}
        }""".trimIndent().replace("\n", "")

        val eventType = if (analysis.isDistracting) "notification_distraction" else "notification_posted"

        eventStore.record(
            ActivityEvent(
                source = EventSource.MOBILE_NOTIFICATION,
                eventType = eventType,
                target = packageName,
                timestampMs = postedAtMs,
                contextJson = contextJson,
                syncEligible = false,
            ),
        )
    }
}
