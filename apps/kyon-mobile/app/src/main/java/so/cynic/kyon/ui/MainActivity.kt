package so.cynic.kyon.ui

import android.app.AppOpsManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.work.WorkInfo
import androidx.work.WorkManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.EventRepository
import so.cynic.kyon.mirror.episodes.EpisodeBuilder
import so.cynic.kyon.mirror.identity.IdentitySnapshot
import so.cynic.kyon.mirror.identity.IdentitySnapshotBuilder
import so.cynic.kyon.mirror.usage.UsageSnapshotCollector
import so.cynic.kyon.shield.reporting.PhoneReportEntity
import so.cynic.kyon.shield.reporting.PhoneReportLabel
import java.net.HttpURLConnection
import java.net.URL
import java.text.DateFormat
import java.security.MessageDigest
import java.util.Locale

class MainActivity : ComponentActivity() {
    private var phoneReports by mutableStateOf<List<PhoneReportEntity>>(emptyList())
    private var phoneReportInput by mutableStateOf("")
    private var selectedPhoneReportLabel by mutableStateOf(PhoneReportLabel.SCAM)
    private var kernelPingStatus by mutableStateOf("Aucun test effectué")
    private var kernelSyncWorkerStatus by mutableStateOf("Inconnu")
    private var kernelSyncWorkerLastRun by mutableStateOf("Jamais")
    private var kernelSyncWorkerResult by mutableStateOf("—")
    private var kernelUrl by mutableStateOf("")
    private var kernelToken by mutableStateOf("")
    private var kernelSyncEnabled by mutableStateOf(false)
    private var events by mutableStateOf<List<ActivityEvent>>(emptyList())
    private var mirrorIdentity by mutableStateOf<IdentitySnapshot?>(null)
    private var usageAccessGranted by mutableStateOf(false)
    private var notificationAccessGranted by mutableStateOf(false)
    private var isRefreshingUsage by mutableStateOf(false)

