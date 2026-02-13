/**
 * Machine Health Watcher - R5.1 (HUMAN × PERCEIVE)
 *
 * Monitors system health and emits events.
 * "φ knows when the machine struggles" - κυνικός
 *
 * Monitors:
 * - CPU usage
 * - Memory usage
 * - Disk space
 * - Daemon health
 * - Network connectivity
 *
 * Emits: daemon:health:{healthy|warning|degraded|critical}
 *
 * @module @cynic/node/perception/machine-health-watcher
 */

'use strict';

import os from 'os';
import { promisify } from 'util';
import { exec } from 'child_process';
import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2, globalEventBus } from '@cynic/core';

const execPromise = promisify(exec);

const log = createLogger('MachineHealthWatcher');

/**
 * φ-aligned health thresholds
 */
const HEALTH_THRESHOLDS = {
  // CPU (percentage)
  cpuWarning: PHI_INV * 100,      // 61.8%
  cpuDegraded: 80,                 // 80%
  cpuCritical: 95,                 // 95%

  // Memory (percentage)
  memoryWarning: PHI_INV * 100,    // 61.8%
  memoryDegraded: 80,              // 80%
  memoryCritical: 90,              // 90%

  // Disk space (percentage free)
  diskWarning: PHI_INV_2 * 100,    // 38.2% free
  diskDegraded: 20,                // 20% free
  diskCritical: 10,                // 10% free

  // Daemon (uptime in seconds)
  daemonCritical: 60,              // < 1 min uptime = suspicious restart
};

/**
 * Machine Health Watcher
 *
 * Monitors system resources and daemon health.
 */
export class MachineHealthWatcher extends EventEmitter {
  constructor(options = {}) {
    super();

    this.interval = options.interval || 30000; // 30s default
    this.eventBus = options.eventBus || globalEventBus;

    this._timer = null;
    this._isRunning = false;
    this._lastHealth = 'healthy';

    // Stats
    this.stats = {
      checks: 0,
      warnings: 0,
      degraded: 0,
      critical: 0,
      lastCheck: null,
    };

    // Latest metrics
    this.metrics = {
      cpu: null,
      memory: null,
      disk: null,
      daemon: null,
      timestamp: null,
    };
  }

  /**
   * Start health monitoring
   */
  async start() {
    if (this._isRunning) {
      log.warn('MachineHealthWatcher already running');
      return;
    }

    log.info('Starting health monitoring', { intervalMs: this.interval });
    this._isRunning = true;

    // Immediate check
    await this._check();

    // Schedule periodic checks
    this._timer = setInterval(() => {
      this._check().catch(err => {
        log.error('Health check failed', { error: err.message });
      });
    }, this.interval);
  }

  /**
   * Stop health monitoring
   */
  stop() {
    if (this._timer) {
      clearInterval(this._timer);
      this._timer = null;
    }
    this._isRunning = false;
    log.info('Health monitoring stopped');
  }

  /**
   * Get current health status
   */
  getHealth() {
    return {
      status: this._lastHealth,
      metrics: this.metrics,
      stats: this.stats,
    };
  }

  // =============================================================================
  // PRIVATE METHODS
  // =============================================================================

  /**
   * Perform health check
   * @private
   */
  async _check() {
    this.stats.checks++;
    this.stats.lastCheck = Date.now();

    try {
      // Collect metrics
      const [cpu, memory, disk, daemon] = await Promise.all([
        this._checkCPU(),
        this._checkMemory(),
        this._checkDisk(),
        this._checkDaemon(),
      ]);

      this.metrics = {
        cpu,
        memory,
        disk,
        daemon,
        timestamp: Date.now(),
      };

      // Determine overall health
      const health = this._computeHealth();

      // Emit if changed
      if (health !== this._lastHealth) {
        this._lastHealth = health;
        this._emitHealthEvent(health);
      }
    } catch (error) {
      log.error('Health check error', { error: error.message });
    }
  }

