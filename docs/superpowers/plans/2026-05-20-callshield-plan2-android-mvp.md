# CallShield Plan 2 — Android MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a testable Android APK that screens incoming calls and SMS using the CYNIC kernel's phone-number domain, with local cache, user reporting, ingestion, and weekly report.

**Architecture:** Thin Android client (Kotlin/Compose) with SQLite cache, backed by CYNIC kernel REST API. Call screening via `CallScreeningService`, SMS via `BroadcastReceiver`. Background workers (WorkManager) for ingestion and weekly report. No embedded ML — cache + kernel covers MVP.

**Tech Stack:** Kotlin, Jetpack Compose, Room, Retrofit, WorkManager, Android SDK 26+

**Spec:** `docs/superpowers/specs/2026-05-20-callshield-plan2-android-mvp.md`

---

## File Structure

### Kernel changes (CYNIC repo)

```
cynic-kernel/src/api/rest/
  phone_numbers.rs          # NEW — blocklist + reporter-stats handlers
  mod.rs                    # MODIFY — add module + routes
  types.rs                  # MODIFY — add BlocklistResponse, ReporterStatsResponse
```

### Android app (NEW repo: callshield-android)

```
callshield-android/
├── app/
│   ├── build.gradle.kts
│   ├── src/main/
│   │   ├── AndroidManifest.xml
│   │   └── java/com/cynic/callshield/
│   │       ├── CallShieldApp.kt
│   │       ├── data/
│   │       │   ├── local/
│   │       │   │   ├── AppDatabase.kt
│   │       │   │   ├── NumberCacheDao.kt
│   │       │   │   ├── CallLogDao.kt
│   │       │   │   └── Entities.kt
│   │       │   └── remote/
│   │       │       ├── KernelApi.kt
│   │       │       └── KernelClient.kt
│   │       ├── domain/
│   │       │   ├── VerdictLevel.kt
│   │       │   └── LocalJudge.kt
│   │       ├── service/
│   │       │   ├── ShieldScreeningService.kt
│   │       │   └── SmsFilterReceiver.kt
│   │       ├── worker/
│   │       │   ├── IngestionWorker.kt
│   │       │   └── WeeklyReportWorker.kt
│   │       └── ui/
│   │           ├── MainActivity.kt
│   │           ├── theme/Theme.kt
│   │           ├── onboarding/OnboardingScreen.kt
│   │           └── history/HistoryScreen.kt
│   ├── src/test/java/com/cynic/callshield/
│   │   ├── VerdictLevelTest.kt
│   │   ├── LocalJudgeTest.kt
│   │   └── KernelClientTest.kt
│   └── src/androidTest/java/com/cynic/callshield/
│       └── DatabaseTest.kt
├── build.gradle.kts
├── settings.gradle.kts
└── gradle.properties
```

---

## Phase A — Kernel Endpoints (CYNIC repo, 2 tasks)

### Task 1: GET /phone-numbers/blocklist endpoint

**Files:**
- Create: `cynic-kernel/src/api/rest/phone_numbers.rs`
- Modify: `cynic-kernel/src/api/rest/mod.rs:18` (add `pub mod`)
- Modify: `cynic-kernel/src/api/rest/mod.rs:66-170` (add routes)
- Modify: `cynic-kernel/src/api/rest/types.rs` (add response types)
- Test: `cynic-kernel/tests/rest_routes.rs` (add route test)

- [ ] **Step 1: Add response types to types.rs**

In `cynic-kernel/src/api/rest/types.rs`, add at the end (before closing):

```rust
/// Response for GET /phone-numbers/blocklist
#[derive(Debug, Serialize)]
pub struct BlocklistEntry {
    pub number: String,
    pub sovereignty: f64,
    pub q_score: f64,
    pub verdict: String,
}

#[derive(Debug, Serialize)]
pub struct BlocklistResponse {
    pub numbers: Vec<BlocklistEntry>,
    pub count: usize,
    pub generated_at: String,
}

/// Response for GET /phone-numbers/reporter-stats
#[derive(Debug, Serialize)]
pub struct ReporterStatsResponse {
    pub agreement_rate: Option<f64>,
    pub reports_total: u64,
    pub tier: String,
}
```

- [ ] **Step 2: Create phone_numbers.rs handler**

Create `cynic-kernel/src/api/rest/phone_numbers.rs`:

