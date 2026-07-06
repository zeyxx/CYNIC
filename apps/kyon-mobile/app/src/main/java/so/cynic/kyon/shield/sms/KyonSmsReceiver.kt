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

class KyonSmsReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Telephony.Sms.Intents.SMS_RECEIVED_ACTION) return

        val messages = Telephony.Sms.Intents.getMessagesFromIntent(intent)
        if (messages.isEmpty()) return

        val app = context.applicationContext as? KyonApp ?: return
        if (!app.featureFlags.shieldEnabled) {
            return
        }

        val decisionUseCase = app.container.shieldDecision
        val recordActivityEvent = app.container.recordActivityEvent
        val scope = CoroutineScope(Dispatchers.IO)
        val timestampMs = messages.minOfOrNull { it.timestampMillis }.takeIf { it != 0L }
            ?: System.currentTimeMillis()
        val sender = messages.firstOrNull()?.originatingAddress.orEmpty()

        scope.launch {
            val decision = decisionUseCase.execute(sender)
            recordActivityEvent.execute(
                ActivityEvent(
                    source = EventSource.MOBILE_SMS,
                    eventType = "sms_received",
                    target = "sms_sender:${decision.peerHash}",
                    timestampMs = timestampMs,
                    privacy = EventPrivacy.METADATA_ONLY,
                    contextJson = """{"content_captured":false,"sender_hash":"${decision.peerHash}","parts":${messages.size},"blocked_by_report":${decision.shouldBlock}}""",
                    syncEligible = false,
                ),
            )
        }
    }
}
