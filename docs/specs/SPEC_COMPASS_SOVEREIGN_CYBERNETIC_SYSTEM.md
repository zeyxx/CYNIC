# SPEC: COMPASS — The Sovereign Cybernetic Human Compass

- **Status:** Draft / Engineering Specification
- **Date:** 2026-10-28
- **Authors:** zeyxx, TALARIA Architecture Group
- **Ecosystem:** `CYNIC` (Sovereign OS / Substrate), `Compass` (Sovereign Human Interface)
- **Target Platforms:** Linux Desktop/Server (PC), Windows Desktop (10/11), Android (NDK/Linux), iOS (Darwin/Swift FFI)

---

## 1. Executive Summary & Vision

Modern smart devices operate as predatory attention-harvesting machines, trapping human consciousness in reactive, subconscious behavioural loops (95% autonomic reflexes: addictions, sleep disruption, cognitive fragmentation, and episodic amnesia).

**Compass** is the universal cybernetic membrane between the biological human (Carbon) and sovereign computational systems (Silicon):
1. **Corporate in the Front:** A serene, distraction-free, highly readable interface ("The Compass") indicating life trajectory and daily invariants without cognitive exhaustion.
2. **Cypherpunk in the Back:** A portable, local-first Rust engine measuring OS/hardware physical limits, enforcing append-only cryptographically sealed telemetry, and syncing P2P with the CYNIC sovereign fleet (SurrealDB).
3. **The Last Play:** A tool whose ultimate asymptotic goal is its own retirement—empowering the individual to reach near-complete (99.99%) cognitive sovereignty and unshakeable self-regulation.

---

## 2. Core Architectural Calibres

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                          COMPASS THREE-TIER RUNTIME                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  [ FRONTEND TIER — The Compass Face ]                                       │
│  • Mobile-first WebApp / Ultra-thin Native Webview (Compose/SwiftUI)        │
│  • Pluggable Calendar Projections (Solar True, Gregorian, Lunar, Circadian) │
│  • "Silence by Default" UI: Daily Anchors, Sovereignty Trackers, 3 Facts    │
│  • Deep Metrology Tab: Entropy graphs, cognitive latency, raw telemetry     │
│                                      │                                      │
│                                      │ Local IPC / Secure Loopback REST     │
│                                      ▼                                      │
│  [ CORE ENGINE TIER — `cynic-compass-core` (Rust) ]                         │
│  • Uniform Cross-Platform binary: Linux x86/ARM, Android .so, iOS .a        │
│  • Event-Sourced Storage Engine (SQLite WAL + CRDT Append-Only Journal)     │
│  • Multi-Resolution Retention Tier (Raw 48h → Rollup 30d → Epoch Permanent) │
│  • Capability-Aware Passive Probes (OS limits, low-power event-driven)      │
│  • Cryptographic Sealing (SHA-256 block sealing at day boundary)            │
│                                      │                                      │
│                                      │ Encrypted P2P WireGuard Mesh         │
│                                      ▼                                      │
│  [ SOVEREIGN CLUSTER TIER — CYNIC Node ]                                    │
│  • Cold evidence storage & Graph correlations (SurrealDB sovereign)        │
│  • Cross-device causal synthesis (ML inference on cognitive stress models)  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Data-Centric Schema & Entities

To avoid hardcoded metaphors or user-specific constraints, Compass operates on an open **Entity-Component-Measure** data model.