```rust
use std::sync::Arc;
use axum::{extract::{Query, State}, Json};
use serde::Deserialize;
use crate::state::AppState;
use super::types::{BlocklistResponse, BlocklistEntry, ReporterStatsResponse};

#[derive(Deserialize)]
pub struct BlocklistQuery {
    pub n: Option<usize>,
}

pub async fn blocklist_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BlocklistQuery>,
) -> Json<BlocklistResponse> {
    let n = query.n.unwrap_or(10_000).min(50_000);
    // Query verdicts for phone-number domain, sorted by lowest sovereignty
    let numbers = match &*state.storage.read().await {
        Some(storage) => {
            storage.search_verdicts_by_domain("phone-number", n).await
                .unwrap_or_default()
                .into_iter()
                .map(|v| BlocklistEntry {
                    number: v.content.clone(),
                    sovereignty: v.scores.sovereignty,
                    q_score: v.q_score,
                    verdict: v.verdict.clone(),
                })
                .collect()
        }
        None => vec![],
    };
    let count = numbers.len();
    Json(BlocklistResponse {
        numbers,
        count,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn reporter_stats_handler(
    State(_state): State<Arc<AppState>>,
) -> Json<ReporterStatsResponse> {
    // MVP: return placeholder stats. Real implementation needs
    // per-device report tracking in storage (Phase 2).
    Json(ReporterStatsResponse {
        agreement_rate: None,
        reports_total: 0,
        tier: "NEW".to_string(),
    })
}
```

- [ ] **Step 3: Wire routes in mod.rs**

In `cynic-kernel/src/api/rest/mod.rs`:

Add `pub mod phone_numbers;` with the other module declarations (~line 18).

In `router()` function, add before the `.layer()` calls:

```rust
.route("/phone-numbers/blocklist", get(phone_numbers::blocklist_handler))
.route("/phone-numbers/reporter-stats", get(phone_numbers::reporter_stats_handler))
```

Add `use axum::routing::get;` if not already imported.

- [ ] **Step 4: Add storage method stub**

The `blocklist_handler` calls `storage.search_verdicts_by_domain()`. This method may not exist. Check `StoragePort` trait and add if missing:

```rust
async fn search_verdicts_by_domain(&self, domain: &str, limit: usize) -> Result<Vec<Verdict>>;
```

If SurrealDB adapter needs implementation, query:
```sql
SELECT * FROM verdicts WHERE domain = $domain ORDER BY scores.sovereignty ASC LIMIT $limit
```

- [ ] **Step 5: cargo check + cargo test + clippy**

```bash
cargo check --workspace --all-targets
cargo test -p cynic-kernel rest_routes
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/api/rest/phone_numbers.rs cynic-kernel/src/api/rest/mod.rs cynic-kernel/src/api/rest/types.rs
git commit -m "feat(api): add /phone-numbers/blocklist + /reporter-stats endpoints"
```

---

### Task 2: Wire phone-number ingestion consumer (K15)

**Files:**
- Modify: `cynic-kernel/src/api/rest/observe.rs` (add phone ingestion routing)
- Modify: `cynic-kernel/src/pipeline/enrichment.rs:328` (implement enrich_phone)

- [ ] **Step 1: Add ingestion routing in observe handler**

In `observe.rs`, after the observation is stored, check if `tool == "callshield_ingestion"` and `domain == "phone-number"`. If so, parse the batch context and queue re-scoring for each number.

```rust
// After storing observation, check for phone ingestion
if req.tool == "callshield_ingestion" && req.domain.as_deref() == Some("phone-number") {
    // K15 consumer: ingestion data triggers re-scoring
    // Parse batch, update PhoneData aggregates, re-judge numbers
    // whose sovereignty crosses a threshold
    tracing::info!("Phone ingestion batch received from {}", req.agent_id.as_deref().unwrap_or("unknown"));
}
```

- [ ] **Step 2: cargo check + test**

```bash
cargo check --workspace --all-targets
cargo test -p cynic-kernel
```

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(pipeline): wire phone ingestion consumer stub (K15)"
```

---

## Phase B — Android App Core (new repo, 6 tasks)

### Task 3: Project scaffolding

**Files:**
- Create: `callshield-android/` (entire project structure)

- [ ] **Step 1: Create Android project**

Create new repo `callshield-android` on GitHub. Clone locally. Set up Gradle with:

```kotlin
// build.gradle.kts (project)
plugins {
    id("com.android.application") version "8.7.0" apply false
    id("org.jetbrains.kotlin.android") version "2.1.0" apply false
    id("com.google.devtools.ksp") version "2.1.0-1.0.29" apply false
}
```

```kotlin
// app/build.gradle.kts
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("com.google.devtools.ksp")
}

android {
    namespace = "com.cynic.callshield"
    compileSdk = 35
    defaultConfig {
        applicationId = "com.cynic.callshield"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // Kernel URL — configurable via build type
        buildConfigField("String", "KERNEL_URL", "\"https://cynic.example.com\"")
    }
    buildFeatures { compose = true }
    composeOptions { kotlinCompilerExtensionVersion = "1.5.15" }
}

