package so.cynic.kyon.application.sync

import so.cynic.kyon.ports.ActivityEventStore
import so.cynic.kyon.ports.ObservationSink

class FlushPendingEvents(
    private val eventStore: ActivityEventStore,
    private val observationSink: ObservationSink,
) {
    suspend fun execute(limit: Int = 50, syncedAtMs: Long): SyncBatchResult {
        val pendingEvents = eventStore.pendingSync(limit)
        if (pendingEvents.isEmpty()) {
            return SyncBatchResult(syncedCount = 0, pendingCount = 0)
        }

        val syncedIds = buildList {
            for (event in pendingEvents) {
                val success = observationSink.post(event)
                if (success && event.id != null) {
                    add(event.id)
                }
            }
        }

        if (syncedIds.isNotEmpty()) {
            eventStore.markSynced(syncedIds, syncedAtMs)
        }

        return SyncBatchResult(
            syncedCount = syncedIds.size,
            pendingCount = pendingEvents.size,
        )
    }
}