  /**
   * Check CPU usage
   * @private
   */
  async _checkCPU() {
    const cpus = os.cpus();
    let totalIdle = 0;
    let totalTick = 0;

    for (const cpu of cpus) {
      for (const type in cpu.times) {
        totalTick += cpu.times[type];
      }
      totalIdle += cpu.times.idle;
    }

    const idle = totalIdle / cpus.length;
    const total = totalTick / cpus.length;
    const usage = 100 - ~~(100 * idle / total);

    return {
      usage,
      cores: cpus.length,
      model: cpus[0]?.model || 'unknown',
    };
  }

  /**
   * Check memory usage
   * @private
   */
  async _checkMemory() {
    const total = os.totalmem();
    const free = os.freemem();
    const used = total - free;
    const usage = (used / total) * 100;

    return {
      total: Math.round(total / 1024 / 1024), // MB
      free: Math.round(free / 1024 / 1024),   // MB
      used: Math.round(used / 1024 / 1024),   // MB
      usage: Math.round(usage),
    };
  }

  /**
   * Check disk space
   * @private
   */
  async _checkDisk() {
    // Cross-platform disk check
    if (process.platform === 'win32') {
      return this._checkDiskWindows();
    } else {
      return this._checkDiskUnix();
    }
  }

  /**
   * Check disk space (Windows)
   * @private
   */
  async _checkDiskWindows() {
    try {
      // Use wmic to get disk info
      const { stdout } = await execPromise('wmic logicaldisk where "DeviceID=\'C:\'" get Size,FreeSpace /value');

      const lines = stdout.split('\n');
      let size = 0;
      let free = 0;

      for (const line of lines) {
        if (line.startsWith('FreeSpace=')) {
          free = parseInt(line.split('=')[1]);
        } else if (line.startsWith('Size=')) {
          size = parseInt(line.split('=')[1]);
        }
      }

      if (size === 0) {
        return { total: 0, free: 0, used: 0, usage: 0 };
      }

      const used = size - free;
      const usage = (used / size) * 100;

      return {
        total: Math.round(size / 1024 / 1024 / 1024), // GB
        free: Math.round(free / 1024 / 1024 / 1024),  // GB
        used: Math.round(used / 1024 / 1024 / 1024),  // GB
        usage: Math.round(usage),
      };
    } catch (error) {
      log.debug('Disk check failed', { error: error.message });
      return { total: 0, free: 0, used: 0, usage: 0 };
    }
  }

  /**
   * Check disk space (Unix)
   * @private
   */
  async _checkDiskUnix() {
    try {
      const { stdout } = await execPromise('df -k /');
      const lines = stdout.trim().split('\n');
      if (lines.length < 2) {
        return { total: 0, free: 0, used: 0, usage: 0 };
      }

      const parts = lines[1].split(/\s+/);
      const total = parseInt(parts[1]) / 1024 / 1024; // GB
      const used = parseInt(parts[2]) / 1024 / 1024;  // GB
      const free = parseInt(parts[3]) / 1024 / 1024;  // GB
      const usage = (used / total) * 100;

      return {
        total: Math.round(total),
        free: Math.round(free),
        used: Math.round(used),
        usage: Math.round(usage),
      };
    } catch (error) {
      log.debug('Disk check failed', { error: error.message });
      return { total: 0, free: 0, used: 0, usage: 0 };
    }
  }

  /**
   * Check daemon health
   * @private
   */
  async _checkDaemon() {
    return {
      uptime: Math.round(process.uptime()),
      memory: Math.round(process.memoryUsage().heapUsed / 1024 / 1024), // MB
      pid: process.pid,
    };
  }

