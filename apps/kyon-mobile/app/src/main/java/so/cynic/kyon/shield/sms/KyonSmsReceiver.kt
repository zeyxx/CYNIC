package so.cynic.kyon.shield.sms

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.provider.Telephony
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventPrivacy
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.EventRepository
import java.security.MessageDigest
import java.util.Locale

class KyonSmsReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Telephony.Sms.Intents.SMS_RECEIVED_ACTION) return

        val messages = Telephony.Sms.Intents.getMessagesFromIntent(intent)
        if (messages.isEmpty()) return

        val app = context.applicationContext as KyonApp
        val repository = EventRepository(app.database.activityEvents())
        val scope = CoroutineScope(Dispatchers.IO)
        val senderHash = hashSender(messages.firstOrNull()?.originatingAddress.orEmpty())
        val timestampMs = messages.minOfOrNull { it.timestampMillis }.takeIf { it != 0L }
            ?: System.currentTimeMillis()

        scope.launch {
            repository.record(
                ActivityEvent(
                    source = EventSource.MOBILE_SMS,
                    eventType = "sms_received",
                    target = "sms_sender:$senderHash",
                    timestampMs = timestampMs,
                    privacy = EventPrivacy.METADATA_ONLY,
                    contextJson = """{"content_captured":false,"sender_hash":"$senderHash","parts":${messages.size}}""",
                    syncEligible = true,
                ),
            )
        }
    }

    private fun hashSender(sender: String): String {
        val normalized = sender.filter { !it.isWhitespace() }.lowercase(Locale.US)
        val input = normalized.ifBlank { "unknown" }
        val digest = MessageDigest.getInstance("SHA-256").digest(input.toByteArray())
        return digest.joinToString(separator = "") { "%02x".format(it) }.take(16)
    }
}