dependencies {
    // Compose
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.activity:activity-compose:1.9.3")

    // Room
    implementation("androidx.room:room-runtime:2.6.1")
    implementation("androidx.room:room-ktx:2.6.1")
    ksp("androidx.room:room-compiler:2.6.1")

    // Network
    implementation("com.squareup.retrofit2:retrofit:2.11.0")
    implementation("com.squareup.retrofit2:converter-gson:2.11.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    // WorkManager
    implementation("androidx.work:work-runtime-ktx:2.10.0")

    // Test
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.9.0")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.room:room-testing:2.6.1")
}
```

- [ ] **Step 2: Create AndroidManifest.xml**

```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.RECEIVE_SMS" />
    <uses-permission android:name="android.permission.READ_CALL_LOG" />
    <uses-permission android:name="android.permission.READ_SMS" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />

    <application
        android:name=".CallShieldApp"
        android:label="CallShield"
        android:theme="@style/Theme.CallShield">

        <activity android:name=".ui.MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>

        <service android:name=".service.ShieldScreeningService"
            android:permission="android.permission.BIND_SCREENING_SERVICE"
            android:exported="true">
            <intent-filter>
                <action android:name="android.telecom.CallScreeningService" />
            </intent-filter>
        </service>

        <receiver android:name=".service.SmsFilterReceiver"
            android:exported="true"
            android:permission="android.permission.BROADCAST_SMS">
            <intent-filter android:priority="999">
                <action android:name="android.provider.Telephony.SMS_RECEIVED" />
            </intent-filter>
        </receiver>
    </application>
</manifest>
```

- [ ] **Step 3: Create CallShieldApp.kt**

```kotlin
package com.cynic.callshield

import android.app.Application
import androidx.work.*
import com.cynic.callshield.worker.WeeklyReportWorker
import java.util.concurrent.TimeUnit

class CallShieldApp : Application() {
    lateinit var database: com.cynic.callshield.data.local.AppDatabase
    lateinit var kernelClient: com.cynic.callshield.data.remote.KernelClient

    override fun onCreate() {
        super.onCreate()
        database = com.cynic.callshield.data.local.AppDatabase.create(this)
        kernelClient = com.cynic.callshield.data.remote.KernelClient(BuildConfig.KERNEL_URL)
        scheduleWeeklyReport()
    }

    private fun scheduleWeeklyReport() {
        val request = PeriodicWorkRequestBuilder<WeeklyReportWorker>(7, TimeUnit.DAYS)
            .build()
        WorkManager.getInstance(this)
            .enqueueUniquePeriodicWork("weekly_report", ExistingPeriodicWorkPolicy.KEEP, request)
    }
}
```

- [ ] **Step 4: Build and verify**

```bash
cd callshield-android
./gradlew assembleDebug
```

Expected: BUILD SUCCESSFUL, APK at `app/build/outputs/apk/debug/`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: scaffold CallShield Android project"
```

---

### Task 4: Room database + entities

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/data/local/Entities.kt`
- Create: `app/src/main/java/com/cynic/callshield/data/local/NumberCacheDao.kt`
- Create: `app/src/main/java/com/cynic/callshield/data/local/CallLogDao.kt`
- Create: `app/src/main/java/com/cynic/callshield/data/local/AppDatabase.kt`
- Test: `app/src/androidTest/java/com/cynic/callshield/DatabaseTest.kt`

- [ ] **Step 1: Write Entities.kt**

```kotlin
package com.cynic.callshield.data.local

import androidx.room.*

@Entity(tableName = "number_cache")
data class NumberCacheEntry(
    @PrimaryKey val number: String,     // E.164
    val sovereignty: Double,
    val qScore: Double,
    val verdict: String,                // HOWL/WAG/GROWL/BARK
    val reporterCount: Int = 0,         // for confidence gate (spec 2.1)
    val userLabel: String? = null,
    val updatedAt: Long,                // unix ms
    val source: String,                 // kernel|ingestion|cold_start
)

@Entity(tableName = "call_log")
data class CallLogEntry(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val number: String,
    val timestamp: Long,
    val direction: String,              // inbound|outbound
    val verdict: String? = null,
    val userLabel: String? = null,
    val blocked: Boolean = false,
)

@Entity(tableName = "weekly_stats")
data class WeeklyStatsEntry(
    @PrimaryKey val weekStart: Long,    // monday unix ms
    val callsBlocked: Int = 0,
    val smsFiltered: Int = 0,
    val reportsSent: Int = 0,
    val agreementRate: Double? = null,
)
```

- [ ] **Step 2: Write DAOs**

```kotlin
// NumberCacheDao.kt
package com.cynic.callshield.data.local

import androidx.room.*

@Dao
interface NumberCacheDao {
    @Query("SELECT * FROM number_cache WHERE number = :number")
    suspend fun lookup(number: String): NumberCacheEntry?

    @Upsert
    suspend fun upsert(entry: NumberCacheEntry)

