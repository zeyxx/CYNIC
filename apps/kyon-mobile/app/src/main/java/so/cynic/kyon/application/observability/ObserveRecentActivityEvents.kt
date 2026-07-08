package so.cynic.kyon.application.observability

import kotlinx.coroutines.flow.Flow
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.ports.ActivityEventStore

class ObserveRecentActivityEvents(
    private val eventStore: ActivityEventStore,
) {
    fun execute(limit: Int = 100): Flow<List<ActivityEvent>> {
        return eventStore.observeRecent(limit)
    }
}

