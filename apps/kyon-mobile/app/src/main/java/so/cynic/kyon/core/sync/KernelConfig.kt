package so.cynic.kyon.core.sync

data class KernelConfig(
    val baseUrl: String,
    val bearerToken: String,
    val syncEnabled: Boolean,
)
