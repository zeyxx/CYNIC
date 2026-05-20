# CallShield — Plan 2: Android MVP App

> **Prerequisite:** Plan 1 (kernel phone-number domain) — DONE (PR#236).
> **Parent spec:** `2026-05-19-callshield-anti-spam-app-design.md` — full product vision.
> **This spec:** What gets built for the first testable Android app.

**Date:** 2026-05-20
**Status:** Design — pending implementation plan
**Platform:** Android-first (iOS follows 4-6 weeks after Android MVP validated)

---

## 1. Scope

### 1.1 In Scope (v0.1 MVP)

| Feature | Description |
|---------|-------------|
| **Call screening** | Block/label incoming calls using 4-level system (red/yellow/green/gray) |
| **SMS filter** | Same 4-level logic applied to incoming SMS senders |
| **Opt-in ingestion** | At install, scan existing call log + SMS metadata to bootstrap registry |
| **Manual reporting** | Post-call notification: "Label this call?" (legitimate/nuisance/scam) |
| **Weekly report** | Push notification summarizing blocked calls, filtered SMS, reporter score |
| **Offline cache** | SQLite local cache for known numbers — works without network |
| **Kernel enrichment** | REST calls to CYNIC kernel for cache misses and ambiguous numbers |

### 1.2 Out of Scope (v0.1)

| Feature | When | Why deferred |
|---------|------|-------------|
| iOS app | v0.2 (4-6 weeks after Android MVP) | Android-first; `CallScreeningService` > `CallKit` |
| Phone-to-phone gossip (BLE/WiFi Direct) | v0.2+ | Needs critical mass of users first |
| Voice proxy / challenge vocal | Plan 3 | Requires telecom infrastructure |
| Federation multi-node | Plan 4 | No second node exists yet |
| Contestation by number owner | v0.2 | Needs OTP verification infrastructure |
| Embedded ML model (TFLite) | v0.2 if latency measured > 100ms | YAGNI — cache + kernel covers MVP |
| Solana anchor | Already in kernel | No app-side work needed |
| Play Store public release | After beta validation | Side-load or closed testing first |

### 1.3 Target Users (v0.1)

10-20 testers via APK side-load or Play Store closed testing track.
Android testers available immediately. iPhone testers reserved for iOS v0.2.

---

## 2. UX Design

### 2.1 Call Verdict Display

The app never shows numeric scores to users. Verdicts use a 2-axis decision (sovereignty + confidence), aligned with the parent spec's presumption-of-innocence model.

**Decision table (2-axis: sovereignty x confidence):**

| Sovereignty | Confidence | Level | Color | Action |
|------------|-----------|-------|-------|--------|
| < 0.25 | >= 0.30 | **Bloque** | Red | `disallowCall()` — block |
| < 0.25 | < 0.30 | **Suspect** | Yellow | Ring + warning (not enough data to block) |
| 0.25 - 0.50 | any | **Suspect** | Yellow | Ring + warning overlay |
| > 0.50 | any | **Sur** | Green | Normal ring |
| No data | — | **Inconnu** | Gray | Normal ring (presumption of innocence) |

**Critical rule: the app NEVER blocks when confidence < 0.30.** Blocking a legitimate call is worse than letting spam through. This matches the parent spec's decision table (section 3.1).

Confidence is derived from `reporter_count`: `confidence = min(0.95, 1 - 1/(1 + log(reporter_count)))`. Numbers with < 5 reports rarely reach confidence 0.30.

Threshold basis (conjecture — synthetic baseline, requires real-world recalibration):
- Scam mean sovereignty: 0.174 (well below 0.25 threshold)
- Legitimate mean sovereignty: 0.600 (well above 0.50 threshold)
- Gap: 0.426 — suggests separation is feasible but must be validated on real data

**Falsifiable:** If real-world false positive rate > 5% (legitimate calls blocked), thresholds need recalibration. Thresholds will be re-derived from FTC + device ingestion data during beta.

### 2.2 SMS Filter

Same 4-level logic applied to SMS sender number:
- Android: `BroadcastReceiver` on `SMS_RECEIVED`
- Filtered SMS moved to a "Filtered" folder in-app (not deleted)
- User can review and recover false positives
- No hard latency constraint (SMS can wait 200ms for kernel lookup)

### 2.3 Post-Call Report Flow

```
Call ends -> 5s delay -> notification:
  "Appel de +33 891 653 201"
  "Comment etait cet appel ?"
  [Legitime] [Nuisance] [Arnaque] [Ignorer]

User taps -> POST /observe to kernel:
  domain: "phone-number"
  target: "+33891653201"
  context: { label: "scam" }
  agent_id: SHA256(device_install_id + weekly_salt)
```

The report feeds the kernel pipeline: stimulus -> deterministic dog -> verdict update -> cache invalidation pushed to devices that cached this number.

### 2.4 Weekly Report

Push notification every Monday 9:00 local time:

```
Rapport de la semaine
  12 appels bloques
  3 SMS filtres
  Numero le plus signale : +33 891 XX XX XX
  Ton score de fiabilite : 94% d'accord avec la communaute
  [Voir le detail]
```

**K15 consumer:** The weekly report makes the app's value VISIBLE. Without it, the app blocks silently and users forget it exists. The report also shows the user's reporter reliability score — incentivizes accurate labeling.

Tapping opens a detail screen with:
- List of blocked/filtered numbers this week
- Option to contest any decision (mark as false positive/negative)
- Reporter reliability trend (last 4 weeks)

---

## 3. Opt-In Ingestion at Install

### 3.1 Flow

```
First launch -> onboarding screens (3):
  1. "CallShield protege contre le spam telephonique"
  2. Permissions: CallScreeningService + SMS filter
  3. Opt-in ingestion:
     "Voulez-vous partager vos historiques d'appels et SMS
      pour aider la communaute ? Seuls les numeros et metadonnees
      sont transmis, jamais le contenu de vos messages."
     [Oui, contribuer] [Non merci]
```

### 3.2 What Gets Transmitted (Numbers + Metadata Only)

Per contact number extracted from call log + SMS:

```json
{
  "number": "+33891653201",
  "call_count_inbound": 12,
  "call_count_outbound": 0,
  "sms_count_inbound": 3,
  "sms_count_outbound": 0,
  "first_seen": "2026-03",
  "last_seen": "2026-05",
  "user_blocked": true,
  "in_contacts": false
}
```

**What is NEVER transmitted:**
- Message content (body text)
- Contact names
- User's own phone number
- GPS / location data
- Timestamps more precise than month granularity

### 3.3 Kernel Aggregation

Across devices:

```
Number +33891653201:
  Seen on: 47 devices
  Blocked by: 38 devices (81%)
  In contacts: 2 devices (4%)
  Mean inbound calls: 8.3
  Mean outbound calls: 0.1
  -> Strong spam signal (high block rate, near-zero outbound, not in contacts)
```

Maps to PhoneData fields:
- `reporter_count` <- device count
- `mean_reporter_trust` <- device attestation score (Play Integrity)
- `label_distribution` <- derived from block rate + contact membership
- `total_events` <- sum of call+sms counts across devices

**K15 consumer (acting):** Ingestion observations are consumed by the kernel's phone-number pipeline. When a batch arrives: (1) the kernel updates `PhoneData` aggregates for each number, (2) re-runs `build_phone_stimulus -> deterministic dog` to produce a fresh verdict, (3) if the verdict crosses a threshold (e.g., sovereignty drops below 0.25), emits a cache invalidation event to devices that cached this number. The acting gate: a number that was "unknown" becomes "blocked" when enough devices report it as spam via ingestion.

### 3.4 Privacy Model

| Data | Stored locally | Transmitted | Stored on kernel |
|------|---------------|-------------|-----------------|
| Message content | Yes (phone native) | **Never** | Never |
| Contact names | Yes (phone native) | **Never** | Never |
| Phone numbers (others) | Yes (cache) | Yes (clear) | Aggregated counts only |
| Call/SMS metadata | Yes (counts) | Yes (monthly granularity) | Aggregated counts only |
| User's phone number | Yes (SIM) | **Never** | Never |
| Device ID | Generated at install | Anonymous token | Anonymous token |
| Location | Not accessed | **Never** | Never |

Numbers transmitted in clear (not hashed) because the kernel needs to aggregate reports across users. The kernel never stores which device reported which number — only aggregate counts.

**Device identity:** The `agent_id` used in `/observe` calls is `SHA256(device_install_id + weekly_salt)` — a rotating pseudonym that changes every week. This prevents the kernel from building a long-term profile of which numbers a device interacts with, while still allowing short-term Sybil detection (same agent_id reporting 50 numbers in one batch = suspect). The `weekly_salt` is derived from the ISO week number + a device-local secret generated at install.

### 3.5 Permissions Required

| Permission | Why | When requested |
|-----------|-----|---------------|
| `CALL_SCREENING` (role) | Block/label incoming calls | Onboarding step 2 |
| `RECEIVE_SMS` | Filter incoming SMS | Onboarding step 2 |
| `READ_CALL_LOG` | Ingestion of call history | Onboarding step 3 (opt-in only) |
| `READ_SMS` | Ingestion of SMS metadata | Onboarding step 3 (opt-in only) |
| `INTERNET` | Kernel communication | Auto-granted |
| `POST_NOTIFICATIONS` | Weekly report | After first blocked call |

**Play Store policy:** `READ_SMS` and `READ_CALL_LOG` require Play Console declaration. For closed testing, straightforward. For public release, may need to demonstrate core functionality or move ingestion to companion APK.

---

## 4. Architecture

### 4.1 Component Diagram

```
+---------------------------------------------------+
|  Android App (Kotlin, min SDK 26 / Android 8.0)   |
|                                                    |
|  +----------------+  +-------------------+         |
|  | CallScreening  |  | SmsFilter         |         |
|  | Service        |  | BroadcastReceiver |         |
|  +-------+--------+  +--------+----------+         |
|          |                     |                    |
|          v                     v                    |
|  +----------------------------------------+        |
|  |  LocalJudge                             |        |
|  |  SQLite cache -> lookup number          |        |
|  |  hit + high conf -> local decision      |        |
|  |  miss or ambiguous -> kernel call       |        |
|  +-------------------+--------------------+         |
|                      |                              |
|  +-------------------v--------------------+         |
|  |  KernelClient (Retrofit/Ktor)          |         |
|  |  POST /judge domain=phone-number       |         |
|  |  POST /observe (user reports)          |         |
|  |  POST /observe (ingestion batch)       |         |
|  |  GET  /health (connectivity check)     |         |
|  +----------------------------------------+         |
|                                                    |
|  +----------------------------------------+        |
|  |  IngestionWorker (WorkManager)         |         |
|  |  One-shot at install (opt-in)          |         |
|  |  Reads call log + SMS metadata         |         |
|  |  Batches to kernel /observe            |         |
|  +----------------------------------------+         |
|                                                    |
|  +----------------------------------------+        |
|  |  WeeklyReportWorker (WorkManager)      |         |
|  |  Periodic: every Monday 9:00           |         |
|  |  Reads local stats -> notification     |         |
|  +----------------------------------------+         |
|                                                    |
|  +----------------------------------------+        |
|  |  UI (Jetpack Compose)                  |         |
|  |  - Call history with verdicts           |        |
|  |  - Filtered SMS inbox                   |        |
|  |  - Weekly report detail                 |        |
|  |  - Settings (sensitivity, opt-in)       |        |
|  +----------------------------------------+         |
+---------------------------------------------------+
                    |
                    v
          +--------------------+
          |  CYNIC Kernel      |
          |  (Plan 1, live)    |
          |  /judge            |
          |  /observe          |
          |  /health           |
          +--------------------+
```

### 4.2 Local Cache (SQLite via Room)

```sql
CREATE TABLE number_cache (
  number       TEXT PRIMARY KEY,   -- E.164 format
  sovereignty  REAL NOT NULL,      -- last known sovereignty score
  q_score      REAL NOT NULL,      -- last known Q-score
  verdict      TEXT NOT NULL,      -- HOWL/WAG/GROWL/BARK
  label        TEXT,               -- user's own label if any
  updated_at   INTEGER NOT NULL,   -- unix timestamp
  source       TEXT NOT NULL       -- 'kernel' | 'ingestion' | 'cold_start'
);

CREATE TABLE call_log (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  number       TEXT NOT NULL,
  timestamp    INTEGER NOT NULL,
  direction    TEXT NOT NULL,       -- 'inbound' | 'outbound'
  verdict      TEXT,                -- verdict at time of call
  user_label   TEXT,                -- user's post-call label
  blocked      INTEGER DEFAULT 0
);

CREATE TABLE weekly_stats (
  week_start       INTEGER PRIMARY KEY, -- monday unix timestamp
  calls_blocked    INTEGER DEFAULT 0,
  sms_filtered     INTEGER DEFAULT 0,
  reports_sent     INTEGER DEFAULT 0,
  agreement_rate   REAL                 -- from kernel feedback
);
```

**Cache TTL (from parent spec section 2.5):**
- Confirmed spam (sovereignty < 0.25): TTL = 24h
- Ambiguous (0.25-0.50): TTL = 4h
- Known safe (sovereignty > 0.50): TTL = 12h
- Cache miss: no entry — query kernel on next call

### 4.3 Kernel API Contract

All calls use `Authorization: Bearer $DEVICE_TOKEN` (anonymous device token, not user credentials).

**Lookup (cache miss):**
```
POST /judge
Content-Type: application/json

{
  "content": "+33891653201",
  "domain": "phone-number"
}

-> 200: { "q_score": 0.35, "verdict": "GROWL", "scores": {...} }
-> 503: kernel degraded -> app allows call (presumption of innocence)
```

**User report:**
```
POST /observe
Content-Type: application/json

{
  "tool": "callshield_report",
  "target": "+33891653201",
  "domain": "phone-number",
  "context": "{\"label\":\"scam\",\"call_direction\":\"inbound\"}",
  "agent_id": "sha256(install_id+weekly_salt)",
  "tags": ["user-report"]
}
```

**Ingestion batch:**
```
POST /observe
Content-Type: application/json

{
  "tool": "callshield_ingestion",
  "target": "batch",
  "domain": "phone-number",
  "context": "{\"numbers\":[{\"number\":\"+33891653201\",\"call_count_inbound\":12,...}]}",
  "agent_id": "sha256(install_id+weekly_salt)",
  "tags": ["device-ingestion"]
}
```

**Cold start download:**
```
GET /phone-numbers/blocklist?n=10000
Authorization: Bearer $DEVICE_TOKEN

-> 200: { "numbers": [
    { "number": "+33891653201", "sovereignty": 0.12, "q_score": 0.31, "verdict": "BARK" },
    ...
  ], "count": 10000, "generated_at": "2026-05-20T10:00:00Z" }
-> 503: kernel degraded -> app starts with empty cache (safe default)
```

Response is paginated (10K numbers ~ 500KB). App fetches once at install, refreshes weekly.

**Reporter stats (for weekly report):**
```
GET /phone-numbers/reporter-stats
Authorization: Bearer $DEVICE_TOKEN
X-Agent-Id: SHA256(device_install_id + weekly_salt)

-> 200: { "agreement_rate": 0.94, "reports_total": 47, "tier": "ESTABLISHED" }
-> 404: device has no reports yet -> agreement_rate = null in weekly report
```

### 4.4 Cold Start Strategy

On first install (before ingestion completes):
1. App calls `GET /phone-numbers/blocklist?n=10000` to download top-N known spam numbers
2. Stored in SQLite cache with source = 'cold_start'
3. Local lookup available immediately
4. Ingestion batch runs in background via WorkManager
5. Weekly: `WeeklyReportWorker` calls `GET /phone-numbers/reporter-stats` to fetch agreement rate

---

## 5. Platform Strategy

### 5.1 Android MVP (this spec)

- **Language:** Kotlin
- **Min SDK:** 26 (Android 8.0) — covers 95%+ active devices
- **UI:** Jetpack Compose
- **Architecture:** MVVM + Room + WorkManager + Retrofit/Ktor
- **Distribution:** APK side-load or Play Store closed testing
- **Timeline target:** 4-6 weeks to first testable APK

### 5.2 iOS v0.2 (subsequent spec)

- **Language:** Swift
- **APIs:** CallKit (CXCallDirectoryExtensionProvider) + ILMessageFilterExtension
- **Limitation:** iOS `CallKit` can only LABEL calls, not block them (Apple policy). iOS does NOT expose SMS inbox to third-party apps — `READ_SMS` equivalent does not exist. `ILMessageFilterExtension` can filter INCOMING SMS but cannot read history.
- **Ingestion:** Call log only (no SMS) — Apple provides no API for SMS history access
- **Timeline:** 4-6 weeks after Android MVP validated
- **Shared:** Kernel API (identical), cache schema (SQLite), threshold logic

### 5.3 Cross-Platform Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| No Flutter/KMP | Native per platform | OS-level APIs require native; no cross-platform benefit on critical path |
| Shared cache schema | Same SQLite tables | Room (Android) and SwiftData (iOS) both wrap SQLite |
| No embedded scorer v0.1 | Cache + kernel | Avoids duplicating scorer in Kotlin AND Swift; port Rust via JNI if latency measured > 100ms |

---

## 6. Data Flows

### 6.1 Incoming Call (< 100ms target)

```
Phone rings
  -> CallScreeningService.onScreenCall(details)
  -> LocalJudge.lookup(number)
  -> Cache hit?
     YES + high conf -> immediate verdict (< 2ms)
     YES + low conf  -> async kernel enrichment, use cache verdict now
     NO              -> async kernel POST /judge, allow call (gray/unknown)
  -> Display verdict overlay
  -> Log to call_log table
  -> 5s after call ends: report notification
```

### 6.2 Incoming SMS

```
SMS received
  -> SmsFilter BroadcastReceiver
  -> LocalJudge.lookup(sender_number)
  -> Same logic as calls
  -> If red/yellow: move to "Filtered" folder
  -> If green/gray: normal inbox
```

### 6.3 Ingestion (one-shot at install)

```
User opts in
  -> IngestionWorker scheduled (WorkManager)
  -> Read CallLog.Calls (READ_CALL_LOG)
  -> Read SMS inbox metadata (READ_SMS) — numbers + dates only
  -> Aggregate per number: counts, direction, blocked, contact membership
  -> Batch POST /observe to kernel (chunks of 100 numbers)
  -> Mark ingestion complete in SharedPreferences
```

### 6.4 Weekly Report

```
Every Monday 9:00 (WorkManager PeriodicWorkRequest)
  -> Query call_log: count blocked, filtered, top number
  -> Query weekly_stats: agreement_rate
  -> Build + display notification
  -> Store in weekly_stats table
```

---

## 7. Testing Strategy

### 7.1 Automated Tests

| Test | What | How |
|------|------|-----|
| Cache round-trip | Insert -> lookup -> verify verdict | Room in-memory DB |
| Threshold mapping | Sovereignty 0.17 -> red, 0.60 -> green | Unit test on LocalJudge |
| Kernel client | Verify request format | MockWebServer |
| Ingestion extraction | Verify batch format | ContentProvider mock |
| SMS filter | Verify filtered/passed | BroadcastReceiver test |

### 7.2 Integration Tests

| Test | What |
|------|------|
| Kernel round-trip | POST /judge with real number -> verify response parses |
| Report -> verdict update | POST /observe -> re-query -> verify score changed |
| Cold start download | Install -> verify cache populated |

### 7.3 Manual Testing (10-20 testers, 2 weeks)

1. Install APK on tester devices
2. Opt-in ingestion on 50%+ of devices
3. Observe blocked calls / filtered SMS over 1 week
4. Collect false positive rate (legit calls blocked)
5. Collect false negative rate (spam that got through)
6. Target: FP < 5%, FN < 30%

---

## 8. Success Criteria

| Metric | Target | How measured |
|--------|--------|-------------|
| False positive rate | < 5% | User corrections after block |
| App retention (7 day) | > 60% | Play Console or manual |
| Reports per user per week | > 2 | Kernel /observe count |
| Weekly report open rate | > 30% | Notification tap tracking |
| Ingestion opt-in rate | > 50% | Onboarding funnel |
| Kernel latency P95 | < 200ms | Kernel logs |

**Falsifiable:** FP > 10% after 2 weeks -> recalibrate thresholds. Retention < 40% -> rethink UX. Ingestion opt-in < 30% -> rework onboarding copy.

---

## 9. Data Collection — Real Ground Truth

### 9.1 Current State

- Synthetic dataset: 50 entries, benchmark passed (sovereignty gap 0.426)
- Accuracy on synthetic: not measurable (circular)
- Real data needed for actual accuracy measurement

### 9.2 Sources

| Source | Coverage | Access |
|--------|----------|--------|
| FTC DNC API | US phone complaints, daily updates | Public REST API |
| signal-arnaques.com | FR community reports (522K+) | Web scraping |
| Device ingestion | Real call/SMS metadata from testers | Opt-in via app |

### 9.3 Accuracy Measurement

Once real data collected:
1. Run through deterministic dog
2. Compare verdict to ground truth
3. Compute precision/recall/F1 per category
4. Target: F1 > 0.70 (scam), F1 > 0.60 (nuisance)

---

## 10. Dependencies

| Dependency | Status | Blocker? |
|-----------|--------|----------|
| Kernel /judge | Live | No |
| Kernel /observe | Live | No |
| phone-number domain | Wired (PR#236) | No |
| Deterministic phone dog | Benchmarked | No |
| enrich_phone() | Stub | No (enrichment from ingestion + reports) |
| Android dev environment | To set up | Yes |
| Play Store dev account | To verify | Yes for closed testing |
| Kernel reachable from mobile | Tailscale Funnel | Partial — token distribution |

---

## 11. Plan Relationships

```
Plan 1 (kernel domain) --- DONE (PR#236)
    |
    v
Plan 2 (this spec) --- Android MVP
    |                   v0.1: block/label/report/ingest/weekly
    |                   v0.2: iOS, gossip, contestation
    |
    v
Plan 3 (voice proxy) --- Blocked by Plan 2
    |
Plan 4 (federation) ---- Independent, parallel to Plan 2
```
