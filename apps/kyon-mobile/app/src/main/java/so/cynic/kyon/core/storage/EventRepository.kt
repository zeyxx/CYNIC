package so.cynic.kyon.core.storage

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import so.cynic.kyon.core.model.ActivityEvent

import java.security.MessageDigest

class EventRepository(private val dao: ActivityEventDao) {

    suspend fun record(event: ActivityEvent) {
        // 1. Fetch the prevHash
        val prevHash = dao.getLastHash() ?: "0".repeat(128)
        
        // 2. Compute the new cryptographicHash (SHA-512 of payload + prevHash)
        val payload = "${event.timestampMs}_${event.source.name}_${event.eventType}_${event.target}"
        val md = MessageDigest.getInstance("SHA-512")
        md.update(payload.toByteArray())
        md.update(prevHash.toByteArray())
        
        val hashResult = md.digest()
        val hashHex = hashResult.joinToString("") { "%02x".format(it) }
        
        // 3. Update the model and write to the immutable local ledger
        val securedEvent = event.copy(
            prevHash = prevHash,
            cryptographicHash = hashHex
        )
        
        dao.insert(securedEvent.toEntity())
    }

    suspend fun recent(limit: Int = 100): List<ActivityEvent> =
        dao.recent(limit).map { it.toModel() }

    fun observeRecent(limit: Int = 100): Flow<List<ActivityEvent>> =
        dao.observeRecent(limit).map { events -> events.map { it.toModel() } }

    suspend fun pendingSync(limit: Int = 50): List<ActivityEvent> =
        dao.pendingSync(limit).map { it.toModel() }

    suspend fun markSynced(ids: List<Long>, syncedAtMs: Long) =
        dao.markSynced(ids, syncedAtMs)
}