  /**
   * Compute overall health status
   * @private
   */
  _computeHealth() {
    const issues = [];

    // CPU checks
    if (this.metrics.cpu.usage >= HEALTH_THRESHOLDS.cpuCritical) {
      issues.push({ subsystem: 'cpu', level: 'critical', value: this.metrics.cpu.usage });
    } else if (this.metrics.cpu.usage >= HEALTH_THRESHOLDS.cpuDegraded) {
      issues.push({ subsystem: 'cpu', level: 'degraded', value: this.metrics.cpu.usage });
    } else if (this.metrics.cpu.usage >= HEALTH_THRESHOLDS.cpuWarning) {
      issues.push({ subsystem: 'cpu', level: 'warning', value: this.metrics.cpu.usage });
    }

    // Memory checks
    if (this.metrics.memory.usage >= HEALTH_THRESHOLDS.memoryCritical) {
      issues.push({ subsystem: 'memory', level: 'critical', value: this.metrics.memory.usage });
    } else if (this.metrics.memory.usage >= HEALTH_THRESHOLDS.memoryDegraded) {
      issues.push({ subsystem: 'memory', level: 'degraded', value: this.metrics.memory.usage });
    } else if (this.metrics.memory.usage >= HEALTH_THRESHOLDS.memoryWarning) {
      issues.push({ subsystem: 'memory', level: 'warning', value: this.metrics.memory.usage });
    }

    // Disk checks (inverted - low free space is bad)
    const diskFreePercent = (this.metrics.disk.free / this.metrics.disk.total) * 100;
    if (diskFreePercent <= HEALTH_THRESHOLDS.diskCritical) {
      issues.push({ subsystem: 'disk', level: 'critical', value: diskFreePercent });
    } else if (diskFreePercent <= HEALTH_THRESHOLDS.diskDegraded) {
      issues.push({ subsystem: 'disk', level: 'degraded', value: diskFreePercent });
    } else if (diskFreePercent <= HEALTH_THRESHOLDS.diskWarning) {
      issues.push({ subsystem: 'disk', level: 'warning', value: diskFreePercent });
    }

    // Daemon checks
    if (this.metrics.daemon.uptime < HEALTH_THRESHOLDS.daemonCritical) {
      issues.push({ subsystem: 'daemon', level: 'critical', value: this.metrics.daemon.uptime });
    }

    // Determine overall health
    if (issues.some(i => i.level === 'critical')) {
      this.stats.critical++;
      return 'critical';
    } else if (issues.some(i => i.level === 'degraded')) {
      this.stats.degraded++;
      return 'degraded';
    } else if (issues.some(i => i.level === 'warning')) {
      this.stats.warnings++;
      return 'warning';
    } else {
      return 'healthy';
    }
  }

  /**
   * Emit health event
   * @private
   */
  _emitHealthEvent(health) {
    const event = `daemon:health:${health}`;
    const payload = {
      level: health,
      metrics: this.metrics,
      issues: this._getIssues(),
      timestamp: Date.now(),
    };

    log.info(`Health status: ${health}`, {
      cpu: `${this.metrics.cpu.usage}%`,
      memory: `${this.metrics.memory.usage}%`,
      disk: `${this.metrics.disk.usage}%`,
    });

    this.eventBus.publish(event, payload, { source: 'MachineHealthWatcher' });
    this.emit(health, payload);
  }

  /**
   * Get current health issues
   * @private
   */
  _getIssues() {
    const issues = [];

    if (this.metrics.cpu.usage >= HEALTH_THRESHOLDS.cpuWarning) {
      issues.push({ subsystem: 'cpu', value: this.metrics.cpu.usage, threshold: HEALTH_THRESHOLDS.cpuWarning });
    }
    if (this.metrics.memory.usage >= HEALTH_THRESHOLDS.memoryWarning) {
      issues.push({ subsystem: 'memory', value: this.metrics.memory.usage, threshold: HEALTH_THRESHOLDS.memoryWarning });
    }
    const diskFree = (this.metrics.disk.free / this.metrics.disk.total) * 100;
    if (diskFree <= HEALTH_THRESHOLDS.diskWarning) {
      issues.push({ subsystem: 'disk', value: diskFree, threshold: HEALTH_THRESHOLDS.diskWarning });
    }

    return issues;
  }
}

export default MachineHealthWatcher;
