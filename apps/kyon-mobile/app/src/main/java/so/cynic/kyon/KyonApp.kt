package so.cynic.kyon

import android.app.Application
import android.content.Context
import androidx.room.Room
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import so.cynic.kyon.core.storage.KyonDatabase
import so.cynic.kyon.core.sync.KernelConfig
import so.cynic.kyon.core.sync.KernelObserveClient
import so.cynic.kyon.mirror.sync.KernelSyncWorker
import java.util.concurrent.TimeUnit

class KyonApp : Application() {

    lateinit var database: KyonDatabase
        private set

    /** Runtime kernel configuration — loaded from SharedPreferences, never hardcoded. */
    lateinit var kernelConfig: KernelConfig
        private set

    /** HTTP client wired to kernelConfig — the single sync authority. */
    lateinit var kernelClient: KernelObserveClient
        private set

    override fun onCreate() {
        super.onCreate()

        database = Room.databaseBuilder(
            applicationContext,
            KyonDatabase::class.java,
            "kyon.db",
        ).fallbackToDestructiveMigration().build()

        kernelConfig = loadKernelConfig()
        kernelClient = KernelObserveClient(kernelConfig)

        scheduleSyncWorker()
    }

    /** Reads kernel connection params from SharedPreferences.
     *  Sync is opt-in: [PREF_SYNC_ENABLED] defaults to false.
     *  URL and token are set by the user in Settings — never baked into the build. */
    private fun loadKernelConfig(): KernelConfig {
        val prefs = getSharedPreferences(PREFS_KERNEL, Context.MODE_PRIVATE)
        return KernelConfig(
            baseUrl = prefs.getString(PREF_KERNEL_URL, "") ?: "",
            bearerToken = prefs.getString(PREF_KERNEL_BEARER, "") ?: "",
            syncEnabled = prefs.getBoolean(PREF_SYNC_ENABLED, false),
        )
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
            syncRequest,
        )
    }

    companion object {
        const val PREFS_KERNEL = "kyon_kernel"
        const val PREF_KERNEL_URL = "kernel_url"
        const val PREF_KERNEL_BEARER = "kernel_token"
        const val PREF_SYNC_ENABLED = "sync_enabled"
    }
}
