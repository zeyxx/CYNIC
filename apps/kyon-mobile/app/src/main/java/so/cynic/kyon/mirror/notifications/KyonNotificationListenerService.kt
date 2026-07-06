package so.cynic.kyon.mirror.notifications

import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import so.cynic.kyon.KyonApp

class KyonNotificationListenerService : NotificationListenerService() {
    private val scope = CoroutineScope(Dispatchers.IO)

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val app = application as? KyonApp ?: return
        if (!app.featureFlags.notificationCollectionEnabled) return

        val useCase = app.container.recordNotificationEvent
        val packageName = sbn.packageName
        
        val socialApps = setOf(
            "com.instagram.android", 
            "com.whatsapp", 
            "com.snapchat.android", 
            "com.facebook.katana", 
            "com.facebook.orca",
            "com.twitter.android", 
            "com.zhiliaoapp.musically" // TikTok
        )
        
        val isSocialApp = socialApps.contains(packageName)
        val canExtractSemantics = if (isSocialApp) {
            app.featureFlags.socialSemanticMonitoringEnabled
        } else {
            true // Other apps could be checked against commercialSemanticMonitoringEnabled if needed
        }

        val title = if (canExtractSemantics) { sbn.notification.extras.getString("android.title") ?: "" } else { "" }
        val text = if (canExtractSemantics) { sbn.notification.extras.getCharSequence("android.text")?.toString() ?: "" } else { "" }

        scope.launch {
            useCase.execute(
                packageName = sbn.packageName,
                postedAtMs = sbn.postTime,
                isOngoing = sbn.isOngoing,
                notificationId = sbn.id,
                title = title,
                text = text,
            )
        }
    }
}
