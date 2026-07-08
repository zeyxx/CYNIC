package so.cynic.kyon.application.sync

data class SyncBatchResult(
    val syncedCount: Int,
    val pendingCount: Int,
) {
    val isComplete: Boolean
        get() = syncedCount == pendingCount

    val resultLabel: String
        get() = when {
            pendingCount == 0 -> "nothing to sync"
            isComplete -> "synced"
            else -> "partial sync"
        }
}
