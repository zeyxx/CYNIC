package so.cynic.kyon.application.shield

import so.cynic.kyon.ports.PhoneReportStore

class DeletePhoneReport(
    private val phoneReportStore: PhoneReportStore,
) {
    suspend fun execute(numberHash: String) {
        phoneReportStore.delete(numberHash)
    }
}

