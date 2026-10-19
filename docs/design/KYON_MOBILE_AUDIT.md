# Kyon Mobile — Audit and Refactor Basis

## Scope

Audit of `apps/kyon-mobile` after the build-repair and social-shield design work.

## Build state

- `./gradlew :app:assembleDebug` passes.
- No emulator AVD is available in this environment.
- No instrumented or JVM unit tests exist today.

## Inventory and status

### Core

| File | Status | Notes |
|---|---|---|
| `core/model/ActivityEvent.kt` | Live | Shared event shape and enums. |
| `core/sensing/Sensor.kt` | Live but thin | Interface only; only implemented by usage collector. |
| `core/storage/ActivityEventDao.kt` | Live | Room DAO; used by repository. |
| `core/storage/ActivityEventEntity.kt` | Live | Room entity and mapping. |
| `core/storage/EventRepository.kt` | Live | Central local repository; recently repaired. |
| `core/storage/KyonDatabase.kt` | Live | Database declaration. |
| `core/sync/KernelConfig.kt` | Idle | No runtime instantiation found. |
| `core/sync/KernelObserveClient.kt` | Idle | Duplicate observe path; not called by any worker. |

### Application

| File | Status | Notes |
|---|---|---|
| `KyonApp.kt` | Live | Owns database and schedules periodic sync. |

### Mirror

| File | Status | Notes |
|---|---|---|
| `mirror/episodes/EpisodeBuilder.kt` | Orphan | No caller. |
| `mirror/identity/IdentitySnapshot.kt` | Orphan | No caller. |
| `mirror/notifications/KyonNotificationListenerService.kt` | Live by manifest | Records notification metadata only. |
| `mirror/sync/KernelSyncWorker.kt` | Live by manifest/runtime | Sync path; duplicates HTTP logic; depends on `EventRepository`. |
| `mirror/usage/UsageSnapshotCollector.kt` | Live | Implements `Sensor`; records daily app usage metadata. |

### Shield

| File | Status | Notes |
|---|---|---|
| `shield/calls/KyonCallScreeningService.kt` | Live by manifest | Records call metadata; always allows call. |
| `shield/reporting/PhoneReport.kt` | Orphan | Model only. |
| `shield/sms/KyonSmsReceiver.kt` | Live by manifest | Records SMS metadata; now classifies first contact locally. |
| `shield/grooming/GroomingPolicy.kt` | Live | Pure local decision. |

### UI

| File | Status | Notes |
|---|---|---|
| `ui/MainActivity.kt` | Live | Main projection; permissions, snapshot capture, recent event list, local anti-grooming card. |

## Main findings

1. **Orphans are present**:
   - `EpisodeBuilder`
   - `IdentitySnapshot`
   - `PhoneReport`
   - `KernelConfig`
   - `KernelObserveClient`
2. **Duplicate sync implementation**:
   - `KernelSyncWorker` builds its own HTTP request.
   - `KernelObserveClient` is a parallel but unused implementation.
   - Neither is currently wired to real kernel configuration.
3. **Coupling issue**:
   - Sync is scheduled unconditionally in `KyonApp`.
   - Upload eligibility is now safer for usage snapshots, but sync behavior is still not user-controlled.
4. **Manifest overbreadth**:
   - Phone, call-log, SMS and usage-stats permissions are declared together.
   - Shield-first approach suggests splitting permissions by activated capability.
5. **FCIS gaps**:
   - Pure local policy exists for grooming first contact.
   - Room and Android APIs are still mixed directly in receivers and UI.
   - There is no clear port boundary between local sensing and local persistence.

## Refactor order

1. Remove or quarantine confirmed orphans.
2. Collapse sync implementations into one opt-in adapter.
3. Split permission and capability activation from `MainActivity`.
4. Introduce local ports for event persistence and shield policy.
5. Implement Kyon Social Shield vertical on the reduced surface:
   - onboarding boundaries;
   - manual concern intake;
   - guidance cards;
   - encrypted local locker;
   - explicit export.
6. Add JVM tests for repository mapping and pure policy.

## Non-goals for this refactor

- No automatic social-app content reading.
- No parent or sibling dashboard.
- No remote monitoring.
- No automatic kernel reports.
- No diagnostic claims.
