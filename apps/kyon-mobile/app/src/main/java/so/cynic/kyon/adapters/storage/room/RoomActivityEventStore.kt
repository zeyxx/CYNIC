package so.cynic.kyon.adapters.storage.room

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.storage.ActivityEventDao
import so.cynic.kyon.core.storage.toEntity
import so.cynic.kyon.ports.ActivityEventStore

import so.cynic.kyon.core.domain.HomeostasisEngine
import java.security.MessageDigest

class RoomActivityEventStore(
    private val dao: ActivityEventDao,
    private val homeostasisEngine: HomeostasisEngine,
) : ActivityEventStore {
    override suspend fun record(event: ActivityEvent) {
        // Proof-of-History Ledger
        val prevHash = dao.getLastHash() ?: "0".repeat(128)
        val payload = "${event.timestampMs}_${event.source.name}_${event.eventType}_${event.target}"
        val md = MessageDigest.getInstance("SHA-512")
        md.update(payload.toByteArray())
        md.update(prevHash.toByteArray())
        val hashResult = md.digest()
        val hashHex = hashResult.joinToString("") { "%02x".format(it) }
        
        val securedEvent = event.copy(prevHash = prevHash, cryptographicHash = hashHex)
        dao.insert(securedEvent.toEntity())
        
        // Feed the Organism's Metabolism
        homeostasisEngine.processEvent(securedEvent)
    }

    override suspend fun recent(limit: Int): List<ActivityEvent> =
        dao.recent(limit).map { it.toModel() }

    override fun observeRecent(limit: Int): Flow<List<ActivityEvent>> =
        dao.observeRecent(limit).map { events -> events.map { it.toModel() } }

    override suspend fun pendingSync(limit: Int): List<ActivityEvent> =
        dao.pendingSync(limit).map { it.toModel() }

    override suspend fun markSynced(ids: List<Long>, syncedAtMs: Long) {
        dao.markSynced(ids, syncedAtMs)
    }
}
