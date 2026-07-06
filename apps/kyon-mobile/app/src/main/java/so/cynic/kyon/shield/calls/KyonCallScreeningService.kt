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
import so.cynic.kyon.shield.reporting.PhoneReportLabel

import android.os.PowerManager
import android.content.Context
import java.util.Calendar

class KyonCallScreeningService : CallScreeningService() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onScreenCall(callDetails: Call.Details) {
        val app = application as? KyonApp
        if (app == null) {
            respondToCall(callDetails, allowDefaultCallResponse())
            return
        }

        if (!app.featureFlags.shieldEnabled) {
            respondToCall(callDetails, allowDefaultCallResponse())
            return
        }

        val peer = callDetails.handle?.schemeSpecificPart.orEmpty()
        val useCase = app.container.shieldDecision
        val homeostasisEngine = app.container.homeostasisEngine

        // --- Context & Physical Reality ---
        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        val isScreenOn = powerManager.isInteractive

        scope.launch {
            val currentState = homeostasisEngine.getCurrentState()
            val pressure = currentState?.focusPressure ?: 0.0f
            val isLockdown = currentState?.shieldLevel == "LOCKDOWN"
            val isStrict = currentState?.shieldLevel == "STRICT"

            var blockOverride = false
            var overrideReason = ""

            // 1. Reality-based Heuristics (No Assumptions)
            // We rely purely on the real-time physical state of the device (isScreenOn) 
            // and the biological state of the organism (Pressure).
            if (isLockdown && isScreenOn) {
                // The human is overloaded (LOCKDOWN) but actively engaged with the device.
                // An interruption here breaks an already fragile cognitive state.
                blockOverride = true
                overrideReason = "lockdown_active_usage"
            } else if (isLockdown && !isScreenOn) {
                // The human is overloaded, but the device is off. 
                // They are likely attempting to recover (resting, sleeping).
                blockOverride = true
                overrideReason = "lockdown_recovery_protection"
            } else if (isStrict && isScreenOn) {
                // The human is not yet overloaded, but pressure is high (STRICT) and they are active.
                // Protect their real-time flow state without assuming *what* time of day it is.
                blockOverride = true
                overrideReason = "strict_flow_protection"
            }

            if (blockOverride) {
                app.container.recordActivityEvent.execute(
                    ActivityEvent(
                        source = EventSource.MOBILE_CALL,
                        eventType = "incoming_call_screened",
                        target = "call_peer",
                        timestampMs = System.currentTimeMillis(),
                        privacy = EventPrivacy.METADATA_ONLY,
                        contextJson = """{"content_captured":false,"decision":"blocked_by_context","reason":"$overrideReason","pressure":$pressure,"screen_on":$isScreenOn}""",
                        syncEligible = false,
                    ),
                )
                respondToCall(callDetails, blockCallResponse())
                return@launch
            }

            // 2. Otherwise, fallback to the Standard Shield Decision (Local Reports/Reputation)
            val decision = useCase.execute(peer)

            app.container.recordActivityEvent.execute(
                ActivityEvent(
                    source = EventSource.MOBILE_CALL,
                    eventType = "incoming_call_screened",
                    target = "call_peer:${decision.peerHash}",
                    timestampMs = System.currentTimeMillis(),
                    privacy = EventPrivacy.METADATA_ONLY,
                    contextJson = """{"content_captured":false,"peer_hash":"${decision.peerHash}","decision":"${if (decision.shouldBlock) "blocked_by_report" else "allow_default"}","report_label":"${decision.reportLabel ?: PhoneReportLabel.IGNORE}"}""",
                    syncEligible = false,
                ),
            )

            respondToCall(
                callDetails,
                if (decision.shouldBlock) blockCallResponse() else allowDefaultCallResponse(),
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
}
