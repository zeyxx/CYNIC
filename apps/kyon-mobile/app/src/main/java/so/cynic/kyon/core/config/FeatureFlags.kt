package so.cynic.kyon.core.config

import android.content.SharedPreferences

data class FeatureFlags(
    val shieldEnabled: Boolean,
    val syncEnabled: Boolean,
    val usageCollectionEnabled: Boolean,
    val notificationCollectionEnabled: Boolean,
    val exportEventsEnabled: Boolean,
    val socialSemanticMonitoringEnabled: Boolean,
    val commercialSemanticMonitoringEnabled: Boolean,
) {
    fun saveTo(editor: SharedPreferences.Editor): SharedPreferences.Editor = editor
        .putBoolean(PREF_SHIELD_ENABLED, shieldEnabled)
        .putBoolean(PREF_SYNC_ENABLED, syncEnabled)
        .putBoolean(PREF_USAGE_COLLECTION_ENABLED, usageCollectionEnabled)
        .putBoolean(PREF_NOTIFICATION_COLLECTION_ENABLED, notificationCollectionEnabled)
        .putBoolean(PREF_EXPORT_EVENTS_ENABLED, exportEventsEnabled)
        .putBoolean(PREF_SOCIAL_SEMANTIC_MONITORING_ENABLED, socialSemanticMonitoringEnabled)
        .putBoolean(PREF_COMMERCIAL_SEMANTIC_MONITORING_ENABLED, commercialSemanticMonitoringEnabled)

    companion object {
        const val PREF_SHIELD_ENABLED = "feature_shield_enabled"
        const val PREF_SYNC_ENABLED = "sync_enabled"
        const val PREF_USAGE_COLLECTION_ENABLED = "feature_usage_collection_enabled"
        const val PREF_NOTIFICATION_COLLECTION_ENABLED = "feature_notification_collection_enabled"
        const val PREF_EXPORT_EVENTS_ENABLED = "feature_export_events_enabled"
        const val PREF_SOCIAL_SEMANTIC_MONITORING_ENABLED = "feature_social_semantic_monitoring_enabled"
        const val PREF_COMMERCIAL_SEMANTIC_MONITORING_ENABLED = "feature_commercial_semantic_monitoring_enabled"

        fun readFrom(prefs: SharedPreferences): FeatureFlags = FeatureFlags(
            shieldEnabled = prefs.getBoolean(PREF_SHIELD_ENABLED, true),
            syncEnabled = prefs.getBoolean(PREF_SYNC_ENABLED, false),
            usageCollectionEnabled = prefs.getBoolean(PREF_USAGE_COLLECTION_ENABLED, true),
            notificationCollectionEnabled = prefs.getBoolean(PREF_NOTIFICATION_COLLECTION_ENABLED, true),
            exportEventsEnabled = prefs.getBoolean(PREF_EXPORT_EVENTS_ENABLED, true),
            socialSemanticMonitoringEnabled = prefs.getBoolean(PREF_SOCIAL_SEMANTIC_MONITORING_ENABLED, false), // Default explicit Opt-In
            commercialSemanticMonitoringEnabled = prefs.getBoolean(PREF_COMMERCIAL_SEMANTIC_MONITORING_ENABLED, false), // Default explicit Opt-In
        )
    }
}
