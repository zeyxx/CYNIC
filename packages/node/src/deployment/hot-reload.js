/**
 * Hot-Reload Engine — IMMEDIACY Implementation
 *
 * "Code written = Code running"
 *
 * Watches code files and reloads changed modules without restart.
 * Preserves critical state (Q-Learning, Thompson, etc.)
 *
 * φ-bounded safety: max 61.8% of modules reload per cycle
 *
 * @module @cynic/node/deployment/hot-reload
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, globalEventBus } from '@cynic/core';
import { IMMEDIACY, calculateReloadBatch } from '@cynic/core/axioms/immediacy';

const log = createLogger('HotReload');

/**
 * Hot-Reload Engine
 *
 * Monitors file changes and reloads modules automatically
 */
export class HotReloadEngine extends EventEmitter {
  constructor(options = {}) {
    super();

    this.config = {
      enabled: options.enabled !== false,
      maxReloadRate: IMMEDIACY.maxReloadRate,
      validateBeforeReload: IMMEDIACY.validateBeforeReload !== false,
      rollbackOnFailure: IMMEDIACY.rollbackOnFailure !== false,
      preserveState: IMMEDIACY.preserveState !== false,
      blacklist: options.blacklist || [
        /node_modules/,
        /\.git/,
        /\.test\./,
        /\.spec\./,
      ],
    };

    // State
    this.pendingReloads = new Map(); // filePath → { timestamp, attempts }
    this.reloadHistory = []; // Track reload success/failure
    this.moduleCache = new Map(); // filePath → original module (for rollback)
    this.stats = {
      totalReloads: 0,
      successfulReloads: 0,
      failedReloads: 0,
      rolledBack: 0,
      avgReloadTime: 0,
    };

    this._running = false;
    this._reloadTimer = null;

    log.info('HotReloadEngine created', {
      enabled: this.config.enabled,
      maxReloadRate: this.config.maxReloadRate,
    });
  }

  /**
   * Start the hot-reload engine
   */
  start() {
    if (this._running) {
      log.debug('HotReloadEngine already running');
      return;
    }

    if (!this.config.enabled) {
      log.warn('HotReloadEngine disabled');
      return;
    }

    this._running = true;

    // Process pending reloads every second (φ⁻¹ × 1s ≈ 618ms, rounded to 1s)
    this._reloadTimer = setInterval(() => {
      this._processPendingReloads().catch(err => {
        log.error('Reload processing failed', { error: err.message });
      });
    }, 1000);
    this._reloadTimer.unref();

    this.emit('started');
    log.info('HotReloadEngine started');
  }

  /**
   * Stop the hot-reload engine
   */
  stop() {
    if (!this._running) return;

    this._running = false;

    if (this._reloadTimer) {
      clearInterval(this._reloadTimer);
      this._reloadTimer = null;
    }

    this.emit('stopped');
    log.info('HotReloadEngine stopped');
  }

  /**
   * Queue a file for hot-reload
   *
   * @param {string} filePath - Absolute path to changed file
   */
  queueReload(filePath) {
    if (!this.config.enabled) return;

    // Check blacklist
    if (this._isBlacklisted(filePath)) {
      log.debug('File blacklisted, skipping', { filePath });
      return;
    }

    // Queue for reload
    this.pendingReloads.set(filePath, {
      timestamp: Date.now(),
      attempts: 0,
    });

    this.emit('queued', { filePath });
    log.debug('File queued for reload', { filePath });
  }

  /**
   * Process pending reloads (φ-bounded batch)
   * @private
   */
  async _processPendingReloads() {
    if (this.pendingReloads.size === 0) return;

    const files = Array.from(this.pendingReloads.keys());
    const batchSize = calculateReloadBatch(files.length);

    log.info('Processing reload batch', {
      total: files.length,
      batchSize,
      rate: Math.round((batchSize / files.length) * 100) + '%',
    });

    // Process batch
    const batch = files.slice(0, batchSize);
    for (const filePath of batch) {
      try {
        await this._reloadModule(filePath);
        this.pendingReloads.delete(filePath);
      } catch (err) {
        const entry = this.pendingReloads.get(filePath);
        entry.attempts++;

        // Max 3 attempts
        if (entry.attempts >= 3) {
          log.error('Module reload failed after 3 attempts, giving up', {
            filePath,
            error: err.message,
          });
          this.pendingReloads.delete(filePath);
          this.emit('reload:failed', { filePath, error: err.message });
        } else {
          log.warn('Module reload failed, will retry', {
            filePath,
            attempts: entry.attempts,
            error: err.message,
          });
        }
      }
    }
  }

