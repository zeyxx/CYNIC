package so.cynic.kyon.mirror.sync

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import org.json.JSONObject
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.storage.EventRepository
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL

class KernelSyncWorker(
    appContext: Context,
    workerParams: WorkerParameters
) : CoroutineWorker(appContext, workerParams) {

    override suspend fun doWork(): Result {
        val app = applicationContext as? KyonApp ?: return Result.failure()
        val repository = EventRepository(app.database.activityEvents())
        
        val pendingEvents = repository.pendingSync(limit = 50)
        if (pendingEvents.isEmpty()) {
            return Result.success()
        }
        
        // Use a dummy address for now. Ideally this would be retrieved from SharedPreferences or Config
        val kernelUrl = "http://10.0.2.2:3030/api/rest/observe"
        val syncedIds = mutableListOf<Long>()
        
        for (event in pendingEvents) {
            val success = sendObservation(kernelUrl, event)
            if (success && event.id != null) {
                syncedIds.add(event.id)
            }
        }
        
        if (syncedIds.isNotEmpty()) {
            repository.markSynced(syncedIds, System.currentTimeMillis())
        }
        
        return if (syncedIds.size == pendingEvents.size) Result.success() else Result.retry()
    }
    
    private fun sendObservation(urlString: String, event: ActivityEvent): Boolean {
        return try {
            val url = URL(urlString)
            val conn = url.openConnection() as HttpURLConnection
            conn.requestMethod = "POST"
            conn.setRequestProperty("Content-Type", "application/json")
            conn.setRequestProperty("Authorization", "Bearer dev_key") // Mock key for local dev
            conn.doOutput = true
            
            val payload = JSONObject().apply {
                put("tool", event.eventType)
                put("target", event.target)
                put("domain", "mobile")
                put("status", "observed")
                put("context", event.contextJson)
                put("project", "CYNIC")
                put("agent_id", "kyon-mobile")
            }
            
            OutputStreamWriter(conn.outputStream).use { it.write(payload.toString()) }
            
            val responseCode = conn.responseCode
            conn.disconnect()
            
            responseCode in 200..299
        } catch (e: Exception) {
            Log.e("KernelSyncWorker", "Failed to send observation", e)
            false
        }
    }
}
