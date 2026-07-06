package so.cynic.kyon.mirror.sync

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.Data
import androidx.work.WorkerParameters
import so.cynic.kyon.KyonApp

/**
 * Periodic background worker that flushes sync-eligible local events to the CYNIC kernel.
 *
 * Architecture:
 *  - Delegates syncing to application use case via [so.cynic.kyon.bootstrap.KyonContainer].
 *  - Reads connection params from [KyonApp.kernelConfig] through the composition root.
 *  - No-ops silently when sync is disabled; retries on partial failure.
 */
class KernelSyncWorker(
    appContext: Context,
    workerParams: WorkerParameters,
) : CoroutineWorker(appContext, workerParams) {

    override suspend fun doWork(): Result {
        val app = applicationContext as? KyonApp ?: return Result.failure()

        if (!app.featureFlags.syncEnabled || !app.kernelConfig.syncEnabled) {
            Log.d(TAG, "Kernel sync disabled — skipping")
            return Result.success(
                Data.Builder()
                    .putLong("last_sync_ms", System.currentTimeMillis())
                    .putInt("synced_count", 0)
                    .putInt("pending_count", 0)
                    .putString("result", "sync disabled")
                    .build(),
            )
        }

        val result = app.container.flushPendingEvents.execute(
            limit = 50,
            syncedAtMs = System.currentTimeMillis(),
        )

        Log.d(TAG, "Synced ${result.syncedCount}/${result.pendingCount} events")
        val resultPayload = Data.Builder()
            .putLong("last_sync_ms", System.currentTimeMillis())
            .putInt("synced_count", result.syncedCount)
            .putInt("pending_count", result.pendingCount)
            .putString("result", result.resultLabel)
            .build()

        return if (result.isComplete) Result.success(resultPayload) else Result.retry()
    }

    companion object {
        private const val TAG = "KernelSyncWorker"
    }
}
