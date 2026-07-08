package so.cynic.kyon.application.shield

import so.cynic.kyon.ports.PeerHasher
import so.cynic.kyon.ports.PhoneReportStore
import so.cynic.kyon.shield.reporting.PhoneReport
import so.cynic.kyon.shield.reporting.PhoneReportLabel
import java.util.Locale

class SavePhoneReport(
    private val phoneReportStore: PhoneReportStore,
    private val peerHasher: PeerHasher,
) {
    suspend fun execute(rawInput: String, label: PhoneReportLabel) {
        val normalized = rawInput.trim()
        if (normalized.isBlank()) return

        val numberHash = normalizePhoneReportTarget(normalized)
        phoneReportStore.upsert(
            PhoneReport(
                numberHash = numberHash,
                label = label,
                reportedAtMs = System.currentTimeMillis(),
            ),
        )
    }

    private fun normalizePhoneReportTarget(input: String): String {
        val trimmed = input.trim().lowercase(Locale.US)
        val looksLikeHash = trimmed.length == 16 && trimmed.all { it in '0'..'9' || it in 'a'..'f' }
        return if (looksLikeHash) trimmed else peerHasher.hash(trimmed)
    }
}
