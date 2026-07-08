package so.cynic.kyon.core.storage

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query

@Dao
interface OrganismStateDao {
    @Query("SELECT * FROM organism_state WHERE id = 1")
    suspend fun getHomeostasis(): OrganismStateEntity?

    @Query("SELECT * FROM organism_state WHERE id = 1")
    fun observeHomeostasis(): kotlinx.coroutines.flow.Flow<OrganismStateEntity?>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun updateHomeostasis(state: OrganismStateEntity)
}
