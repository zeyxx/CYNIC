package so.cynic.kyon.application.shield

import so.cynic.kyon.ports.PeerHasher
import so.cynic.kyon.ports.PhoneReportStore
import so.cynic.kyon.shield.reporting.PhoneReportLabel

class ShieldDecision(
    private val hasher: PeerHasher,
    private val phoneReportStore: PhoneReportStore,
) {
    suspend fun execute(peer: String): Decision {
        val normalized = hasher.hash(peer)
        val report = phoneReportStore.findByHash(normalized)
        val label = report?.label
        val shouldBlock = shouldBlock(label)

        return Decision(
            peerHash = normalized,
            reportLabel = label,
            shouldBlock = shouldBlock,
        )
    }

    private fun shouldBlock(label: PhoneReportLabel?): Boolean {
        return when (label) {
            PhoneReportLabel.SCAM, PhoneReportLabel.NUISANCE -> true
            PhoneReportLabel.LEGITIMATE, PhoneReportLabel.IGNORE -> false
            null -> false
        }
    }

    data class Decision(
        val peerHash: String,
        val reportLabel: PhoneReportLabel?,
        val shouldBlock: Boolean,
    )
}
