package so.cynic.kyon.shield.reporting

enum class PhoneReportLabel {
    LEGITIMATE,
    NUISANCE,
    SCAM,
    IGNORE,
}

/**
 * A user report on a phone number.
 *
 * [numberHash] is a SHA-256 prefix (16 hex chars) of the normalized phone number — never the raw
 * number, consistent with [so.cynic.kyon.shield.calls.KyonCallScreeningService] and
 * [so.cynic.kyon.shield.sms.KyonSmsReceiver].
 */
data class PhoneReport(
    val numberHash: String,
    val label: PhoneReportLabel,
    val reportedAtMs: Long,
)
