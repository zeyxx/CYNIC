package so.cynic.kyon.mirror.notifications

import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.EventRepository

class KyonNotificationListenerService : NotificationListenerService() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val repository = EventRepository((application as KyonApp).database.activityEvents())
        val event = ActivityEvent(
            source = EventSource.MOBILE_NOTIFICATION,
            eventType = "notification_posted",
            target = sbn.packageName,
            timestampMs = sbn.postTime,
            contextJson = """{"content_captured":false}""",
            syncEligible = false,
        )

        scope.launch {
            repository.record(event)
        }
    }
}
