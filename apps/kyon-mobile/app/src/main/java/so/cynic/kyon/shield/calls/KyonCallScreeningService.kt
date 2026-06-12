package so.cynic.kyon.shield.calls

import android.telecom.Call
import android.telecom.CallScreeningService
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

class KyonCallScreeningService : CallScreeningService() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onScreenCall(callDetails: Call.Details) {
        val peerHash = hashPeer(callDetails.handle?.schemeSpecificPart.orEmpty())
        val repository = EventRepository((application as KyonApp).database.activityEvents())

        scope.launch {
            repository.record(
                ActivityEvent(
                    source = EventSource.MOBILE_CALL,
                    eventType = "incoming_call_screened",
                    target = "call_peer:$peerHash",
                    timestampMs = System.currentTimeMillis(),
                    privacy = EventPrivacy.METADATA_ONLY,
                    contextJson = """{"content_captured":false,"peer_hash":"$peerHash","decision":"allow_default"}""",
                    syncEligible = true,
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

    private fun hashPeer(peer: String): String {
        val normalized = peer.filter { !it.isWhitespace() }.lowercase(Locale.US)
        val input = normalized.ifBlank { "unknown" }
        val digest = MessageDigest.getInstance("SHA-256").digest(input.toByteArray())
        return digest.joinToString(separator = "") { "%02x".format(it) }.take(16)
    }
}
