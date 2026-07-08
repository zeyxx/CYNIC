package so.cynic.kyon.adapters.storage.room

import so.cynic.kyon.shield.reporting.PhoneReportDao
import so.cynic.kyon.ports.PhoneReportStore
import so.cynic.kyon.shield.reporting.PhoneReport
import so.cynic.kyon.shield.reporting.toEntity
import kotlinx.coroutines.flow.map

class RoomPhoneReportStore(
    private val dao: PhoneReportDao,
) : PhoneReportStore {
    override suspend fun findByHash(numberHash: String): PhoneReport? {
        return dao.findByHash(numberHash)?.toModel()
    }

    override fun observeAll() = dao.observeAll().map { reports ->
        reports.map { it.toModel() }
    }

    override suspend fun upsert(report: PhoneReport) {
        dao.upsert(report.toEntity())
    }

    override suspend fun delete(numberHash: String) {
        dao.delete(numberHash)
    }
}
