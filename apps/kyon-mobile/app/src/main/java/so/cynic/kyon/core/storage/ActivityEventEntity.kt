package so.cynic.kyon.core.storage

import androidx.room.Entity
import androidx.room.PrimaryKey
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventPrivacy
import so.cynic.kyon.core.model.EventSource

@Entity(tableName = "activity_events")
data class ActivityEventEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val source: String,
    val eventType: String,
    val target: String,
    val timestampMs: Long,
    val durationMs: Long?,
    val privacy: String,
    val contextJson: String,
    val syncEligible: Boolean,
    val syncedAtMs: Long?,
) {
    fun toModel(): ActivityEvent = ActivityEvent(
        source = EventSource.valueOf(source),
        eventType = eventType,
        target = target,
        timestampMs = timestampMs,
        durationMs = durationMs,
        privacy = EventPrivacy.valueOf(privacy),
        contextJson = contextJson,
        syncEligible = syncEligible,
    )
}

fun ActivityEvent.toEntity(): ActivityEventEntity = ActivityEventEntity(
    source = source.name,
    eventType = eventType,
    target = target,
    timestampMs = timestampMs,
    durationMs = durationMs,
    privacy = privacy.name,
    contextJson = contextJson,
    syncEligible = syncEligible,
    syncedAtMs = null,
)
