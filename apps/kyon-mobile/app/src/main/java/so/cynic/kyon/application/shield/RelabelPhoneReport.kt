package so.cynic.kyon.application.shield

import so.cynic.kyon.ports.PhoneReportStore
import so.cynic.kyon.shield.reporting.PhoneReport
import so.cynic.kyon.shield.reporting.PhoneReportLabel

class RelabelPhoneReport(
    private val phoneReportStore: PhoneReportStore,
) {
    suspend fun execute(report: PhoneReport, newLabel: PhoneReportLabel) {
        if (report.label == newLabel) return

        phoneReportStore.upsert(
            report.copy(
                label = newLabel,
                reportedAtMs = System.currentTimeMillis(),
            ),
        )
    }
}

