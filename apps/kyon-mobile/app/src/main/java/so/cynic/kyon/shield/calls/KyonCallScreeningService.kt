package so.cynic.kyon.shield.calls

import android.telecom.Call
import android.telecom.CallScreeningService
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.EventRepository

class KyonCallScreeningService : CallScreeningService() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onScreenCall(callDetails: Call.Details) {
        val number = callDetails.handle?.schemeSpecificPart.orEmpty()
        val repository = EventRepository((application as KyonApp).database.activityEvents())

        scope.launch {
            repository.record(
                ActivityEvent(
                    source = EventSource.MOBILE_CALL,
                    eventType = "incoming_call_screened",
                    target = number,
                    timestampMs = System.currentTimeMillis(),
                    contextJson = """{"content_captured":false,"decision":"allow_default"}""",
                    syncEligible = false,
                ),
            )
        }

        respondToCall(callDetails, CallResponse.Builder()
            .setDisallowCall(false)
            .setRejectCall(false)
            .setSkipCallLog(false)
            .setSkipNotification(false)
            .build())
    }
}