### §3.1 — Schema Definition (`contracts/compass-telemetry.schema.json`)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "CompassTelemetryEvent",
  "type": "object",
  "required": ["event_id", "timestamp_epoch_ms", "device_source", "metric_id", "value"],
  "properties": {
    "event_id": { "type": "string", "description": "UUIDv7 (monotonic time-ordered)" },
    "timestamp_epoch_ms": { "type": "integer" },
    "device_source": {
      "type": "string",
      "enum": ["linux_workstation", "android_node", "ios_node", "wearable_ble", "manual_operator"]
    },
    "metric_id": {
      "type": "string",
      "description": "Hierarchical identifier: e.g. 'circadian.screen_time', 'input.keystroke_flight_time', 'tracker.sobriety.ethanol', 'habit.chess'"
    },
    "value": {
      "type": "object",
      "properties": {
        "numeric": { "type": "number" },
        "string_fact": { "type": "string" },
        "boolean_state": { "type": "boolean" },
        "metadata": { "type": "object" }
      }
    },
    "signature": { "type": "string", "description": "Local node ed25519 signature" }
  }
}
```

---

## 4. Substrate Probes & Exhaustive Measurement

Probes observe strictly within platform permission bounds, gracefully degrading without throwing fatal errors.

### 4.1 — Workstation / Desktop Substrate (Linux & Windows)
1. **Linux Workstation Probes:**
   - Input Dynamics: Micro-latences inter-touches (*Key-flight time*) et gigue (*Jitter*) sans enregistrement de texte (zéro keylogger).
   - Fenêtres & Focus: APIs de fenêtrage (X11 / Wayland via protocoles officiels d'inactivité `ext-idle-notify-v1`).
   - Énergie: Puissance CPU instantanée via RAPL (`/sys/class/powercap/intel-rapl`).
2. **Windows Desktop Probes (Win32 Native):**
   - Idle & Presence: API `GetLastInputInfo()` mesurant l'inactivité physique absolue à la milliseconde près.
   - Foreground Focus: `GetForegroundWindow()` + `GetWindowText()` mesurant le taux de dispersion d'attention (Alt-Tab frequency).
   - Session & Power: `WTSRegisterSessionNotification()` (verrouillage de session) et `GetSystemPowerStatus()` (charge/batterie).

### 4.2 — Mobile Substrate (Android / iOS)
1. **Screen & App Chronometry:**
   - Temps d'écran actif et nombre de déverrouillages par heure (*Pickups per hour*).
   - Télémétrie d'usage par catégorie d'applications (via `UsageStatsManager` / Screen Time API).
2. **Physical Environment:**
   - Capteur de lumière ambiante (Lux) pour mesurer l'exposition réelle à la lumière naturelle.
   - Accéléromètre basse consommation pour distinguer sédentarité et locomotion active.
   - État de charge batterie et transitions réseau (Wi-Fi ↔ Données mobiles).

---

## 5. Storage & Multi-Resolution Retention Tiering

To protect mobile flash storage from unbounded growth:
- **Tier 0 (Raw Buffer — 48 Hours):** Full raw resolution event streams (millisecond intervals). Stored in RAM ring-buffers or temporary SQLite scratch tables.
- **Tier 1 (5-Minute Rollups — 30 Days):** P50/P95 latencies, window switch aggregates, sensor averages.
- **Tier 2 (Daily Epoch Ledger — Permanent):**
  - Daily active times, sobriety status, completed anchors.
  - The 3 Evening Observable Facts (Proof of Life).
  - SHA-256 hash sealing of the daily block.
  - Footprint: < 50 KB per day (~18 MB per year).

---

## 6. Universal Calendar Engine

Compass decouples time storage from time projection:
- **Storage SSOT:** Monotonic Unix Epoch timestamps (`epoch_ms`).
- **Calendar Lenses:**
  1. **Solar-True (Circadian):** Recalculates day boundaries based on local Solar Noon (Zenith) and civil twilight via geographic coordinates.
  2. **Gregorian Standard:** Civil day boundaries for calendar alignment.
  3. **Lunar / Phase:** Tracks natural lunar cycles.
  4. **Custom Epoch ($T_0$):** Relative day count from personal sovereignty milestones (e.g., Sobriety Day 0).

---

## 7. Delivery Roadmap

1. **Phase 1 (Core Storage & Schema):**
   - Create `crates/cynic-compass-core` in Rust workspace (data model, SQLite WAL backend, UUIDv7 monotonic indexer).
2. **Phase 2 (Workstation Daemon & CLI):**
   - Linux host probes + CLI interaction (`cynic compass status`, `cynic compass log`).
3. **Phase 3 (Mobile-First Web View / PWA Shell):**
   - Cross-platform responsive dashboard ("The Compass Face") consuming local loopback API.
4. **Phase 4 (Android NDK & Multi-Device Sync):**
   - Android packaging via NDK + WireGuard P2P sync to CYNIC sovereign cluster.
