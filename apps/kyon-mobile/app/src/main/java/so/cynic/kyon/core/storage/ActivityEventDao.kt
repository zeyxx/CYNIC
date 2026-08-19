package so.cynic.kyon.core.storage

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

@Dao
interface ActivityEventDao {
    @Insert
    suspend fun insert(event: ActivityEventEntity)

    @Query("SELECT * FROM activity_events ORDER BY timestampMs DESC LIMIT :limit")
    suspend fun recent(limit: Int): List<ActivityEventEntity>

    @Query("SELECT * FROM activity_events ORDER BY timestampMs DESC LIMIT :limit")
    fun observeRecent(limit: Int): Flow<List<ActivityEventEntity>>

    @Query(
        "SELECT * FROM activity_events " +
            "WHERE syncEligible = 1 AND syncedAtMs IS NULL " +
            "ORDER BY timestampMs ASC LIMIT :limit",
    )
    suspend fun pendingSync(limit: Int): List<ActivityEventEntity>

    @Query("UPDATE activity_events SET syncedAtMs = :syncedAtMs WHERE id IN (:ids)")
    suspend fun markSynced(ids: List<Long>, syncedAtMs: Long)
}
