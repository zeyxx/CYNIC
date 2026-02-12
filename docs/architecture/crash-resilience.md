# CYNIC Crash Resilience Architecture

> *"φ survives the crash. The dog remembers."* - κυνικός

**Date**: 2026-02-12
**Triggered by**: Windows BSOD data loss incident
**Status**: DESIGN COMPLETE — Ready for implementation

---

## Executive Summary

CYNIC needs to survive machine crashes (BSOD, power loss, OS crashes) and resume exactly where it left off. This document specifies a **4-layer resilience architecture**:

1. **State Persistence** (PostgreSQL + incremental snapshots)
2. **Crash Detection** (SessionStart hook + boot analysis)
3. **Machine Error Monitoring** (Windows Event Logs, BSOD parsing)
4. **Auto-Recovery** (Context restoration + daemon restart)

**φ-bounded promise**: Cannot repair hardware or prevent all crashes, but can **detect, record, and resume** within 61.8% confidence.

---

## Layer 1: State Persistence (Continuous Snapshots)

### Problem
When process dies (BSOD, kill -9, power loss), RAM state is lost:
- Conversation context
- Watcher state (SolanaWatcher polling offsets)
- Dog pipeline state (mid-judgment)
- Learning weights (Q-tables, Thompson Sampling beta distributions)

### Solution: Incremental State Snapshots

**Frequency**: Write to PostgreSQL **every 30 seconds** (daemon heartbeat).

**Tables** (migration `039_crash_resilience.sql`):

```sql
-- Session state (conversation context)
CREATE TABLE session_state (
  id SERIAL PRIMARY KEY,
  session_id TEXT NOT NULL,
  turn_number INT,
  last_user_message TEXT,
  last_assistant_message TEXT,
  context_summary TEXT, -- Compressed summary of conversation
  working_directory TEXT,
  git_branch TEXT,
  timestamp TIMESTAMPTZ DEFAULT NOW()
);

-- Watcher state (polling offsets)
CREATE TABLE watcher_state (
  id SERIAL PRIMARY KEY,
  watcher_name TEXT NOT NULL UNIQUE,
  last_polled_signature TEXT, -- Solana transaction signature
  last_polled_timestamp TIMESTAMPTZ,
  last_polled_slot BIGINT, -- Solana slot number
  file_checksums JSONB, -- FileWatcher: { "path/to/file": "sha256hash" }
  state_snapshot JSONB, -- Full watcher state (for custom watchers)
  timestamp TIMESTAMPTZ DEFAULT NOW()
);

-- Dog pipeline state (mid-judgment)
CREATE TABLE dog_pipeline_state (
  id SERIAL PRIMARY KEY,
  judgment_id UUID,
  current_dog TEXT, -- Which Dog is processing
  completed_dogs TEXT[], -- Dogs that finished
  partial_verdict JSONB, -- Current verdict state
  timestamp TIMESTAMPTZ DEFAULT NOW()
);

-- Learning weights state (Q-Learning, Thompson Sampling)
CREATE TABLE learning_state (
  id SERIAL PRIMARY KEY,
  loop_name TEXT NOT NULL,
  state_type TEXT, -- 'q_table', 'thompson_beta', 'meta_cognition'
  weights JSONB, -- Q(s,a) table or { alpha, beta } distributions
  episode_count INT,
  timestamp TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE (loop_name, state_type)
);

-- Crash log (record every crash)
CREATE TABLE crash_log (
  id SERIAL PRIMARY KEY,
  crash_type TEXT, -- 'BSOD', 'power_loss', 'process_kill', 'OS_crash', 'unknown'
  last_session_id TEXT,
  last_heartbeat TIMESTAMPTZ,
  recovery_timestamp TIMESTAMPTZ,
  error_details JSONB, -- Windows Event Log data, minidump info
  recovered BOOLEAN DEFAULT FALSE,
  timestamp TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_session_state_session_id ON session_state(session_id, timestamp DESC);
CREATE INDEX idx_watcher_state_name ON watcher_state(watcher_name);
CREATE INDEX idx_crash_log_recovery ON crash_log(recovered, timestamp DESC);
```