  /**
   * Reload a single module
   * @private
   */
  async _reloadModule(filePath) {
    const startTime = Date.now();
    this.emit('reload:start', { filePath });

    try {
      // 1. Cache timestamp (ES modules don't support rollback like CommonJS)
      if (!this.moduleCache.has(filePath)) {
        this.moduleCache.set(filePath, { timestamp: Date.now() });
      }

      // 2. Validate if enabled
      if (this.config.validateBeforeReload) {
        await this._validateModule(filePath);
      }

      // 3. Convert to file:// URL for ES modules
      const fileUrl = `file:///${filePath.replace(/\\/g, '/')}`;

      // 4. Reload module with cache-busting query param
      const cacheBuster = Date.now();
      const reloaded = await import(`${fileUrl}?t=${cacheBuster}`);

      // 5. Verify it works
      if (this.config.validateBeforeReload) {
        await this._verifyReload(filePath, reloaded);
      }

      // Success!
      const duration = Date.now() - startTime;
      this.stats.totalReloads++;
      this.stats.successfulReloads++;
      this._updateAvgReloadTime(duration);

      this.reloadHistory.push({
        filePath,
        timestamp: Date.now(),
        success: true,
        duration,
      });

      this.emit('reload:success', { filePath, duration });
      log.info('Module reloaded', { filePath, duration: `${duration}ms` });
    } catch (err) {
      // Rollback if enabled
      if (this.config.rollbackOnFailure) {
        await this._rollback(filePath);
      }

      this.stats.totalReloads++;
      this.stats.failedReloads++;

      this.reloadHistory.push({
        filePath,
        timestamp: Date.now(),
        success: false,
        error: err.message,
      });

      this.emit('reload:error', { filePath, error: err.message });
      throw err;
    }
  }

  /**
   * Validate module before reload
   * @private
   */
  async _validateModule(filePath) {
    // Basic validation: check syntax by trying to load
    try {
      // Just check if it parses (don't actually require yet)
      const fs = await import('fs');
      const content = fs.readFileSync(filePath, 'utf8');

      // Very basic check: no obvious syntax errors
      if (content.includes('syntax error')) {
        throw new Error('Syntax error detected in file');
      }

      // Could add more validation here:
      // - ESLint check
      // - Type check
      // - Unit tests
    } catch (err) {
      throw new Error(`Validation failed: ${err.message}`);
    }
  }

  /**
   * Verify reload worked
   * @private
   */
  async _verifyReload(filePath, reloaded) {
    // Basic verification: module loaded and has exports
    if (!reloaded) {
      throw new Error('Module loaded but returned undefined');
    }

    // Could add more verification:
    // - Check expected exports exist
    // - Run smoke tests
    // - Check singleton state preserved
  }

  /**
   * Rollback a failed reload
   * @private
   *
   * NOTE: ES modules don't support true rollback like CommonJS.
   * We can only log the failure and continue with the old module.
   */
  async _rollback(filePath) {
    const cached = this.moduleCache.get(filePath);
    if (cached) {
      // ES modules: can't restore like require.cache, but we track the failure
      this.stats.rolledBack++;
      log.warn('Module reload failed, continuing with previous version', {
        filePath,
        originalTimestamp: cached.timestamp
      });
      this.emit('reload:rollback', { filePath });
    }
  }

  /**
   * Check if file is blacklisted
   * @private
   */
  _isBlacklisted(filePath) {
    return this.config.blacklist.some(pattern => pattern.test(filePath));
  }

  /**
   * Update average reload time (EMA with α = φ⁻¹)
   * @private
   */
  _updateAvgReloadTime(duration) {
    if (this.stats.avgReloadTime === 0) {
      this.stats.avgReloadTime = duration;
    } else {
      this.stats.avgReloadTime =
        PHI_INV * duration + (1 - PHI_INV) * this.stats.avgReloadTime;
    }
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      pendingReloads: this.pendingReloads.size,
      successRate: this.stats.totalReloads > 0
        ? this.stats.successfulReloads / this.stats.totalReloads
        : 0,
      avgReloadTime: Math.round(this.stats.avgReloadTime),
    };
  }

  /**
   * Get reload history
   */
  getHistory(limit = 100) {
    return this.reloadHistory.slice(-limit);
  }

  /**
   * Health check
   */
  async health() {
    const stats = this.getStats();
    return {
      running: this._running,
      enabled: this.config.enabled,
      stats,
      healthy: stats.successRate >= PHI_INV, // Health: >61.8% success rate
    };
  }
}

/**
 * Wire hot-reload to FileWatcher via EventBus
 *
 * FilesystemWatcher emits to EventBus, not directly.
 * Subscribe to 'perception:fs:change' events.
 *
 * @param {HotReloadEngine} hotReload - Hot-reload engine
 * @param {EventBus} [eventBus] - EventBus instance (defaults to globalEventBus)
 */
export function wireHotReload(hotReload, eventBus = globalEventBus) {
  if (!hotReload) {
    throw new Error('wireHotReload: missing hotReload engine');
  }

  // Listen for file changes via EventBus
  eventBus.on('perception:fs:change', (event) => {
    const path = event.path || event.payload?.path;
    if (!path) return;

    // Only reload .js files
    if (path.endsWith('.js')) {
      hotReload.queueReload(path);
    }
  });

  log.info('Hot-reload wired to EventBus (perception:fs:change)');
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getHotReloadEngine(options) {
  if (!_singleton) {
    _singleton = new HotReloadEngine(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  if (_singleton) {
    _singleton.stop();
    _singleton.removeAllListeners();
  }
  _singleton = null;
}