    private val episodeBuilder = EpisodeBuilder()
    private val identitySnapshotBuilder = IdentitySnapshotBuilder()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        loadKernelSettingsFromPrefs()
        setContent {
            KyonShell(
                usageAccessGranted = usageAccessGranted,
                notificationAccessGranted = notificationAccessGranted,
                events = events,
                mirrorIdentity = mirrorIdentity,
                phoneReports = phoneReports,
                phoneReportInput = phoneReportInput,
                selectedPhoneReportLabel = selectedPhoneReportLabel,
                kernelUrl = kernelUrl,
                kernelToken = kernelToken,
                kernelSyncEnabled = kernelSyncEnabled,
                kernelPingStatus = kernelPingStatus,
                kernelSyncWorkerStatus = kernelSyncWorkerStatus,
                kernelSyncWorkerLastRun = kernelSyncWorkerLastRun,
                kernelSyncWorkerResult = kernelSyncWorkerResult,
                isRefreshingUsage = isRefreshingUsage,
                openUsageAccess = {
                    startActivity(Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS))
                },
                openNotificationAccess = {
                    startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
                },
                captureUsageSnapshot = ::captureUsageSnapshot,
                onPhoneReportInputChange = { phoneReportInput = it },
                onPhoneReportLabelSelect = { selectedPhoneReportLabel = it },
                onPhoneReportSubmit = {
                    savePhoneReport(phoneReportInput, selectedPhoneReportLabel)
                    phoneReportInput = ""
                },
                onPhoneReportDelete = { report -> deletePhoneReport(report.numberHash) },
                onPhoneReportRelabel = { report, label ->
                    relabelPhoneReport(report, label)
                },
                onKernelUrlChange = { kernelUrl = it },
                onKernelTokenChange = { kernelToken = it },
                onKernelSyncEnabledChange = { kernelSyncEnabled = it },
                onKernelSettingsSave = { saveKernelSettings() },
                onKernelPing = { pingKernel() },
            )
        }
        observeEvents()
        observePhoneReports()
        observeKernelSyncWorkerStatus()
        refreshKernelSyncStatus()
    }

    override fun onResume() {
        super.onResume()
        usageAccessGranted = hasUsageAccess()
        notificationAccessGranted = hasNotificationAccess()
        loadKernelSettingsFromPrefs()
        loadEvents()
        refreshKernelSyncStatus()
    }

    private fun observeKernelSyncWorkerStatus() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                while (true) {
                    refreshKernelSyncStatus()
                    delay(10_000L)
                }
            }
        }
    }

    private fun observeEvents() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                repository().observeRecent().collect { latest ->
                    events = latest
                    mirrorIdentity = buildIdentityFrom(latest)
                    notificationAccessGranted = hasNotificationAccess()
                }
            }
        }
    }

    private fun observePhoneReports() {
        val app = application as KyonApp
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                app.database.phoneReports().observeAll().collect { latest ->
                    phoneReports = latest
                }
            }
        }
    }

    private fun captureUsageSnapshot() {
        if (isRefreshingUsage) return
        isRefreshingUsage = true

        lifecycleScope.launch(Dispatchers.IO) {
            try {
                val repository = repository()
                val collected = UsageSnapshotCollector(applicationContext).snapshot()
                collected.forEach { repository.record(it) }
                val latest = repository.recent()
                val identity = buildIdentityFrom(latest)

                withContext(Dispatchers.Main) {
                    events = latest
                    mirrorIdentity = identity
                }
            } finally {
                withContext(Dispatchers.Main) {
                    isRefreshingUsage = false
                }
            }
        }
    }

    private fun loadEvents() {
        lifecycleScope.launch(Dispatchers.IO) {
            val latest = repository().recent()
            withContext(Dispatchers.Main) {
                events = latest
                mirrorIdentity = buildIdentityFrom(latest)
            }
        }
    }

    private fun buildIdentityFrom(events: List<ActivityEvent>): IdentitySnapshot {
        val episodes = episodeBuilder.from(events)
        return identitySnapshotBuilder.build(episodes)
    }

    private fun savePhoneReport(rawInput: String, label: PhoneReportLabel) {
        val normalized = rawInput.trim()
        if (normalized.isBlank()) return

        val numberHash = normalizePhoneReportTarget(normalized)
        val report = PhoneReportEntity(
            numberHash = numberHash,
            label = label.name,
            reportedAtMs = System.currentTimeMillis(),
        )

        lifecycleScope.launch(Dispatchers.IO) {
            (application as KyonApp).database.phoneReports().upsert(report)
        }
    }

    private fun deletePhoneReport(numberHash: String) {
        lifecycleScope.launch(Dispatchers.IO) {
            (application as KyonApp).database.phoneReports().delete(numberHash)
        }
    }

    private fun relabelPhoneReport(report: PhoneReportEntity, label: PhoneReportLabel) {
        if (report.label == label.name) return

        val updated = report.copy(
            label = label.name,
            reportedAtMs = System.currentTimeMillis(),
        )
        lifecycleScope.launch(Dispatchers.IO) {
            (application as KyonApp).database.phoneReports().upsert(updated)
        }
    }

    private fun normalizePhoneReportTarget(input: String): String {
        val trimmed = input.trim().lowercase(Locale.US)
        val looksLikeHash = trimmed.length == 16 && trimmed.all { it in '0'..'9' || it in 'a'..'f' || it in 'A'..'F' }
        return if (looksLikeHash) trimmed else phoneHash(trimmed)
    }

    private fun phoneHash(raw: String): String {
        val normalized = raw.filter { !it.isWhitespace() }.lowercase(Locale.US)
            .ifBlank { "unknown" }
        val digest = MessageDigest.getInstance("SHA-256").digest(normalized.toByteArray())
        return digest.joinToString(separator = "") { "%02x".format(it) }.take(16)
    }

    private fun loadKernelSettingsFromPrefs() {
        val app = application as KyonApp
        val prefs = app.getSharedPreferences(KyonApp.PREFS_KERNEL, Context.MODE_PRIVATE)

        kernelUrl = prefs.getString(KyonApp.PREF_KERNEL_URL, "") ?: ""
        kernelToken = prefs.getString(KyonApp.PREF_KERNEL_BEARER, "") ?: ""
        kernelSyncEnabled = prefs.getBoolean(KyonApp.PREF_SYNC_ENABLED, false)
    }

    private fun saveKernelSettings() {
        val app = application as KyonApp
        app.getSharedPreferences(KyonApp.PREFS_KERNEL, Context.MODE_PRIVATE)
            .edit()
            .putString(KyonApp.PREF_KERNEL_URL, kernelUrl.trim())
            .putString(KyonApp.PREF_KERNEL_BEARER, kernelToken.trim())
            .putBoolean(KyonApp.PREF_SYNC_ENABLED, kernelSyncEnabled)
            .apply()

        app.refreshKernelConfig()
        refreshKernelSyncStatus()
    }

    private fun refreshKernelSyncStatus() {
        lifecycleScope.launch(Dispatchers.IO) {
            val infos = WorkManager.getInstance(this@MainActivity)
                .getWorkInfosForUniqueWork("KernelSyncWork")
                .get()

            if (infos.isEmpty()) {
                withContext(Dispatchers.Main) {
                    kernelSyncWorkerStatus = "Worker non planifié"
                    kernelSyncWorkerLastRun = "Jamais"
                    kernelSyncWorkerResult = "—"
                }
                return@launch
            }

            val latest = infos.maxWithOrNull { left, right ->
                val score = compareByState(left.state).compareTo(compareByState(right.state))
                if (score != 0) score else left.runAttemptCount.compareTo(right.runAttemptCount)
            } ?: infos.first()

            val lastSyncMs = latest.outputData.getLong("last_sync_ms", 0L)
            val syncedCount = latest.outputData.getInt("synced_count", 0)
            val pendingCount = latest.outputData.getInt("pending_count", 0)
            val result = latest.outputData.getString("result") ?: "—"

            val statusText = when (latest.state) {
                WorkInfo.State.RUNNING -> "En cours"
                WorkInfo.State.ENQUEUED -> "En attente"
                WorkInfo.State.SUCCEEDED -> "Terminé"
                WorkInfo.State.FAILED -> "Echec"
                WorkInfo.State.BLOCKED -> "Bloqué"
                WorkInfo.State.CANCELLED -> "Annulé"
                else -> "Inconnu"
            }

            withContext(Dispatchers.Main) {
                kernelSyncWorkerStatus = statusText
                kernelSyncWorkerLastRun = if (lastSyncMs > 0L) formatTimestamp(lastSyncMs) else "Jamais"
                kernelSyncWorkerResult = if (pendingCount > 0) {
                    "${syncedCount}/${pendingCount} événements: $result"
                } else {
                    result
                }
            }
        }
    }

    private fun compareByState(state: WorkInfo.State): Int =
        when (state) {
            WorkInfo.State.RUNNING -> 6
            WorkInfo.State.ENQUEUED -> 5
            WorkInfo.State.BLOCKED -> 4
            WorkInfo.State.SUCCEEDED -> 3
            WorkInfo.State.FAILED -> 1
            WorkInfo.State.CANCELLED -> 0
            else -> -1
        }

    private fun pingKernel() {
        val target = kernelUrl.trim().trimEnd('/')
        if (target.isBlank()) {
            kernelPingStatus = "URL vide"
            return
        }

        kernelPingStatus = "Test en cours..."
        lifecycleScope.launch(Dispatchers.IO) {
            val status = try {
                val connection = URL("${target}/health").openConnection() as HttpURLConnection
                try {
                    connection.requestMethod = "GET"
                    connection.connectTimeout = 4000
                    connection.readTimeout = 4000
                    val code = connection.responseCode
                    if (code in 200..299) "Kernel OK (${code})" else "Kernel erreur HTTP ${code}"
                } finally {
                    connection.disconnect()
                }
            } catch (e: Exception) {
                "Kernel unreachable: ${e.message ?: e.javaClass.simpleName}"
            }

            withContext(Dispatchers.Main) {
                kernelPingStatus = status
            }
        }
    }

    private fun repository(): EventRepository =
        EventRepository((application as KyonApp).database.activityEvents())

    private fun hasUsageAccess(): Boolean {
        val appOps = getSystemService(Context.APP_OPS_SERVICE) as AppOpsManager
        val mode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            appOps.unsafeCheckOpNoThrow(
                AppOpsManager.OPSTR_GET_USAGE_STATS,
                android.os.Process.myUid(),
                packageName,
            )
        } else {
            @Suppress("DEPRECATION")
            appOps.checkOpNoThrow(
                AppOpsManager.OPSTR_GET_USAGE_STATS,
                android.os.Process.myUid(),
                packageName,
            )
        }
        return mode == AppOpsManager.MODE_ALLOWED
    }

    private fun hasNotificationAccess(): Boolean {
        val enabledListeners = Settings.Secure.getString(
            contentResolver,
            "enabled_notification_listeners",
        ) ?: return false

        return enabledListeners
            .split(':')
            .mapNotNull(ComponentName::unflattenFromString)
            .any { it.packageName == packageName }
    }
}