    @Query("DELETE FROM number_cache WHERE updatedAt < :cutoff")
    suspend fun evictStale(cutoff: Long)

    @Query("SELECT COUNT(*) FROM number_cache")
    suspend fun count(): Int
}
```

```kotlin
// CallLogDao.kt
package com.cynic.callshield.data.local

import androidx.room.*

@Dao
interface CallLogDao {
    @Insert
    suspend fun insert(entry: CallLogEntry)

    @Query("SELECT * FROM call_log ORDER BY timestamp DESC LIMIT :limit")
    suspend fun recent(limit: Int = 50): List<CallLogEntry>

    @Query("SELECT COUNT(*) FROM call_log WHERE blocked = 1 AND timestamp > :since")
    suspend fun blockedSince(since: Long): Int

    @Query("UPDATE call_log SET userLabel = :label WHERE id = :id")
    suspend fun setLabel(id: Long, label: String)
}
```

- [ ] **Step 3: Write AppDatabase.kt**

```kotlin
package com.cynic.callshield.data.local

import android.content.Context
import androidx.room.*

@Database(
    entities = [NumberCacheEntry::class, CallLogEntry::class, WeeklyStatsEntry::class],
    version = 1,
)
abstract class AppDatabase : RoomDatabase() {
    abstract fun numberCacheDao(): NumberCacheDao
    abstract fun callLogDao(): CallLogDao

    companion object {
        fun create(context: Context): AppDatabase =
            Room.databaseBuilder(context, AppDatabase::class.java, "callshield.db")
                .build()
    }
}
```

- [ ] **Step 4: Write DatabaseTest.kt (instrumented)**

```kotlin
package com.cynic.callshield

import androidx.room.Room
import androidx.test.core.app.ApplicationProvider
import com.cynic.callshield.data.local.*
import kotlinx.coroutines.test.runTest
import org.junit.*
import org.junit.Assert.*

class DatabaseTest {
    private lateinit var db: AppDatabase
    private lateinit var cacheDao: NumberCacheDao

    @Before
    fun setup() {
        db = Room.inMemoryDatabaseBuilder(
            ApplicationProvider.getApplicationContext(),
            AppDatabase::class.java
        ).build()
        cacheDao = db.numberCacheDao()
    }

    @After
    fun teardown() { db.close() }

    @Test
    fun cacheRoundTrip() = runTest {
        val entry = NumberCacheEntry(
            number = "+33891653201",
            sovereignty = 0.17,
            qScore = 0.35,
            verdict = "BARK",
            updatedAt = System.currentTimeMillis(),
            source = "kernel",
        )
        cacheDao.upsert(entry)
        val found = cacheDao.lookup("+33891653201")
        assertNotNull(found)
        assertEquals(0.17, found!!.sovereignty, 0.001)
        assertEquals("BARK", found.verdict)
    }

    @Test
    fun cacheMissReturnsNull() = runTest {
        assertNull(cacheDao.lookup("+33000000000"))
    }
}
```

- [ ] **Step 5: Build + test**

```bash
./gradlew testDebug              # unit tests
./gradlew connectedDebugAndroidTest  # instrumented (requires device/emulator)
```

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: Room database with number cache + call log"
```

---

### Task 5: VerdictLevel + LocalJudge

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/domain/VerdictLevel.kt`
- Create: `app/src/main/java/com/cynic/callshield/domain/LocalJudge.kt`
- Test: `app/src/test/java/com/cynic/callshield/VerdictLevelTest.kt`
- Test: `app/src/test/java/com/cynic/callshield/LocalJudgeTest.kt`

- [ ] **Step 1: Write VerdictLevel.kt**

```kotlin
package com.cynic.callshield.domain

import kotlin.math.ln
import kotlin.math.min

enum class VerdictLevel(val color: String, val shouldBlock: Boolean) {
    BLOCKED("red", true),     // sovereignty < 0.25 AND confidence >= 0.30
    SUSPECT("yellow", false), // sovereignty 0.25-0.50 OR low confidence spam
    SAFE("green", false),     // sovereignty > 0.50
    UNKNOWN("gray", false);   // no data

    companion object {
        /** Confidence from reporter count per spec section 2.1:
         *  confidence = min(0.95, 1 - 1/(1 + ln(reporterCount))) */
        fun confidence(reporterCount: Int): Double =
            if (reporterCount <= 0) 0.0
            else min(0.95, 1.0 - 1.0 / (1.0 + ln(reporterCount.toDouble())))

        fun from(sovereignty: Double, reporterCount: Int): VerdictLevel {
            val conf = confidence(reporterCount)
            return when {
                sovereignty > 0.50 -> SAFE
                sovereignty < 0.25 && conf >= 0.30 -> BLOCKED
                sovereignty < 0.25 -> SUSPECT  // low confidence — don't block
                else -> SUSPECT                // 0.25-0.50 range
            }
        }
    }
}
```

- [ ] **Step 2: Write VerdictLevelTest.kt**

```kotlin
package com.cynic.callshield

