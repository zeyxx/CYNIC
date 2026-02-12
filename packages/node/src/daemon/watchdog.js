/**
 * CYNIC Daemon — Watchdog
 *
 * Internal self-monitoring service. Runs as a 30s interval inside the daemon.
 * Detects degradation, escalates failures, and triggers recovery.
 *
 * Health checks:
 *   1. Heap usage (warn φ⁻¹ = 61.8%, critical 80%)
 *   2. Event loop latency (warn >100ms)
 *   3. Subsystem health (boot providers, wiring)
 *
 * Escalation (circuit breaker):
 *   WARNING → log + emit daemon:health:degraded
 *   CRITICAL → clear caches, disable non-critical services
 *   FATAL (3 consecutive critical) → sentinel file + exit(1) → auto-spawn resurrects
 *
 * "Le chien veille sur lui-même" — CYNIC
 *
 * @module @cynic/node/daemon/watchdog
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';
import v8 from 'v8';
import { createLogger, PHI_INV, globalEventBus } from '@cynic/core';

const log = createLogger('Watchdog');

/** Watchdog check interval: 30s */
const CHECK_INTERVAL_MS = 30_000;

/** Heap thresholds */
const HEAP_WARN_RATIO = PHI_INV;  // 61.8%
const HEAP_CRITICAL_RATIO = 0.80;

/** Event loop latency thresholds (ms) */
const LOOP_WARN_MS = 100;
const LOOP_CRITICAL_MS = 500;

/** Consecutive critical failures before FATAL */
const FATAL_THRESHOLD = 3;

/** Sentinel file for restart requests */
const DAEMON_DIR = path.join(os.homedir(), '.cynic', 'daemon');
const RESTART_SENTINEL = path.join(DAEMON_DIR, 'restart-requested');

/**
 * Health status levels
 */
export const HealthLevel = {
  HEALTHY: 'healthy',
  WARNING: 'warning',
  CRITICAL: 'critical',
  FATAL: 'fatal',
};

/**
 * CYNIC Daemon Watchdog
 *
 * Self-monitoring circuit breaker with escalation.
 */
export class Watchdog {
  /**
   * @param {Object} [options]
   * @param {number} [options.interval] - Check interval in ms
   * @param {number} [options.fatalThreshold] - Consecutive critical before FATAL
   * @param {Function} [options.onFatal] - Custom fatal handler (for testing)
   */
  constructor(options = {}) {
    this._interval = options.interval ?? CHECK_INTERVAL_MS;
    this._fatalThreshold = options.fatalThreshold ?? FATAL_THRESHOLD;
    this._onFatal = options.onFatal ?? null;
    this._timer = null;
    this._running = false;

    // Subsystem health tracking
    this._subsystems = new Map();

    // Circuit breaker state
    this._consecutiveCritical = 0;
    this._lastCheck = null;
    this._checkCount = 0;

    // Latest metrics
    this._metrics = {
      heapRatio: 0,
      heapUsedMB: 0,
      heapTotalMB: 0,
      eventLoopLatencyMs: 0,
      level: HealthLevel.HEALTHY,
      degradedSubsystems: [],
      lastCheckTime: null,
    };
  }

  /**
   * Start the watchdog timer.
   */
  start() {
    if (this._running) return;

    this._running = true;
    this._timer = setInterval(() => this._check(), this._interval);
    this._timer.unref();

    // Initial check
    this._check();

    log.info('Watchdog started', { interval: this._interval });
  }

  /**
   * Stop the watchdog timer.
   */
  stop() {
    if (this._timer) {
      clearInterval(this._timer);
      this._timer = null;
    }
    this._running = false;
    log.info('Watchdog stopped');
  }

  /**
   * Get current health status.
   *
   * @returns {Object} Health status snapshot
   */
  getStatus() {
    return {
      ...this._metrics,
      running: this._running,
      consecutiveCritical: this._consecutiveCritical,
      checkCount: this._checkCount,
      subsystems: Object.fromEntries(this._subsystems),
    };
  }