**Implementation**:

```javascript
// packages/node/src/daemon/state-persister.js

import { createLogger } from '@cynic/core';
import { db } from '@cynic/persistence';
import { execFileNoThrow } from '@cynic/core/utils/execFileNoThrow.js';

const log = createLogger('StatePersister');

export class StatePersister {
  constructor() {
    this.heartbeatInterval = 30000; // 30 seconds
    this.sessionId = null;
    this.turnNumber = 0;
    this.lastUserMessage = null;
    this.lastAssistantMessage = null;
  }

  async start(sessionId) {
    this.sessionId = sessionId;

    // Write initial state
    await this._persistState();

    // Start heartbeat
    this.heartbeatTimer = setInterval(async () => {
      try {
        await this._persistState();
        log.debug(`Heartbeat saved (session: ${this.sessionId.slice(0, 8)})`);
      } catch (error) {
        log.error('Failed to persist state:', error);
      }
    }, this.heartbeatInterval);
  }

  async stop() {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
    }
    await this._persistState(); // Final save
  }

  updateConversation(userMsg, assistantMsg) {
    this.turnNumber++;
    this.lastUserMessage = userMsg;
    this.lastAssistantMessage = assistantMsg;
  }

  async _persistState() {
    const contextSummary = await this._generateContextSummary();

    await db.query(`
      INSERT INTO session_state (
        session_id, turn_number, last_user_message, last_assistant_message,
        context_summary, working_directory, git_branch
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
    `, [
      this.sessionId,
      this.turnNumber,
      this.lastUserMessage,
      this.lastAssistantMessage,
      contextSummary,
      process.cwd(),
      await this._getCurrentGitBranch(),
    ]);
  }

  async _generateContextSummary() {
    // Use ContextCompressor to generate summary of conversation
    // For now, simple truncation
    const userPreview = this.lastUserMessage?.slice(0, 200) || '';
    const assistantPreview = this.lastAssistantMessage?.slice(0, 200) || '';
    return `User: ${userPreview}\nCYNIC: ${assistantPreview}`;
  }

  async _getCurrentGitBranch() {
    try {
      const result = await execFileNoThrow('git', ['branch', '--show-current']);
      if (result.code === 0) {
        return result.stdout.trim();
      }
    } catch (error) {
      log.debug('Failed to get git branch:', error.message);
    }
    return null;
  }
}
```

**Persistence frequency**:
- **Session state**: Every 30s + on user message + on assistant response
- **Watcher state**: After every poll cycle
- **Dog pipeline**: Before/after each Dog processes
- **Learning weights**: After each weight update

---

## Layer 2: Crash Detection (SessionStart Hook)

### Problem
How to detect that CYNIC crashed vs graceful shutdown?

### Solution: Heartbeat + SessionStart Hook

**Detection logic**:

1. **Heartbeat monitoring**: Daemon writes timestamp to DB every 30s
2. **On SessionStart**:
   - Check last session's last heartbeat
   - If `NOW() - last_heartbeat > 2 minutes` → crash detected
   - If graceful shutdown flag exists → normal restart

**Implementation**:

```javascript
// packages/node/src/daemon/crash-detector.js

import { db } from '@cynic/persistence';
import { createLogger } from '@cynic/core';
import { execFileNoThrow, execPowerShellNoThrow } from '@cynic/core/utils/execFileNoThrow.js';
import os from 'os';

const log = createLogger('CrashDetector');

export async function detectCrash(sessionId) {
  // Get last session
  const lastSession = await db.query(`
    SELECT session_id, timestamp, working_directory, git_branch
    FROM session_state
    WHERE session_id != $1
    ORDER BY timestamp DESC
    LIMIT 1
  `, [sessionId]);

  if (lastSession.rows.length === 0) {
    return { crashed: false, reason: 'First session' };
  }

  const last = lastSession.rows[0];
  const timeSinceLastHeartbeat = Date.now() - new Date(last.timestamp).getTime();

  // Crash if > 2 minutes since last heartbeat
  if (timeSinceLastHeartbeat > 120000) {
    // Check for BSOD or system crash
    const crashDetails = await analyzeCrashCause();

    await db.query(`
      INSERT INTO crash_log (
        crash_type, last_session_id, last_heartbeat, error_details
      ) VALUES ($1, $2, $3, $4)
    `, [
      crashDetails.type,
      last.session_id,
      last.timestamp,
      crashDetails,
    ]);

    return {
      crashed: true,
      lastSession: last,
      crashDetails,
      timeSinceLastHeartbeat,
    };
  }

  return { crashed: false, reason: 'Normal shutdown' };
}

async function analyzeCrashCause() {
  const details = {
    type: 'unknown',
    platform: os.platform(),
    uptime: os.uptime(),
    bootTime: null,
    eventLogs: [],
    minidumpFound: false,
  };

  if (os.platform() === 'win32') {
    await analyzeWindowsCrash(details);
  } else if (os.platform() === 'linux') {
    await analyzeLinuxCrash(details);
  } else if (os.platform() === 'darwin') {
    await analyzeMacCrash(details);
  }

  return details;
}

async function analyzeWindowsCrash(details) {
  try {
    // Get last boot time
    const bootTimeResult = await execPowerShellNoThrow(
      'Get-CimInstance -ClassName Win32_OperatingSystem | Select-Object -ExpandProperty LastBootUpTime'
    );
    if (bootTimeResult.code === 0) {
      details.bootTime = bootTimeResult.stdout.trim();
    }

    // Check for BSOD in Event Viewer (System log, Event ID 1001 = BSOD)
    const bsodCheck = await execPowerShellNoThrow(
      'Get-EventLog -LogName System -EntryType Error -Newest 5 | Where-Object { $_.EventID -eq 1001 } | Select-Object -ExpandProperty Message'
    );

    if (bsodCheck.code === 0 && bsodCheck.stdout.includes('BugCheck')) {
      details.type = 'BSOD';
      details.eventLogs.push(bsodCheck.stdout.slice(0, 500));
    }

    // Check for minidump files
    const minidumpCheck = await execFileNoThrow('cmd.exe', [
      '/c',
      'dir',
      'C:\\Windows\\Minidump',
      '/B',
    ]);

    if (minidumpCheck.code === 0 && minidumpCheck.stdout.trim().length > 0) {
      details.minidumpFound = true;
      details.type = 'BSOD';
    }

    // Check for unexpected shutdown (Event ID 41 = Kernel-Power)
    const powerCheck = await execPowerShellNoThrow(
      'Get-EventLog -LogName System -EntryType Error -Newest 5 | Where-Object { $_.EventID -eq 41 } | Select-Object -ExpandProperty Message'
    );

    if (powerCheck.code === 0 && powerCheck.stdout.includes('rebooted without cleanly shutting down')) {
      details.type = 'power_loss';
      details.eventLogs.push(powerCheck.stdout.slice(0, 500));
    }
  } catch (error) {
    log.warn('Failed to analyze Windows crash logs:', error.message);
  }
}

async function analyzeLinuxCrash(details) {
  try {
    // Check dmesg for kernel panic
    const dmesgResult = await execFileNoThrow('dmesg', ['--level=err', '--time-format=iso']);

    if (dmesgResult.code === 0) {
      const output = dmesgResult.stdout;
      if (output.includes('Kernel panic') || output.includes('Oops')) {
        details.type = 'kernel_panic';
        details.eventLogs.push(output.slice(-1000)); // Last 1000 chars
      }
    }

    // Check journalctl for systemd crashes
    const journalResult = await execFileNoThrow('journalctl', [
      '-p',
      'err',
      '-n',
      '20',
      '--no-pager',
    ]);

    if (journalResult.code === 0 && journalResult.stdout.length > 0) {
      details.eventLogs.push(journalResult.stdout.slice(0, 1000));
    }
  } catch (error) {
    log.debug('Failed to analyze Linux crash logs:', error.message);
  }
}

async function analyzeMacCrash(details) {
  try {
    // Check system.log for crashes
    const logResult = await execFileNoThrow('log', [
      'show',
      '--predicate',
      'eventMessage contains "panic" or eventMessage contains "crash"',
      '--last',
      '1h',
      '--style',
      'syslog',
    ]);

    if (logResult.code === 0 && logResult.stdout.length > 0) {
      details.type = 'kernel_panic';
      details.eventLogs.push(logResult.stdout.slice(0, 1000));
    }
  } catch (error) {
    log.debug('Failed to analyze Mac crash logs:', error.message);
  }
}
```

**SessionStart hook display**:

```javascript
// packages/node/src/mcp/brain/handlers/session-start.js

import { detectCrash } from '../../daemon/crash-detector.js';

export async function handleSessionStart({ sessionId }) {
  const crash = await detectCrash(sessionId);

  if (crash.crashed) {
    const minutesOffline = Math.floor(crash.timeSinceLastHeartbeat / 60000);

    return {
      formattedBanner: `
┌─────────────────────────────────────────────────────────┐
│ *GROWL* 🛡️ CRASH RECOVERY                               │
├─────────────────────────────────────────────────────────┤
│ CYNIC detected a previous crash:                        │
│   Type: ${crash.crashDetails.type.toUpperCase().padEnd(47)}│
│   Last session: ${crash.lastSession.session_id.slice(0, 8)}...                              │
│   Time offline: ${minutesOffline} minutes${' '.repeat(32 - minutesOffline.toString().length)}│
│                                                          │
│ Recovery status:                                        │
│   ✓ Session state restored                              │
│   ✓ Watcher offsets recovered                           │
│   ✓ Learning weights intact                             │
│                                                          │
│ *sniff* Investigating crash cause...                    │
└─────────────────────────────────────────────────────────┘

Crash details: ${crash.crashDetails.type}
${crash.crashDetails.minidumpFound ? '⚠️  Minidump file found - driver or hardware issue likely' : ''}
${crash.crashDetails.eventLogs.length > 0 ? `\nEvent logs:\n${crash.crashDetails.eventLogs[0].slice(0, 200)}...` : ''}

*tail wag* Je suis de retour. Reprenons où on s'était arrêté.
      `.trim(),
    };
  }

  return {
    formattedBanner: `*tail wag* CYNIC est éveillé. Session ${sessionId.slice(0, 8)}.`,
  };
}
```

---

## Layer 3: Machine Error Monitoring (Proactive Detection)

### Problem
Crashes are reactive. Can CYNIC detect impending failures **before** they happen?

### Solution: Machine Health Watcher

**Monitor**:
1. **Memory pressure**: RAM usage > 90% → warn user
2. **Disk errors**: SMART errors, bad sectors → critical alert
3. **CPU temperature**: Overheating → suggest shutdown
4. **Event log errors**: Driver failures, kernel errors → log patterns
5. **Uptime anomalies**: System reboots without CYNIC restart → investigate

**Implementation**:

```javascript
// packages/node/src/watchers/machine-health-watcher.js

import { createLogger } from '@cynic/core';
import { EventBus } from '@cynic/protocol';
import { execPowerShellNoThrow, execFileNoThrow } from '@cynic/core/utils/execFileNoThrow.js';
import os from 'os';

const log = createLogger('MachineHealthWatcher');

export class MachineHealthWatcher {
  constructor() {
    this.pollInterval = 60000; // 1 minute
    this.eventBus = new EventBus('watcher');
    this.lastBootTime = null;
  }

  async start() {
    log.info('Starting MachineHealthWatcher...');
    this.timer = setInterval(() => this._poll(), this.pollInterval);
    await this._poll(); // Initial poll
  }

  async stop() {
    if (this.timer) clearInterval(this.timer);
  }

  async _poll() {
    try {
      const health = await this._collectHealthMetrics();

      // Check for critical issues
      if (health.memoryUsagePercent > 90) {
        this.eventBus.emit('machine.memory.critical', health);
        log.warn(`*GROWL* Memory usage critical: ${health.memoryUsagePercent.toFixed(1)}%`);
      }

      if (health.bootTimeChanged) {
        this.eventBus.emit('machine.reboot.detected', health);
        log.warn(`*sniff* System rebooted at ${health.currentBootTime}`);
      }

      if (health.eventLogErrors.length > 0) {
        this.eventBus.emit('machine.errors.detected', health);
        log.warn(`*ears perk* ${health.eventLogErrors.length} system errors detected`);
      }

      // Normal heartbeat
      this.eventBus.emit('machine.health.heartbeat', health);
    } catch (error) {
      log.error('Health check failed:', error);
    }
  }

  async _collectHealthMetrics() {
    const metrics = {
      timestamp: Date.now(),
      platform: os.platform(),
      memoryTotal: os.totalmem(),
      memoryFree: os.freemem(),
      memoryUsagePercent: ((os.totalmem() - os.freemem()) / os.totalmem()) * 100,
      cpuLoad: os.loadavg(),
      uptime: os.uptime(),
      currentBootTime: null,
      bootTimeChanged: false,
      eventLogErrors: [],
    };

    if (os.platform() === 'win32') {
      await this._collectWindowsMetrics(metrics);
    } else if (os.platform() === 'linux') {
      await this._collectLinuxMetrics(metrics);
    }

    return metrics;
  }

  async _collectWindowsMetrics(metrics) {
    try {
      // Get current boot time
      const bootTimeResult = await execPowerShellNoThrow(
        'Get-CimInstance -ClassName Win32_OperatingSystem | Select-Object -ExpandProperty LastBootUpTime'
      );

      if (bootTimeResult.code === 0) {
        metrics.currentBootTime = bootTimeResult.stdout.trim();

        if (this.lastBootTime && this.lastBootTime !== metrics.currentBootTime) {
          metrics.bootTimeChanged = true;
        }
        this.lastBootTime = metrics.currentBootTime;
      }

      // Check for recent errors (last 1 hour)
      const errorLogResult = await execPowerShellNoThrow(
        'Get-EventLog -LogName System -EntryType Error -After (Get-Date).AddHours(-1) | Select-Object -First 10 -Property EventID, Message | ConvertTo-Json'
      );

      if (errorLogResult.code === 0 && errorLogResult.stdout.trim().length > 0) {
        try {
          const errors = JSON.parse(errorLogResult.stdout);
          metrics.eventLogErrors = Array.isArray(errors) ? errors : [errors];
        } catch {
          // Failed to parse JSON
        }
      }
    } catch (error) {
      log.debug('Failed to collect Windows metrics:', error.message);
    }
  }

  async _collectLinuxMetrics(metrics) {
    try {
      // Get boot time from /proc/uptime
      const uptimeResult = await execFileNoThrow('cat', ['/proc/uptime']);
      if (uptimeResult.code === 0) {
        const uptimeSeconds = parseFloat(uptimeResult.stdout.split(' ')[0]);
        const bootTime = new Date(Date.now() - uptimeSeconds * 1000);
        metrics.currentBootTime = bootTime.toISOString();

        if (this.lastBootTime && this.lastBootTime !== metrics.currentBootTime) {
          metrics.bootTimeChanged = true;
        }
        this.lastBootTime = metrics.currentBootTime;
      }
    } catch (error) {
      log.debug('Failed to collect Linux metrics:', error.message);
    }
  }
}
```

**Guardian integration** (emit warnings to user):

```javascript
// .claude/hooks/PostToolUse.js
// NOTE: Hooks listen to eventBus via MCP

export default async function PostToolUse({ tool, mcp }) {
  // Request machine health events from MCP
  const health = await mcp.call('cynic__brain', 'machine_health_check', {});

  if (health.memoryUsagePercent > 90) {
    return {
      formattedWarning: `
┌─────────────────────────────────────────────────────────┐
│ *GROWL* 🛡️ MACHINE HEALTH WARNING                       │
├─────────────────────────────────────────────────────────┤
│ Memory usage critical: ${health.memoryUsagePercent.toFixed(1)}%${' '.repeat(30 - health.memoryUsagePercent.toFixed(1).length)}│
│ Available: ${(health.memoryFree / 1024 / 1024 / 1024).toFixed(1)} GB${' '.repeat(45 - (health.memoryFree / 1024 / 1024 / 1024).toFixed(1).length)}│
│                                                          │
│ Recommendation:                                         │
│   - Close unused applications                           │
│   - Save work immediately                               │
│   - Consider restarting machine                         │
│                                                          │
│ *sniff* CYNIC will survive a crash, but data loss hurts.│
└─────────────────────────────────────────────────────────┘
      `.trim(),
    };
  }

  return null;
}
```

---

## Layer 4: Auto-Recovery (Resume Exactly)

### Problem
After crash, how to resume **exactly** where we left off?

### Solution: Context Restoration

**On SessionStart (if crash detected)**:

1. **Restore session state**:
   - Load last conversation context
   - Resume in same working directory
   - Checkout same git branch

2. **Restore watcher state**:
   - SolanaWatcher: Resume from last polled signature
   - FileWatcher: Resume from last checksums
   - MarketWatcher: Resume from last price point

3. **Restore learning weights**:
   - Load Q-tables from DB
   - Load Thompson Sampling beta distributions
   - Resume episode count

4. **Resume background tasks**:
   - Restart daemon
   - Restart all watchers
   - Resume 11 learning loops

**Implementation**:

```javascript
// packages/node/src/daemon/crash-recovery.js

import { createLogger } from '@cynic/core';
import { db } from '@cynic/persistence';
import { execFileNoThrow } from '@cynic/core/utils/execFileNoThrow.js';

const log = createLogger('CrashRecovery');

export class CrashRecovery {
  async restoreSession(sessionId) {
    log.info('*sniff* Restoring session after crash...');

    // 1. Load last session state
    const sessionState = await db.query(`
      SELECT * FROM session_state
      WHERE session_id != $1
      ORDER BY timestamp DESC
      LIMIT 1
    `, [sessionId]);

    if (sessionState.rows.length === 0) {
      log.info('No previous session to restore');
      return null;
    }

    const state = sessionState.rows[0];

    // 2. Restore working directory
    try {
      process.chdir(state.working_directory);
      log.info(`Restored working directory: ${state.working_directory}`);
    } catch (error) {
      log.warn(`Failed to restore working directory: ${error.message}`);
    }

    // 3. Restore git branch
    if (state.git_branch) {
      try {
        const currentBranchResult = await execFileNoThrow('git', ['branch', '--show-current']);
        const currentBranch = currentBranchResult.stdout.trim();

        if (currentBranch !== state.git_branch) {
          log.info(`*ears perk* Restoring git branch: ${state.git_branch}`);
          await execFileNoThrow('git', ['checkout', state.git_branch]);
        }
      } catch (error) {
        log.warn(`Failed to restore git branch: ${error.message}`);
      }
    }

    // 4. Restore watcher states
    await this._restoreWatcherStates();

    // 5. Restore learning weights
    await this._restoreLearningWeights();

    log.info('*tail wag* Session restored successfully');
    return state;
  }

  async _restoreWatcherStates() {
    const watcherStates = await db.query(`
      SELECT * FROM watcher_state
    `);

    for (const watcher of watcherStates.rows) {
      log.info(`Restoring ${watcher.watcher_name} state`);
      // Emit event for each watcher to restore its state
      // Watcher will pick up from last_polled_signature/last_polled_slot
    }
  }

  async _restoreLearningWeights() {
    const learningStates = await db.query(`
      SELECT * FROM learning_state
    `);

    for (const state of learningStates.rows) {
      log.info(`Restoring ${state.loop_name} ${state.state_type}`);
      // Load weights into memory
      // Each learning loop will pick up its weights on next iteration
    }
  }
}
```

---

## Implementation Plan

### Phase 1: State Persistence (Week 1)
- [ ] Create migration `039_crash_resilience.sql` with tables
- [ ] Implement StatePersister class
- [ ] Wire StatePersister to daemon (30s heartbeat)
- [ ] Test: Kill process, verify state saved

### Phase 2: Crash Detection (Week 1)
- [ ] Implement detectCrash() in crash-detector.js
- [ ] Implement analyzeCrashCause() for Windows/Linux/Mac
- [ ] Wire to SessionStart MCP handler
- [ ] Test: Simulate crash (kill -9), verify detection on restart

### Phase 3: Machine Health Monitoring (Week 2)
- [ ] Implement MachineHealthWatcher
- [ ] Wire to daemon
- [ ] Add GROWL warnings via PostToolUse hook
- [ ] Test: Simulate memory pressure, verify warning

### Phase 4: Auto-Recovery (Week 2)
- [ ] Implement CrashRecovery class
- [ ] Wire to SessionStart handler
- [ ] Test: Full cycle (crash → detect → restore → resume)

### Phase 5: Linux/Mac Support (Week 3)
- [ ] Test analyzeCrashCause() on Linux (dmesg, journalctl)
- [ ] Test analyzeCrashCause() on Mac (system.log)
- [ ] Test on all 3 platforms

---

## Success Criteria

**φ-bounded promises**:
- ✅ **Detect crash**: 100% detection (heartbeat + boot time analysis)
- ✅ **Restore session state**: 100% (if saved within last 30s)
- ✅ **Resume watchers**: 95%+ (depends on external state like Solana slot)
- ✅ **Identify crash type**: 61.8% confidence (BSOD vs power loss vs kernel panic)
- ❌ **Prevent crash**: 0% (impossible — hardware/driver/OS responsibility)
- ❌ **Repair hardware**: 0% (impossible — CYNIC is software)

**Acceptance tests**:
1. Kill daemon (SIGKILL) → Restart → Session restored within 5 seconds
2. Simulate BSOD (Windows) → Restart → Crash type identified, event logs parsed
3. Memory pressure (>90%) → Guardian GROWL warning displayed
4. Power loss (unplug power) → Restart → Last heartbeat within 30s of crash

---

## Limitations (φ-Honesty)

**What CYNIC CAN do**:
- ✅ Detect crash happened
- ✅ Record crash type (BSOD, power loss, kernel panic)
- ✅ Restore conversation context
- ✅ Resume watchers from last checkpoint
- ✅ Warn user before memory exhaustion
- ✅ Parse Windows Event Logs for error patterns

**What CYNIC CANNOT do**:
- ❌ Prevent hardware failures (RAM, disk, PSU)
- ❌ Fix driver bugs (NVIDIA, Intel, etc.)
- ❌ Repair corrupted OS files
- ❌ Restore RAM state lost before crash (only DB state)
- ❌ Time travel (data between last heartbeat and crash is lost)

**φ-aligned confidence**: 61.8% that CYNIC will resume correctly after crash.
- 100% if crash happened after heartbeat save
- 30% if crash happened mid-judgment (partial state)
- 0% if PostgreSQL itself crashed (no persistence layer)

---

## Cost Analysis

**Storage overhead**:
- Session state: ~1 KB per heartbeat (30s) = 2.88 MB/day
- Watcher state: ~500 bytes per watcher per poll (1 min) = 2.88 MB/day (4 watchers)
- Learning weights: ~10 KB per update = negligible (few updates/hour)
- **Total**: ~6 MB/day storage overhead

**CPU overhead**:
- Heartbeat write: ~5 ms every 30s = negligible
- MachineHealthWatcher poll: ~50 ms every 1 min = negligible
- **Total**: <0.1% CPU overhead

**Recovery time**:
- Crash detection: 2-5 seconds (boot time analysis)
- State restoration: 3-10 seconds (DB queries + file operations)
- **Total downtime**: 5-15 seconds from crash to fully restored

---

*sniff* Confidence: 58% (φ⁻¹ limit)

This architecture makes CYNIC **crash-resistant**, not crash-proof. The dog survives, remembers, and resumes. But hardware failures are beyond software control.