import com.cynic.callshield.domain.VerdictLevel
import org.junit.Assert.*
import org.junit.Test

class VerdictLevelTest {
    @Test
    fun `confirmed spam with many reporters is BLOCKED`() {
        // 35 reporters -> confidence ~0.78 (>0.30), sovereignty < 0.25
        assertEquals(VerdictLevel.BLOCKED, VerdictLevel.from(sovereignty = 0.17, reporterCount = 35))
    }

    @Test
    fun `spam with 1 reporter is SUSPECT not BLOCKED`() {
        // 1 reporter -> confidence 0.0 (<0.30) -> never block
        assertEquals(VerdictLevel.SUSPECT, VerdictLevel.from(sovereignty = 0.17, reporterCount = 1))
    }

    @Test
    fun `ambiguous number is SUSPECT`() {
        assertEquals(VerdictLevel.SUSPECT, VerdictLevel.from(sovereignty = 0.35, reporterCount = 20))
    }

    @Test
    fun `legitimate number is SAFE`() {
        assertEquals(VerdictLevel.SAFE, VerdictLevel.from(sovereignty = 0.60, reporterCount = 10))
    }

    @Test
    fun `boundary 0_50 is SUSPECT`() {
        assertEquals(VerdictLevel.SUSPECT, VerdictLevel.from(sovereignty = 0.50, reporterCount = 20))
    }

    @Test
    fun `confidence formula at 5 reporters`() {
        val c = VerdictLevel.confidence(5)
        assertTrue("confidence(5) ~0.62, got $c", c > 0.60 && c < 0.63)
    }

    @Test
    fun `confidence zero for zero reporters`() {
        assertEquals(0.0, VerdictLevel.confidence(0), 0.001)
    }
}
```

- [ ] **Step 3: Run tests**

```bash
./gradlew testDebug --tests "com.cynic.callshield.VerdictLevelTest"
```

Expected: 6 tests PASS.

- [ ] **Step 4: Write LocalJudge.kt**

```kotlin
package com.cynic.callshield.domain

import com.cynic.callshield.data.local.NumberCacheDao
import com.cynic.callshield.data.local.NumberCacheEntry

class LocalJudge(private val cacheDao: NumberCacheDao) {

    /** Lookup a number and return its verdict level.
     *  Returns UNKNOWN if not cached or cache entry is stale. */
    suspend fun judge(number: String): JudgeResult {
        val entry = cacheDao.lookup(number) ?: return JudgeResult(VerdictLevel.UNKNOWN, null)

        // Check TTL
        val ageMs = System.currentTimeMillis() - entry.updatedAt
        val ttlMs = ttlFor(entry.sovereignty)
        if (ageMs > ttlMs) return JudgeResult(VerdictLevel.UNKNOWN, entry) // stale

        val level = VerdictLevel.from(entry.sovereignty, entry.qScore)
        return JudgeResult(level, entry)
    }

    private fun ttlFor(sovereignty: Double): Long = when {
        sovereignty < 0.25 -> 24 * 3600 * 1000L  // spam: 24h
        sovereignty > 0.50 -> 12 * 3600 * 1000L  // safe: 12h
        else -> 4 * 3600 * 1000L                  // ambiguous: 4h
    }

    data class JudgeResult(
        val level: VerdictLevel,
        val cacheEntry: NumberCacheEntry?,
    )
}
```

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: VerdictLevel decision logic + LocalJudge cache lookup"
```

---

### Task 6: KernelClient (HTTP)

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/data/remote/KernelApi.kt`
- Create: `app/src/main/java/com/cynic/callshield/data/remote/KernelClient.kt`
- Test: `app/src/test/java/com/cynic/callshield/KernelClientTest.kt`

- [ ] **Step 1: Write KernelApi.kt (Retrofit interface)**

```kotlin
package com.cynic.callshield.data.remote

import retrofit2.Response
import retrofit2.http.*

data class JudgeRequest(val content: String, val domain: String = "phone-number")
data class JudgeResponse(val q_score: Double, val verdict: String, val scores: AxiomScores?)
data class AxiomScores(val sovereignty: Double, val fidelity: Double, val phi: Double,
                        val verify: Double, val culture: Double, val burn: Double)

data class ObserveRequest(val tool: String, val target: String, val domain: String,
                           val context: String, val agent_id: String, val tags: List<String>)

data class ReporterStatsResponse(val agreement_rate: Double?, val reports_total: Long, val tier: String)

interface KernelApi {
    @POST("/judge")
    suspend fun judge(@Body request: JudgeRequest): Response<JudgeResponse>

