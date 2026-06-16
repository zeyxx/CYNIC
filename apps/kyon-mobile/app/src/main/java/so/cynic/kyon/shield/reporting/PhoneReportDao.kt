package so.cynic.kyon.shield.reporting

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

@Dao
interface PhoneReportDao {

    /** Insert or replace — one label per hash, latest report wins. */
    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(report: PhoneReportEntity)

    @Query("SELECT * FROM phone_reports WHERE numberHash = :numberHash LIMIT 1")
    suspend fun findByHash(numberHash: String): PhoneReportEntity?

    @Query("SELECT * FROM phone_reports ORDER BY reportedAtMs DESC")
    fun observeAll(): Flow<List<PhoneReportEntity>>

    @Query("SELECT * FROM phone_reports WHERE label = :label ORDER BY reportedAtMs DESC")
    suspend fun byLabel(label: String): List<PhoneReportEntity>

    @Query("DELETE FROM phone_reports WHERE numberHash = :numberHash")
    suspend fun delete(numberHash: String)
}
