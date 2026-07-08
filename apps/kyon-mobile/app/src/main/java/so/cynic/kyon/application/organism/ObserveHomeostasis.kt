package so.cynic.kyon.application.organism

import kotlinx.coroutines.flow.Flow
import so.cynic.kyon.core.storage.OrganismStateDao
import so.cynic.kyon.core.storage.OrganismStateEntity

class ObserveHomeostasis(
    private val dao: OrganismStateDao
) {
    fun execute(): Flow<OrganismStateEntity?> {
        return dao.observeHomeostasis()
    }
}
