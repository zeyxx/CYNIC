package so.cynic.kyon.application.observability

import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.ports.ActivityEventStore

class LoadRecentActivityEvents(
    private val eventStore: ActivityEventStore,
) {
    suspend fun execute(limit: Int = 100): List<ActivityEvent> {
        return eventStore.recent(limit)
    }
}

