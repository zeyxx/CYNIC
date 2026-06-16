package so.cynic.kyon.mirror.sync

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.storage.EventRepository

/**
 * Periodic background worker that flushes sync-eligible local events to the CYNIC kernel.
 *
 * Architecture:
 *  - Delegates all HTTP to [so.cynic.kyon.core.sync.KernelObserveClient] (single sync authority).
 *  - Reads connection params from [KyonApp.kernelConfig] — never hardcoded.
 *  - No-ops silently when sync is disabled; retries on partial failure.
 */
class KernelSyncWorker(
    appContext: Context,
    workerParams: WorkerParameters,
) : CoroutineWorker(appContext, workerParams) {

    override suspend fun doWork(): Result {
        val app = applicationContext as? KyonApp ?: return Result.failure()

        if (!app.kernelConfig.syncEnabled) {
            Log.d(TAG, "Kernel sync disabled — skipping")
            return Result.success()
        }

        val repository = EventRepository(app.database.activityEvents())
        val client = app.kernelClient

        val pendingEvents = repository.pendingSync(limit = 50)
        if (pendingEvents.isEmpty()) {
            Log.d(TAG, "No pending events — nothing to sync")
            return Result.success()
        }

        val syncedIds = mutableListOf<Long>()

        withContext(Dispatchers.IO) {
            for (event in pendingEvents) {
                val success = client.postEvent(event)
                if (success && event.id != null) {
                    syncedIds.add(event.id)
                }
            }
        }

        if (syncedIds.isNotEmpty()) {
            repository.markSynced(syncedIds, System.currentTimeMillis())
        }

        Log.d(TAG, "Synced ${syncedIds.size}/${pendingEvents.size} events")
        return if (syncedIds.size == pendingEvents.size) Result.success() else Result.retry()
    }

    companion object {
        private const val TAG = "KernelSyncWorker"
    }
}