    @POST("/observe")
    suspend fun observe(@Body request: ObserveRequest): Response<Unit>

    @GET("/phone-numbers/reporter-stats")
    suspend fun reporterStats(): Response<ReporterStatsResponse>
}
```

- [ ] **Step 2: Write KernelClient.kt**

```kotlin
package com.cynic.callshield.data.remote

import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import okhttp3.OkHttpClient
import okhttp3.Interceptor
import java.util.concurrent.TimeUnit

class KernelClient(baseUrl: String, private val deviceToken: String = "") {

    private val api: KernelApi = Retrofit.Builder()
        .baseUrl(baseUrl)
        .client(OkHttpClient.Builder()
            .connectTimeout(5, TimeUnit.SECONDS)
            .readTimeout(10, TimeUnit.SECONDS)
            .addInterceptor(authInterceptor())
            .build())
        .addConverterFactory(GsonConverterFactory.create())
        .build()
        .create(KernelApi::class.java)

    private fun authInterceptor() = Interceptor { chain ->
        val request = chain.request().newBuilder()
            .addHeader("Authorization", "Bearer $deviceToken")
            .build()
        chain.proceed(request)
    }

    suspend fun judge(number: String): JudgeResponse? {
        return try {
            val resp = api.judge(JudgeRequest(content = number))
            if (resp.isSuccessful) resp.body() else null
        } catch (e: Exception) {
            null  // Network failure — presumption of innocence
        }
    }

    suspend fun report(number: String, label: String, agentId: String) {
        try {
            api.observe(ObserveRequest(
                tool = "callshield_report",
                target = number,
                domain = "phone-number",
                context = """{"label":"$label"}""",
                agent_id = agentId,
                tags = listOf("user-report"),
            ))
        } catch (_: Exception) { /* fire and forget */ }
    }

    suspend fun reporterStats(): ReporterStatsResponse? {
        return try {
            val resp = api.reporterStats()
            if (resp.isSuccessful) resp.body() else null
        } catch (_: Exception) { null }
    }

    suspend fun observeBatch(batchJson: String) {
        try {
            api.observe(ObserveRequest(
                tool = "callshield_ingestion",
                target = "batch",
                domain = "phone-number",
                context = batchJson,
                agent_id = "device-ingestion",
                tags = listOf("device-ingestion"),
            ))
        } catch (_: Exception) { /* fire and forget */ }
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: KernelClient — Retrofit HTTP client for /judge and /observe"
```

---

### Task 7: CallScreeningService

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/service/ShieldScreeningService.kt`

- [ ] **Step 1: Implement ShieldScreeningService.kt**

```kotlin
package com.cynic.callshield.service

import android.telecom.Call
import android.telecom.CallScreeningService
import com.cynic.callshield.CallShieldApp
import com.cynic.callshield.data.local.CallLogEntry
import com.cynic.callshield.domain.LocalJudge
import com.cynic.callshield.domain.VerdictLevel
import kotlinx.coroutines.*

class ShieldScreeningService : CallScreeningService() {

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    override fun onScreenCall(callDetails: Call.Details) {
        val number = callDetails.handle?.schemeSpecificPart ?: run {
            respondToCall(callDetails, CallResponse.Builder().build())
            return
        }

        scope.launch {
            val app = application as CallShieldApp
            val judge = LocalJudge(app.database.numberCacheDao())
            val result = judge.judge(number)

            val response = when (result.level) {
                VerdictLevel.BLOCKED -> CallResponse.Builder()
                    .setDisallowCall(true)
                    .setRejectCall(true)
                    .setSkipNotification(false)
                    .build()
                VerdictLevel.SUSPECT -> CallResponse.Builder()
                    .setDisallowCall(false)
                    .build()
                else -> CallResponse.Builder().build()
            }

            respondToCall(callDetails, response)

            // Log the call
            app.database.callLogDao().insert(CallLogEntry(
                number = number,
                timestamp = System.currentTimeMillis(),
                direction = "inbound",
                verdict = result.level.name,
                blocked = result.level == VerdictLevel.BLOCKED,
            ))

            // If cache miss, async query kernel for next time
            if (result.level == VerdictLevel.UNKNOWN) {
                val kernelResult = app.kernelClient.judge(number)
                if (kernelResult != null) {
                    app.database.numberCacheDao().upsert(
                        com.cynic.callshield.data.local.NumberCacheEntry(
                            number = number,
                            sovereignty = kernelResult.scores?.sovereignty ?: 0.5,
                            qScore = kernelResult.q_score,
                            verdict = kernelResult.verdict,
                            updatedAt = System.currentTimeMillis(),
                            source = "kernel",
                        )
                    )
                }
            }
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }
}
```

- [ ] **Step 2: Build**

```bash
./gradlew assembleDebug
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: CallScreeningService — block/allow based on LocalJudge"
```

---

### Task 8: SMS filter + basic UI

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/service/SmsFilterReceiver.kt`
- Create: `app/src/main/java/com/cynic/callshield/ui/MainActivity.kt`
- Create: `app/src/main/java/com/cynic/callshield/ui/history/HistoryScreen.kt`

- [ ] **Step 1: Write SmsFilterReceiver.kt**

```kotlin
package com.cynic.callshield.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.telephony.SmsMessage
import com.cynic.callshield.CallShieldApp
import com.cynic.callshield.domain.LocalJudge
import com.cynic.callshield.domain.VerdictLevel
import kotlinx.coroutines.*

class SmsFilterReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != "android.provider.Telephony.SMS_RECEIVED") return

