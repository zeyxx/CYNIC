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
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import so.cynic.kyon.KyonApp
import so.cynic.kyon.core.model.ActivityEvent
import so.cynic.kyon.core.model.EventSource
import so.cynic.kyon.core.storage.EventRepository
import so.cynic.kyon.mirror.usage.UsageSnapshotCollector
import java.text.DateFormat

class MainActivity : ComponentActivity() {
    private var events by mutableStateOf<List<ActivityEvent>>(emptyList())
    private var usageAccessGranted by mutableStateOf(false)
    private var notificationAccessGranted by mutableStateOf(false)
    private var isRefreshingUsage by mutableStateOf(false)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            KyonShell(
                usageAccessGranted = usageAccessGranted,
                notificationAccessGranted = notificationAccessGranted,
                events = events,
                isRefreshingUsage = isRefreshingUsage,
                openUsageAccess = {
                    startActivity(Intent(Settings.ACTION_USAGE_ACCESS_SETTINGS))
                },
                openNotificationAccess = {
                    startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
                },
                captureUsageSnapshot = ::captureUsageSnapshot,
            )
        }
        observeEvents()
    }

    override fun onResume() {
        super.onResume()
        usageAccessGranted = hasUsageAccess()
        notificationAccessGranted = hasNotificationAccess()
        loadEvents()
    }

    private fun observeEvents() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                repository().observeRecent().collect { latest ->
                    events = latest
                    notificationAccessGranted = hasNotificationAccess()
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
                UsageSnapshotCollector(applicationContext).snapshot()
                    .forEach { repository.record(it) }
                val latest = repository.recent()

                withContext(Dispatchers.Main) {
                    events = latest
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

enum class Screen {
    JOURNAL, STATUS
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun KyonShell(
    usageAccessGranted: Boolean,
    notificationAccessGranted: Boolean,
    events: List<ActivityEvent>,
    isRefreshingUsage: Boolean,
    openUsageAccess: () -> Unit,
    openNotificationAccess: () -> Unit,
    captureUsageSnapshot: () -> Unit,
) {
    var currentScreen by remember { mutableStateOf(Screen.JOURNAL) }

    val kyonColorScheme = darkColorScheme(
        primary = Color(0xFF64B5F6),
        onPrimary = Color(0xFF00334D),
        primaryContainer = Color(0xFF004C6D),
        onPrimaryContainer = Color(0xFFC4E7FF),
        secondary = Color(0xFF81C784),
        surface = Color(0xFF1E1E1E),
        background = Color(0xFF121212),
        error = Color(0xFFE57373)
    )

    MaterialTheme(colorScheme = kyonColorScheme) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text("Kyon Agent", fontWeight = FontWeight.Bold) },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.background,
                        titleContentColor = MaterialTheme.colorScheme.primary,
                    )
                )
            },
            bottomBar = {
                NavigationBar {
                    NavigationBarItem(
                        selected = currentScreen == Screen.JOURNAL,
                        onClick = { currentScreen = Screen.JOURNAL },
                        icon = { Icon(Icons.AutoMirrored.Filled.List, contentDescription = "Journal") },
                        label = { Text("Journal") }
                    )
                    NavigationBarItem(
                        selected = currentScreen == Screen.STATUS,
                        onClick = { currentScreen = Screen.STATUS },
                        icon = { Icon(Icons.Default.Info, contentDescription = "Status") },
                        label = { Text("Status") }
                    )
                }
            }
        ) { padding ->
            Surface(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                color = MaterialTheme.colorScheme.background
            ) {
                when (currentScreen) {
                    Screen.JOURNAL -> JournalScreen(events)
                    Screen.STATUS -> StatusScreen(
                        usageAccessGranted = usageAccessGranted,
                        notificationAccessGranted = notificationAccessGranted,
                        isRefreshingUsage = isRefreshingUsage,
                        openUsageAccess = openUsageAccess,
                        openNotificationAccess = openNotificationAccess,
                        captureUsageSnapshot = captureUsageSnapshot
                    )
                }
            }
        }
    }
}

