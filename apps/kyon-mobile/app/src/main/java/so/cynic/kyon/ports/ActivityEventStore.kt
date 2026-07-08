package so.cynic.kyon.ports

import kotlinx.coroutines.flow.Flow
import so.cynic.kyon.core.model.ActivityEvent

interface ActivityEventStore {
    suspend fun record(event: ActivityEvent)

    suspend fun recent(limit: Int = 100): List<ActivityEvent>

    fun observeRecent(limit: Int = 100): Flow<List<ActivityEvent>>

    suspend fun pendingSync(limit: Int = 50): List<ActivityEvent>

    suspend fun markSynced(ids: List<Long>, syncedAtMs: Long)
}