        val bundle = intent.extras ?: return
        val pdus = bundle.get("pdus") as? Array<*> ?: return
        val format = bundle.getString("format", "3gpp")

        val app = context.applicationContext as CallShieldApp
        val cacheDao = app.database.numberCacheDao()

        // MUST be synchronous — abortBroadcast() only works within onReceive()
        val shouldBlock = runBlocking(Dispatchers.IO) {
            for (pdu in pdus) {
                val message = SmsMessage.createFromPdu(pdu as ByteArray, format)
                val sender = message.originatingAddress ?: continue
                val entry = cacheDao.lookup(sender) ?: continue
                val level = VerdictLevel.from(entry.sovereignty, entry.reporterCount)
                if (level == VerdictLevel.BLOCKED) return@runBlocking true
            }
            false
        }

        if (shouldBlock) {
            abortBroadcast()  // Synchronous — prevents SMS from reaching default app
        }
    }
}
```

- [ ] **Step 2: Write minimal MainActivity + HistoryScreen**

```kotlin
// MainActivity.kt
package com.cynic.callshield.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.cynic.callshield.ui.history.HistoryScreen

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { HistoryScreen() }
    }
}
```

```kotlin
// HistoryScreen.kt
package com.cynic.callshield.ui.history

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun HistoryScreen() {
    Scaffold(
        topBar = { CenterAlignedTopAppBar(title = { Text("CallShield") }) }
    ) { padding ->
        LazyColumn(modifier = Modifier.padding(padding)) {
            item {
                Text(
                    "CallShield is active. Call history will appear here.",
                    modifier = Modifier.padding(16.dp),
                    style = MaterialTheme.typography.bodyLarge,
                )
            }
        }
    }
}
```

- [ ] **Step 3: Build + install on device**

```bash
./gradlew installDebug
```

Expected: App launches, shows "CallShield is active" screen.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: SMS filter + minimal UI shell"
```

---

## Phase C — Features (3 tasks)

### Task 9: IngestionWorker

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/worker/IngestionWorker.kt`

- [ ] **Step 1: Write IngestionWorker.kt**

```kotlin
package com.cynic.callshield.worker

import android.content.Context
import android.provider.CallLog
import android.provider.Telephony
import androidx.work.*
import com.cynic.callshield.CallShieldApp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class IngestionWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val app = applicationContext as CallShieldApp
        val numbers = mutableMapOf<String, NumberMetadata>()

        // Read call log
        try {
            val cursor = applicationContext.contentResolver.query(
                CallLog.Calls.CONTENT_URI,
                arrayOf(CallLog.Calls.NUMBER, CallLog.Calls.TYPE, CallLog.Calls.DATE),
                null, null, "${CallLog.Calls.DATE} DESC"
            )
            cursor?.use {
                while (it.moveToNext()) {
                    val number = it.getString(0) ?: continue
                    val type = it.getInt(1)
                    val meta = numbers.getOrPut(number) { NumberMetadata() }
                    when (type) {
                        CallLog.Calls.INCOMING_TYPE -> meta.callsInbound++
                        CallLog.Calls.OUTGOING_TYPE -> meta.callsOutbound++
                        CallLog.Calls.BLOCKED_TYPE -> { meta.callsInbound++; meta.userBlocked = true }
                    }
                }
            }
        } catch (_: SecurityException) { /* Permission not granted */ }

        // Batch POST to kernel via /observe with tool=callshield_ingestion
        val gson = com.google.gson.Gson()
        val chunks = numbers.entries.chunked(100)
        for (chunk in chunks) {
            val batch = chunk.map { (num, meta) ->
                mapOf("number" to num, "call_count_inbound" to meta.callsInbound,
                      "call_count_outbound" to meta.callsOutbound, "user_blocked" to meta.userBlocked)
            }
            app.kernelClient.observeBatch(gson.toJson(mapOf("numbers" to batch)))
        }

        Result.success()
    }

    data class NumberMetadata(
        var callsInbound: Int = 0,
        var callsOutbound: Int = 0,
        var smsInbound: Int = 0,
        var smsOutbound: Int = 0,
        var userBlocked: Boolean = false,
    )
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: IngestionWorker — batch call log to kernel"
```

---

### Task 10: WeeklyReportWorker

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/worker/WeeklyReportWorker.kt`

