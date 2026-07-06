package so.cynic.kyon.core.storage

import androidx.room.Database
import androidx.room.RoomDatabase
import so.cynic.kyon.shield.reporting.PhoneReportDao
import so.cynic.kyon.shield.reporting.PhoneReportEntity

@Database(
    entities = [
        ActivityEventEntity::class,
        PhoneReportEntity::class,
        OrganismStateEntity::class,
    ],
    version = 3,
    exportSchema = false,
)
abstract class KyonDatabase : RoomDatabase() {
    abstract fun activityEvents(): ActivityEventDao
    abstract fun phoneReports(): PhoneReportDao
    abstract fun organismState(): OrganismStateDao
}
