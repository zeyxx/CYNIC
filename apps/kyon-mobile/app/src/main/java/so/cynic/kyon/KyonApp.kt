package so.cynic.kyon

import android.app.Application
import androidx.room.Room
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import so.cynic.kyon.core.storage.KyonDatabase
import so.cynic.kyon.mirror.sync.KernelSyncWorker
import java.util.concurrent.TimeUnit

class KyonApp : Application() {
    lateinit var database: KyonDatabase
        private set

    override fun onCreate() {
        super.onCreate()
        database = Room.databaseBuilder(
            applicationContext,
            KyonDatabase::class.java,
            "kyon.db",
        ).build()

        scheduleSyncWorker()
    }

    private fun scheduleSyncWorker() {
        val constraints = Constraints.Builder()
            .setRequiredNetworkType(NetworkType.CONNECTED)
            .build()

        val syncRequest = PeriodicWorkRequestBuilder<KernelSyncWorker>(15, TimeUnit.MINUTES)
            .setConstraints(constraints)
            .build()

        WorkManager.getInstance(this).enqueueUniquePeriodicWork(
            "KernelSyncWork",
            ExistingPeriodicWorkPolicy.KEEP,
            syncRequest
        )
    }
}