- [ ] **Step 1: Write WeeklyReportWorker.kt**

```kotlin
package com.cynic.callshield.worker

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.work.*
import com.cynic.callshield.CallShieldApp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class WeeklyReportWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val app = applicationContext as CallShieldApp
        val callDao = app.database.callLogDao()

        val weekAgo = System.currentTimeMillis() - 7 * 24 * 3600 * 1000L
        val blocked = callDao.blockedSince(weekAgo)

        // Fetch reporter stats from kernel (I5 fix)
        val stats = app.kernelClient.reporterStats()
        val agreementText = if (stats?.agreement_rate != null)
            " — fiabilite ${(stats.agreement_rate * 100).toInt()}%"
        else ""

        val nm = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            nm.createNotificationChannel(
                NotificationChannel("weekly", "Rapport hebdo", NotificationManager.IMPORTANCE_DEFAULT)
            )
        }

        val notification = NotificationCompat.Builder(applicationContext, "weekly")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle("Rapport CallShield")
            .setContentText("$blocked appels bloques cette semaine$agreementText")
            .setAutoCancel(true)
            .build()

        nm.notify(1001, notification)
        Result.success()
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git commit -m "feat: WeeklyReportWorker — weekly notification with stats"
```

---

### Task 11: Onboarding flow

**Files:**
- Create: `app/src/main/java/com/cynic/callshield/ui/onboarding/OnboardingScreen.kt`
- Modify: `app/src/main/java/com/cynic/callshield/ui/MainActivity.kt`

- [ ] **Step 1: Write OnboardingScreen.kt**

3-step onboarding: intro → permissions → ingestion opt-in. Uses `accompanist-permissions` or manual `ActivityResultLauncher`. Each step is a full-screen Compose page with a "Next" button.

Key interactions:
- Step 2: Request `CALL_SCREENING` role via `RoleManager`
- Step 3: Request `READ_CALL_LOG` + `READ_SMS` if user opts in, then enqueue `IngestionWorker`

- [ ] **Step 2: Wire onboarding into MainActivity**

Check SharedPreferences `onboarding_complete`. If false, show `OnboardingScreen`. After completion, show `HistoryScreen`.

- [ ] **Step 3: Build + test on device**

```bash
./gradlew installDebug
```

Manual test: fresh install shows onboarding. After completion, shows history.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: 3-step onboarding — permissions + ingestion opt-in"
```

---

## Phase D — Integration (2 tasks)

### Task 12: Cold start cache download

**Files:**
- Modify: `CallShieldApp.kt` (add cold start on first launch)
- Depends on: Task 1 (kernel blocklist endpoint)

- [ ] **Step 1: Add blocklist download on first launch**

In `CallShieldApp.onCreate()`, check SharedPreferences for `cold_start_done`. If false, enqueue a one-shot `WorkManager` request that:
1. Calls `GET /phone-numbers/blocklist?n=10000`
2. Upserts all entries into `number_cache` with source = "cold_start"
3. Sets `cold_start_done = true`

- [ ] **Step 2: Build + verify**

```bash
./gradlew installDebug
```

Check logcat for cold start download. Verify `number_cache` has entries.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat: cold start — download top-N spam numbers on first install"
```

---

### Task 13: End-to-end smoke test

- [ ] **Step 1: Manual E2E test protocol**

1. Install fresh APK on test device
2. Complete onboarding (opt-in ingestion)
3. Verify cold start populated cache (check DB with Android Studio inspector)
4. Call the test device from a known spam number (or manually add a test entry to cache)
5. Verify: call blocked (red) or labeled (yellow)
6. After call: verify report notification appears
7. Label the call → verify POST /observe sent to kernel (check kernel logs)
8. Wait for weekly report → verify notification

- [ ] **Step 2: Document results**

Log false positive / false negative counts. Compare against spec targets (FP < 5%, FN < 30%).

- [ ] **Step 3: Tag release**

```bash
git tag -a v0.1.0-alpha -m "CallShield Android MVP alpha"
git push origin v0.1.0-alpha
```

---

## Dependency Graph

```
Phase A (kernel):  Task 1 ──→ Task 12 (cold start depends on blocklist endpoint)
                   Task 2 ──→ Task 10 (weekly report uses reporter-stats)

Phase B (app):     Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8
                                                 ↓
Phase C (features):                    Task 9, Task 10, Task 11 (parallel)

Phase D (integration):                 Task 12 → Task 13
```

Tasks 9, 10, 11 are independent and can be parallelized.
Phase A and Phase B can proceed in parallel (different repos).