@Composable
private fun KyonShell(
    usageAccessGranted: Boolean,
    notificationAccessGranted: Boolean,
    events: List<ActivityEvent>,
    mirrorIdentity: IdentitySnapshot?,
    phoneReports: List<PhoneReportEntity>,
    phoneReportInput: String,
    selectedPhoneReportLabel: PhoneReportLabel,
    kernelUrl: String,
    kernelToken: String,
    kernelSyncEnabled: Boolean,
    kernelPingStatus: String,
    kernelSyncWorkerStatus: String,
    kernelSyncWorkerLastRun: String,
    kernelSyncWorkerResult: String,
    isRefreshingUsage: Boolean,
    openUsageAccess: () -> Unit,
    openNotificationAccess: () -> Unit,
    captureUsageSnapshot: () -> Unit,
    onPhoneReportInputChange: (String) -> Unit,
    onPhoneReportLabelSelect: (PhoneReportLabel) -> Unit,
    onPhoneReportSubmit: () -> Unit,
    onPhoneReportDelete: (PhoneReportEntity) -> Unit,
    onPhoneReportRelabel: (PhoneReportEntity, PhoneReportLabel) -> Unit,
    onKernelUrlChange: (String) -> Unit,
    onKernelTokenChange: (String) -> Unit,
    onKernelSyncEnabledChange: (Boolean) -> Unit,
    onKernelSettingsSave: () -> Unit,
    onKernelPing: () -> Unit,
) {
    var selectedFilter by remember { mutableStateOf(EventFilter.ALL) }
    val filteredEvents = events.filter { selectedFilter.includes(it) }

    MaterialTheme {
        Surface(modifier = Modifier.fillMaxSize()) {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .statusBarsPadding()
                    .padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Kyon", style = MaterialTheme.typography.headlineMedium)
                        Text("Shield activé par signalements locaux. Mirror local, metadata only.")
                    }
                }

                item {
                    StatusCard(
                        usageAccessGranted = usageAccessGranted,
                        notificationAccessGranted = notificationAccessGranted,
                        mirrorIdentity = mirrorIdentity,
                        kernelSyncEnabled = kernelSyncEnabled,
                        kernelSyncWorkerStatus = kernelSyncWorkerStatus,
                        kernelSyncWorkerLastRun = kernelSyncWorkerLastRun,
                        kernelSyncWorkerResult = kernelSyncWorkerResult,
                    )
                }

                item {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Button(
                            modifier = Modifier.weight(1f),
                            onClick = openUsageAccess,
                        ) {
                            Text("Usage")
                        }
                        OutlinedButton(
                            modifier = Modifier.weight(1f),
                            onClick = openNotificationAccess,
                        ) {
                            Text("Notifs")
                        }
                    }
                }

                item {
                    Button(
                        modifier = Modifier.fillMaxWidth(),
                        enabled = usageAccessGranted && !isRefreshingUsage,
                        onClick = captureUsageSnapshot,
                    ) {
                        Text(if (isRefreshingUsage) "Snapshot..." else "Snapshot usage")
                    }
                }

                item {
                    PhoneReportCard(
                        phoneReportInput = phoneReportInput,
                        reports = phoneReports,
                        selectedLabel = selectedPhoneReportLabel,
                        onInputChange = onPhoneReportInputChange,
                        onLabelSelect = onPhoneReportLabelSelect,
                        onSubmit = onPhoneReportSubmit,
                        onDelete = onPhoneReportDelete,
                        onRelabel = onPhoneReportRelabel,
                    )
                }

                item {
                    KernelSettingsCard(
                        kernelUrl = kernelUrl,
                        kernelToken = kernelToken,
                        kernelSyncEnabled = kernelSyncEnabled,
                        kernelPingStatus = kernelPingStatus,
                        onKernelUrlChange = onKernelUrlChange,
                        onKernelTokenChange = onKernelTokenChange,
                        onKernelSyncEnabledChange = onKernelSyncEnabledChange,
                        onSave = onKernelSettingsSave,
                        onPing = onKernelPing,
                    )
                }

                item {
                    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Text(
                            "Journal local (${filteredEvents.size})",
                            style = MaterialTheme.typography.titleMedium,
                        )
                        EventFilterTabs(
                            selected = selectedFilter,
                            onSelected = { selectedFilter = it },
                        )
                    }
                }

                if (filteredEvents.isEmpty()) {
                    item {
                        EmptyEventCard()
                    }
                } else {
                    items(filteredEvents) { event ->
                        EventCard(event)
                    }
                }
            }
        }
    }
}

