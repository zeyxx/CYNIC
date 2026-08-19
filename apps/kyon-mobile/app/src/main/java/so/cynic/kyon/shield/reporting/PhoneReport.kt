package so.cynic.kyon.shield.reporting

enum class PhoneReportLabel {
    LEGITIMATE,
    NUISANCE,
    SCAM,
    IGNORE,
}

data class PhoneReport(
    val number: String,
    val label: PhoneReportLabel,
    val reportedAtMs: Long,
)
