package so.cynic.kyon.ports

import kotlinx.coroutines.flow.Flow
import so.cynic.kyon.shield.reporting.PhoneReport

interface PhoneReportStore {
    suspend fun findByHash(numberHash: String): PhoneReport?
    fun observeAll(): Flow<List<PhoneReport>>
    suspend fun upsert(report: PhoneReport)
    suspend fun delete(numberHash: String)
}