  /**
   * Register a subsystem for health tracking.
   *
   * @param {string} name - Subsystem name
   * @param {Function} healthCheck - Returns { healthy: boolean, message?: string }
   */
  registerSubsystem(name, healthCheck) {
    this._subsystems.set(name, {
      check: healthCheck,
      healthy: true,
      lastMessage: null,
      failCount: 0,
    });
  }

  /**
   * Run a single health check cycle.
   * @private
   */
  async _check() {
    this._checkCount++;
    const startTime = Date.now();
    const issues = [];

    // 1. Heap usage (compare against heap size limit, NOT heap total)
    const mem = process.memoryUsage();
    const heapStats = v8.getHeapStatistics();
    const heapSizeLimit = heapStats.heap_size_limit;
    const heapRatio = mem.heapUsed / heapSizeLimit;
    this._metrics.heapRatio = heapRatio;
    this._metrics.heapUsedMB = Math.round(mem.heapUsed / 1024 / 1024);
    this._metrics.heapTotalMB = Math.round(mem.heapTotal / 1024 / 1024);
    this._metrics.heapLimitMB = Math.round(heapSizeLimit / 1024 / 1024);

    if (heapRatio >= HEAP_CRITICAL_RATIO) {
      issues.push({ subsystem: 'heap', level: HealthLevel.CRITICAL, message: `Heap ${(heapRatio * 100).toFixed(1)}% (critical)` });
    } else if (heapRatio >= HEAP_WARN_RATIO) {
      issues.push({ subsystem: 'heap', level: HealthLevel.WARNING, message: `Heap ${(heapRatio * 100).toFixed(1)}% (warn)` });
    }

    // 2. Event loop latency
    const loopLatency = await this._measureEventLoopLatency();
    this._metrics.eventLoopLatencyMs = loopLatency;

    if (loopLatency >= LOOP_CRITICAL_MS) {
      issues.push({ subsystem: 'eventLoop', level: HealthLevel.CRITICAL, message: `Event loop ${loopLatency}ms (critical)` });
    } else if (loopLatency >= LOOP_WARN_MS) {
      issues.push({ subsystem: 'eventLoop', level: HealthLevel.WARNING, message: `Event loop ${loopLatency}ms (warn)` });
    }

    // 3. Registered subsystems
    for (const [name, sub] of this._subsystems) {
      try {
        const result = typeof sub.check === 'function' ? await sub.check() : { healthy: true };
        sub.healthy = result.healthy !== false;
        sub.lastMessage = result.message || null;
        if (!sub.healthy) {
          sub.failCount++;
          issues.push({ subsystem: name, level: HealthLevel.WARNING, message: result.message || `${name} unhealthy` });
        } else {
          sub.failCount = 0;
        }
      } catch (err) {
        sub.healthy = false;
        sub.failCount++;
        sub.lastMessage = err.message;
        issues.push({ subsystem: name, level: HealthLevel.WARNING, message: err.message });
      }
    }

    // Determine overall level
    const hasCritical = issues.some(i => i.level === HealthLevel.CRITICAL);
    const hasWarning = issues.some(i => i.level === HealthLevel.WARNING);

    const degraded = issues.map(i => i.subsystem);
    this._metrics.degradedSubsystems = degraded;
    this._metrics.lastCheckTime = startTime;

    if (hasCritical) {
      this._metrics.level = HealthLevel.CRITICAL;
      this._consecutiveCritical++;

      log.warn('Health CRITICAL', { issues, consecutive: this._consecutiveCritical });

      // Emit degradation event
      try {
        globalEventBus.emit('daemon:health:degraded', {
          level: HealthLevel.CRITICAL,
          issues,
          consecutiveCritical: this._consecutiveCritical,
          timestamp: startTime,
        });
      } catch { /* non-blocking */ }

      // Attempt recovery
      this._attemptRecovery(issues);

      // Check for FATAL threshold
      if (this._consecutiveCritical >= this._fatalThreshold) {
        this._handleFatal(issues);
        return;
      }

    } else if (hasWarning) {
      this._metrics.level = HealthLevel.WARNING;
      this._consecutiveCritical = 0;

      log.debug('Health WARNING', { issues });

      try {
        globalEventBus.emit('daemon:health:degraded', {
          level: HealthLevel.WARNING,
          issues,
          timestamp: startTime,
        });
      } catch { /* non-blocking */ }

    } else {
      this._metrics.level = HealthLevel.HEALTHY;
      this._consecutiveCritical = 0;
    }

    this._lastCheck = startTime;
  }

