package so.cynic.kyon.application.shield

import kotlinx.coroutines.flow.Flow
import so.cynic.kyon.ports.PhoneReportStore
import so.cynic.kyon.shield.reporting.PhoneReport

class ObservePhoneReports(
    private val phoneReportStore: PhoneReportStore,
) {
    fun execute(): Flow<List<PhoneReport>> {
        return phoneReportStore.observeAll()
    }
}

