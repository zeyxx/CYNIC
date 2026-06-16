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
import so.cynic.kyon.shield.reporting.PhoneReportDao
import so.cynic.kyon.shield.reporting.PhoneReportLabel
import java.security.MessageDigest
import java.util.Locale

class KyonCallScreeningService : CallScreeningService() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onScreenCall(callDetails: Call.Details) {
        val peerHash = hashPeer(callDetails.handle?.schemeSpecificPart.orEmpty())
        val app = application as? KyonApp
        if (app == null) {
            respondToCall(callDetails, allowDefaultCallResponse())
            return
        }

        val eventRepository = EventRepository(app.database.activityEvents())
        val reportDao: PhoneReportDao = app.database.phoneReports()

        scope.launch {
            val report = reportDao.findByHash(peerHash)
            val shouldBlock = shouldBlock(report?.label)

            eventRepository.record(
                ActivityEvent(
                    source = EventSource.MOBILE_CALL,
                    eventType = "incoming_call_screened",
                    target = "call_peer:$peerHash",
                    timestampMs = System.currentTimeMillis(),
                    privacy = EventPrivacy.METADATA_ONLY,
                    contextJson = """{"content_captured":false,"peer_hash":"$peerHash","decision":"${if (shouldBlock) "blocked_by_report" else "allow_default"}","report_label":"${report?.label ?: "NONE"}"}""",
                    syncEligible = false,
                ),
            )

            respondToCall(
                callDetails,
                if (shouldBlock) blockCallResponse() else allowDefaultCallResponse(),
            )
        }
    }

    private fun blockCallResponse() = CallResponse.Builder()
        .setDisallowCall(true)
        .setRejectCall(true)
        .setSkipCallLog(true)
        .setSkipNotification(false)
        .build()

    private fun allowDefaultCallResponse() = CallResponse.Builder()
        .setDisallowCall(false)
        .setRejectCall(false)
        .setSkipCallLog(false)
        .setSkipNotification(false)
        .build()

    private fun shouldBlock(label: String?): Boolean {
        val normalized = label ?: return false
        return try {
            when (PhoneReportLabel.valueOf(normalized)) {
                PhoneReportLabel.SCAM, PhoneReportLabel.NUISANCE -> true
                PhoneReportLabel.LEGITIMATE, PhoneReportLabel.IGNORE -> false
            }
        } catch (_: IllegalArgumentException) {
            false
        }
    }

    private fun hashPeer(peer: String): String {
        val normalized = peer.filter { !it.isWhitespace() }.lowercase(Locale.US)
        val input = normalized.ifBlank { "unknown" }
        val digest = MessageDigest.getInstance("SHA-256").digest(input.toByteArray())
        return digest.joinToString(separator = "") { "%02x".format(it) }.take(16)
    }
}
