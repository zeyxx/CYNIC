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
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.EventRepository

class KyonSmsReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Telephony.Sms.Intents.SMS_RECEIVED_ACTION) return

        val app = context.applicationContext as KyonApp
        val repository = EventRepository(app.database.activityEvents())
        val scope = CoroutineScope(Dispatchers.IO)

        Telephony.Sms.Intents.getMessagesFromIntent(intent).forEach { message ->
            scope.launch {
                repository.record(
                    ActivityEvent(
                        source = EventSource.MOBILE_SMS,
                        eventType = "sms_received",
                        target = message.originatingAddress.orEmpty(),
                        timestampMs = System.currentTimeMillis(),
                        contextJson = """{"content_captured":false}""",
                        syncEligible = false,
                    ),
                )
            }
        }
    }
}