  /**
   * Measure event loop latency via setTimeout(0).
   * @returns {Promise<number>} Actual delay in ms
   * @private
   */
  _measureEventLoopLatency() {
    return new Promise((resolve) => {
      const start = Date.now();
      setTimeout(() => resolve(Date.now() - start), 0);
    });
  }

  /**
   * Attempt recovery from critical state.
   *
   * Actions:
   *   1. Trigger GC if available
   *   2. Clear non-critical caches
   *   3. Emit memory pressure event
   *
   * @param {Object[]} issues - Current health issues
   * @private
   */
  _attemptRecovery(issues) {
    const heapCritical = issues.some(i => i.subsystem === 'heap' && i.level === HealthLevel.CRITICAL);

    if (heapCritical) {
      // 1. Try GC if --expose-gc
      if (typeof global.gc === 'function') {
        global.gc();
        log.info('Forced GC triggered');
      }

      // 2. Emit memory pressure for consumers to clear caches
      try {
        globalEventBus.emit('daemon:memory:pressure', {
          heapRatio: this._metrics.heapRatio,
          heapUsedMB: this._metrics.heapUsedMB,
          timestamp: Date.now(),
        });
      } catch { /* non-blocking */ }

      log.info('Recovery attempted (memory pressure)');
    }
  }

  /**
   * Handle FATAL condition: write sentinel and exit.
   *
   * The thin hook daemon-client will auto-spawn a new daemon on next request.
   *
   * @param {Object[]} issues - Current health issues
   * @private
   */
  _handleFatal(issues) {
    log.error('Health FATAL — requesting restart', {
      consecutiveCritical: this._consecutiveCritical,
      issues,
    });

    // Write restart sentinel
    try {
      if (!fs.existsSync(DAEMON_DIR)) {
        fs.mkdirSync(DAEMON_DIR, { recursive: true });
      }
      fs.writeFileSync(RESTART_SENTINEL, JSON.stringify({
        reason: 'watchdog_fatal',
        issues,
        consecutiveCritical: this._consecutiveCritical,
        timestamp: Date.now(),
        pid: process.pid,
      }));
    } catch (err) {
      log.error('Failed to write restart sentinel', { error: err.message });
    }

    // Custom handler (for testing) or exit
    if (this._onFatal) {
      this._onFatal(issues);
    } else {
      process.exit(1);
    }
  }
}

/**
 * Check for and clean up stale restart sentinel.
 * Called at daemon startup to detect crash recovery.
 *
 * @returns {{ recovered: boolean, previousCrash?: Object }}
 */
export function checkRestartSentinel() {
  try {
    if (!fs.existsSync(RESTART_SENTINEL)) {
      return { recovered: false };
    }

    const raw = fs.readFileSync(RESTART_SENTINEL, 'utf8');
    const data = JSON.parse(raw);

    // Clean up sentinel
    fs.unlinkSync(RESTART_SENTINEL);

    log.info('Recovered from crash', {
      reason: data.reason,
      previousPid: data.pid,
      crashTime: new Date(data.timestamp).toISOString(),
    });

    return { recovered: true, previousCrash: data };
  } catch {
    // Clean up corrupt sentinel
    try { fs.unlinkSync(RESTART_SENTINEL); } catch { /* ignore */ }
    return { recovered: false };
  }
}

export default Watchdog;