private enum class EventFilter(
    val label: String,
    private val source: EventSource?,
) {
    ALL("All", null),
    USAGE("Usage", EventSource.MOBILE_USAGE),
    NOTIFICATIONS("Notifications", EventSource.MOBILE_NOTIFICATION),
    CALLS("Calls", EventSource.MOBILE_CALL),
    SMS("SMS", EventSource.MOBILE_SMS);

    fun includes(event: ActivityEvent): Boolean = source == null || event.source == source
}

@Composable
private fun EventFilterTabs(
    selected: EventFilter,
    onSelected: (EventFilter) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        EventFilter.entries.forEach { filter ->
            FilterChip(
                selected = selected == filter,
                onClick = { onSelected(filter) },
                label = { Text(filter.label) },
            )
        }
    }
}

@Composable
private fun StatusCard(
    usageAccessGranted: Boolean,
    notificationAccessGranted: Boolean,
    mirrorIdentity: IdentitySnapshot?,
    kernelSyncEnabled: Boolean,
    kernelSyncWorkerStatus: String,
    kernelSyncWorkerLastRun: String,
    kernelSyncWorkerResult: String,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text("Etat local", style = MaterialTheme.typography.titleMedium)
            StatusRow("Usage access", usageAccessGranted)
            StatusRow("Notification listener", notificationAccessGranted)
            StatusRow("Mirror profile", mirrorIdentity != null, falseLabel = "inactive")
            StatusRow("Kernel sync", kernelSyncEnabled, falseLabel = "off")
            Text(
                "Statut worker: $kernelSyncWorkerStatus",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                "Dernier flush: $kernelSyncWorkerLastRun",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (kernelSyncWorkerResult.isNotBlank()) {
                Text(
                    "Résultat: $kernelSyncWorkerResult",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            StatusRow("Content capture", false, falseLabel = "never")

            if (mirrorIdentity != null) {
                Text(
                    "Top surfaces: ${mirrorIdentity.topSurfaces.joinToString(", ")}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    "Night usage episodes: ${mirrorIdentity.nightUsageEpisodes} • " +
                        "Confidence: ${mirrorIdentity.confidence}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun StatusRow(
    label: String,
    enabled: Boolean,
    falseLabel: String = "missing",
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label)
        Text(
            text = if (enabled) "on" else falseLabel,
            color = if (enabled) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
        )
    }
}

@Composable
private fun PhoneReportCard(
    phoneReportInput: String,
    reports: List<PhoneReportEntity>,
    selectedLabel: PhoneReportLabel,
    onInputChange: (String) -> Unit,
    onLabelSelect: (PhoneReportLabel) -> Unit,
    onSubmit: () -> Unit,
    onDelete: (PhoneReportEntity) -> Unit,
    onRelabel: (PhoneReportEntity, PhoneReportLabel) -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier
                .padding(14.dp)
                .heightIn(max = 260.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text("Signaler un numéro", style = MaterialTheme.typography.titleMedium)
            Text(
                "Saisis un numero (ou hash 16 chars) pour enregistrer un signalement local.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            OutlinedTextField(
                value = phoneReportInput,
                onValueChange = onInputChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text("Numero ou hash") },
            )
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                PhoneReportLabel.entries.forEach { label ->
                    FilterChip(
                        selected = selectedLabel == label,
                        onClick = { onLabelSelect(label) },
                        label = { Text(label.name.lowercase()) },
                    )
                }
            }
            Button(
                modifier = Modifier.fillMaxWidth(),
                enabled = phoneReportInput.isNotBlank(),
                onClick = onSubmit,
            ) {
                Text("Ajouter signalement")
            }

            Text(
                "Derniers rapports (${reports.size})",
                style = MaterialTheme.typography.bodySmall,
            )
            if (reports.isEmpty()) {
                Text(
                    "Aucun rapport local",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    reports.take(6).forEach { report ->
                        ReportRow(
                            report = report,
                            onDelete = { onDelete(report) },
                            onRelabel = { label -> onRelabel(report, label) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ReportRow(
    report: PhoneReportEntity,
    onDelete: () -> Unit,
    onRelabel: (PhoneReportLabel) -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(
            text = "${formatTimestamp(report.reportedAtMs)} | ${report.numberHash}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            PhoneReportLabel.entries.forEach { label ->
                FilterChip(
                    selected = report.label == label.name,
                    onClick = { onRelabel(label) },
                    label = { Text(label.name.lowercase()) },
                )
            }
            TextButton(onClick = onDelete) {
                Text("Supprimer")
            }
        }
    }
}

@Composable
private fun KernelSettingsCard(
    kernelUrl: String,
    kernelToken: String,
    kernelSyncEnabled: Boolean,
    kernelPingStatus: String,
    onKernelUrlChange: (String) -> Unit,
    onKernelTokenChange: (String) -> Unit,
    onKernelSyncEnabledChange: (Boolean) -> Unit,
    onSave: () -> Unit,
    onPing: () -> Unit,
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text("Kernel settings", style = MaterialTheme.typography.titleMedium)
            OutlinedTextField(
                value = kernelUrl,
                onValueChange = onKernelUrlChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text("Kernel URL") },
            )
            OutlinedTextField(
                value = kernelToken,
                onValueChange = onKernelTokenChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                label = { Text("Token Bearer") },
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text("Sync active")
                Switch(
                    checked = kernelSyncEnabled,
                    onCheckedChange = onKernelSyncEnabledChange,
                )
            }
            Button(
                modifier = Modifier.fillMaxWidth(),
                onClick = onSave,
            ) {
                Text("Enregistrer paramètres kernel")
            }
            Button(
                modifier = Modifier.fillMaxWidth(),
                onClick = onPing,
            ) {
                Text("Tester le kernel")
            }
            Text(
                "Etat connexion: $kernelPingStatus",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun EventCard(event: ActivityEvent) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(event.eventType, style = MaterialTheme.typography.titleSmall)
            Text(event.target, style = MaterialTheme.typography.bodyMedium)
            Text(
                "${event.source.name.lowercase()} - ${formatTimestamp(event.timestampMs)}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (event.durationMs != null) {
                Text(
                    "duration ${formatDuration(event.durationMs)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun EmptyEventCard() {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("Aucun evenement local")
            Spacer(Modifier.height(4.dp))
            Text(
                "Les services Shield et Mirror ecriront ici apres autorisation.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

private fun formatTimestamp(timestampMs: Long): String =
    DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
        .format(timestampMs)

private fun formatDuration(durationMs: Long): String {
    val minutes = durationMs / 60_000L
    return if (minutes > 0) "${minutes}m" else "${durationMs / 1_000L}s"
}
