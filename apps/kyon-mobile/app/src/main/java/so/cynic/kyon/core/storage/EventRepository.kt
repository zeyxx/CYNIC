package so.cynic.kyon.core.storage

import so.cynic.kyon.core.model.ActivityEvent

class EventRepository(private val dao: ActivityEventDao) {
    suspend fun record(event: ActivityEvent) {
        dao.insert(event.toEntity())
    }

    suspend fun recent(limit: Int = 100): List<ActivityEvent> =
        dao.recent(limit).map { it.toModel() }
}
