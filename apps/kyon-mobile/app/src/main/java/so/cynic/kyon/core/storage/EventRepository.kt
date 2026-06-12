package so.cynic.kyon.core.storage

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import so.cynic.kyon.core.model.ActivityEvent

class EventRepository(private val dao: ActivityEventDao) {
    suspend fun record(event: ActivityEvent) {
        dao.insert(event.toStorageEntity())
    }

    suspend fun recent(limit: Int = 100): List<ActivityEvent> =
        dao.recent(limit).map { it.toModel() }

    fun observeRecent(limit: Int = 100): Flow<List<ActivityEvent>> =
        dao.observeRecent(limit).map { events -> events.map { it.toModel() } }

    suspend fun pendingSync(limit: Int = 50): List<ActivityEvent> =
        dao.pendingSync(limit).map { it.toModel() }

    suspend fun markSynced(ids: List<Long>, syncedAtMs: Long) {
        dao.markSynced(ids, syncedAtMs)
    }
}

private fun ActivityEvent.toStorageEntity(): ActivityEventEntity = ActivityEventEntity(
    source = source.name,
    eventType = eventType,
    target = target,
    timestampMs = timestampMs,
    durationMs = durationMs,
    privacy = privacy.name,
    contextJson = contextJson,
    syncEligible = syncEligible,
    syncedAtMs = null,
)