@Composable
private fun JournalScreen(events: List<ActivityEvent>) {
    var selectedFilter by remember { mutableStateOf(EventFilter.ALL) }
    val filteredEvents = events.filter { selectedFilter.includes(it) }

    Column(modifier = Modifier.fillMaxSize()) {
        EventFilterTabs(
            selected = selectedFilter,
            onSelected = { selectedFilter = it },
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
        )

        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(
                start = 16.dp, end = 16.dp, top = 8.dp, bottom = 24.dp
            )
        ) {
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

@Composable
private fun StatusScreen(
    usageAccessGranted: Boolean,
    notificationAccessGranted: Boolean,
    isRefreshingUsage: Boolean,
    openUsageAccess: () -> Unit,
    openNotificationAccess: () -> Unit,
    captureUsageSnapshot: () -> Unit,
) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)
    ) {
        item {
            AnimatedVisibility(visible = !usageAccessGranted || !notificationAccessGranted) {
                PermissionsCard(
                    usageAccessGranted = usageAccessGranted,
                    notificationAccessGranted = notificationAccessGranted,
                    openUsageAccess = openUsageAccess,
                    openNotificationAccess = openNotificationAccess
                )
            }
        }

        item {
            StatusCard(
                usageAccessGranted = usageAccessGranted,
                notificationAccessGranted = notificationAccessGranted,
            )
        }

        item {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Manual Actions", style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary)
                    Button(
                        modifier = Modifier.fillMaxWidth(),
                        enabled = usageAccessGranted && !isRefreshingUsage,
                        onClick = captureUsageSnapshot,
                    ) {
                        Text(if (isRefreshingUsage) "Snapshotting..." else "Snapshot Usage Stats")
                    }
                }
            }
        }
    }
}

@Composable
private fun PermissionsCard(
    usageAccessGranted: Boolean,
    notificationAccessGranted: Boolean,
    openUsageAccess: () -> Unit,
    openNotificationAccess: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
        shape = RoundedCornerShape(16.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Default.Warning, contentDescription = "Warning", tint = MaterialTheme.colorScheme.error)
                Spacer(modifier = Modifier.size(8.dp))
                Text(
                    "Permissions Required",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onErrorContainer,
                    fontWeight = FontWeight.Bold
                )
            }
            Text(
                "Kyon needs permissions to effectively shield and mirror your device.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onErrorContainer
            )
            
            if (!usageAccessGranted) {
                OutlinedButton(onClick = openUsageAccess, modifier = Modifier.fillMaxWidth()) {
                    Text("Grant Usage Access")
                }
            }
            if (!notificationAccessGranted) {
                OutlinedButton(onClick = openNotificationAccess, modifier = Modifier.fillMaxWidth()) {
                    Text("Grant Notification Access")
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
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
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
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        shape = RoundedCornerShape(16.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Shield & Mirror Status", style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary)
            StatusRow("Usage access", usageAccessGranted)
            StatusRow("Notification listener", notificationAccessGranted)
            StatusRow("Kernel sync", true, falseLabel = "off")
            StatusRow("Content capture", false, falseLabel = "never")
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
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(label, style = MaterialTheme.typography.bodyLarge)
        Text(
            text = if (enabled) "ON" else falseLabel.uppercase(),
            style = MaterialTheme.typography.labelLarge,
            color = if (enabled) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.error,
            fontWeight = FontWeight.Bold
        )
    }
}

@Composable
private fun EventCard(event: ActivityEvent) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(event.eventType, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary)
                Text(
                    formatTimestamp(event.timestampMs),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            Text(event.target, style = MaterialTheme.typography.bodyMedium)
            
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Text(
                    event.source.name,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.secondary
                )
                if (event.durationMs != null) {
                    Text(
                        "duration: ${formatDuration(event.durationMs)}",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun EmptyEventCard() {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
    ) {
        Column(modifier = Modifier.padding(24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            Icon(Icons.AutoMirrored.Filled.List, contentDescription = null, modifier = Modifier.size(48.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(16.dp))
            Text("Log is empty", style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.height(4.dp))
            Text(
                "Shield and Mirror services will write events here after being authorized.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = androidx.compose.ui.text.style.TextAlign.Center
            )
        }
    }
}

private fun formatTimestamp(timestampMs: Long): String =
    DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(timestampMs)

private fun formatDuration(durationMs: Long): String {
    val minutes = durationMs / 60_000L
    return if (minutes > 0) "${minutes}m" else "${durationMs / 1_000L}s"
}
