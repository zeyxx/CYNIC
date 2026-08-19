package so.cynic.kyon.core.model

enum class EventSource {
    MOBILE_USAGE,
    MOBILE_NOTIFICATION,
    MOBILE_CALL,
    MOBILE_SMS,
    USER_REPORT,
}

enum class EventPrivacy {
    METADATA_ONLY,
    LOCAL_SECRET,
}

data class ActivityEvent(
    val id: Long? = null,
    val source: EventSource,
    val eventType: String,
    val target: String,
    val timestampMs: Long,
    val durationMs: Long? = null,
    val privacy: EventPrivacy = EventPrivacy.METADATA_ONLY,
    val contextJson: String = "{}",
    val syncEligible: Boolean = false,
)
