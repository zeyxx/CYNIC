package so.cynic.kyon.adapters.sync.kernel

import org.json.JSONObject
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.sync.KernelConfig
import so.cynic.kyon.ports.ObservationSink
import java.net.HttpURLConnection
import java.net.URL

class KernelObservationSink(
    private val config: KernelConfig,
) : ObservationSink {
    override suspend fun post(event: ActivityEvent): Boolean {
        if (!config.syncEnabled) return false

        val payload = JSONObject()
            .put("domain", domainFor(event))
            .put("target", event.target)
            .put("tool", "kyon_mobile")
            .put("agent_id", "kyon-device-local")
            .put("content", event.eventType)
            .put("context", JSONObject(event.contextJson))

        val connection = URL("${config.baseUrl.trimEnd('/')}/observe").openConnection()
            as HttpURLConnection
        return try {
            connection.requestMethod = "POST"
            connection.setRequestProperty("Authorization", "Bearer ${config.bearerToken}")
            connection.setRequestProperty("Content-Type", "application/json")
            connection.doOutput = true
            connection.outputStream.use { it.write(payload.toString().toByteArray()) }
            connection.responseCode in 200..299
        } catch (_: Exception) {
            false
        } finally {
            connection.disconnect()
        }
    }

    private fun domainFor(event: ActivityEvent): String =
        when (event.source) {
            EventSource.MOBILE_CALL, EventSource.MOBILE_SMS -> "phone-number"
            else -> "personal-activity"
        }
}
