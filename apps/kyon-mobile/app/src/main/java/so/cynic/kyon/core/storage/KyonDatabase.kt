package so.cynic.kyon.core.storage

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(
    entities = [ActivityEventEntity::class],
    version = 1,
    exportSchema = false,
)
abstract class KyonDatabase : RoomDatabase() {
    abstract fun activityEvents(): ActivityEventDao
}
