package so.cynic.kyon.application.observability

import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.ports.ActivityEventStore

class RecordActivityEvent(
    private val eventStore: ActivityEventStore,
) {
    suspend fun execute(event: ActivityEvent) {
        eventStore.record(event)
    }
}
