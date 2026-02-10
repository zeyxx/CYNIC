/**
 * CYNIC Process Registry — Cross-Process Discovery
 *
 * Gap #6 Fix: MCP/Hooks/Daemon have no handshake protocol.
 * This module gives CYNIC cross-process awareness.
 *
 * PATTERN: Same as consciousness-readback.js — file-based persistence
 * at ~/.cynic/processes/registry.json. Proven cross-process communication.
 *
 * THREE OPERATIONS:
 *   1. announce() — "I'm here" (MCP/Daemon write on boot)
 *   2. discover() — "Who's here?" (Hooks read before calling MCP)
 *   3. depart()   — "I'm leaving" (cleanup on shutdown)
 *
 * STALENESS: Entries older than 2× heartbeat interval are pruned.
 * File locking: Best-effort (read-modify-write with JSON merge).
 *
 * "γνῶθι σεαυτόν across processes" — κυνικός
 *
 * @module @cynic/core/topology/process-registry
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';
import { PHI_INV } from '../axioms/constants.js';
import { createLogger } from '../logger.js';

const log = createLogger('ProcessRegistry');

// ──────────────────────────────────────────────────────────────
// CONSTANTS
// ──────────────────────────────────────────────────────────────

const REGISTRY_DIR = path.join(os.homedir(), '.cynic', 'processes');
const REGISTRY_FILE = path.join(REGISTRY_DIR, 'registry.json');

/** Default heartbeat interval: 30s (matches HeartbeatService) */
const DEFAULT_HEARTBEAT_MS = 30_000;

/** Staleness threshold: 2× heartbeat = 60s. No heartbeat in 60s → considered dead */
const STALENESS_FACTOR = 2;

/** Max age before hard-pruning: 10 minutes. Even if stale, keep for diagnostics */
const MAX_AGE_MS = 10 * 60 * 1000;

// ──────────────────────────────────────────────────────────────
// PROCESS REGISTRY
// ──────────────────────────────────────────────────────────────

class ProcessRegistry {
  constructor() {
    this._heartbeatTimer = null;
    this._heartbeatInterval = DEFAULT_HEARTBEAT_MS;
    this._processId = null;         // Our key in the registry
    this._processInfo = null;       // Our announced info
    this._shutdownHandlers = [];    // Cleanup callbacks
    this._registryFile = REGISTRY_FILE;  // Overridable for testing
    this._registryDir = REGISTRY_DIR;
  }

  // ──────────────────────────────────────────────────────────
  // ANNOUNCE — "I'm here"
  // Called by MCP/Daemon on boot. Writes to registry file.
  // ──────────────────────────────────────────────────────────

  /**
   * Announce this process to the registry.
   *
   * @param {Object} info - Process information
   * @param {string} info.mode - ExecutionMode (mcp, daemon, hook, test)
   * @param {string} [info.endpoint] - HTTP endpoint if reachable (e.g., MCP URL)
   * @param {string[]} [info.capabilities] - What this process can do
   * @param {Object} [info.meta] - Additional metadata
   * @param {number} [info.heartbeatInterval] - Custom heartbeat interval in ms
   * @returns {string} processId assigned to this process
   */
  announce(info) {
    const processId = `${info.mode}-${process.pid}`;
    this._processId = processId;

    if (info.heartbeatInterval) {
      this._heartbeatInterval = info.heartbeatInterval;
    }

    this._processInfo = {
      mode: info.mode,
      pid: process.pid,
      ppid: process.ppid,
      endpoint: info.endpoint || null,
      capabilities: info.capabilities || [],
      meta: info.meta || {},
      startedAt: Date.now(),
      lastHeartbeat: Date.now(),
    };

    // Write to registry
    this._writeEntry(processId, this._processInfo);

    // Start heartbeat (update lastHeartbeat periodically)
    this._startHeartbeat();

    // Register shutdown cleanup
    this._registerShutdownCleanup();

    log.info(`Announced: ${processId}`, {
      mode: info.mode,
      endpoint: info.endpoint || 'none',
      capabilities: info.capabilities?.length || 0,
    });

    return processId;
  }

  // ──────────────────────────────────────────────────────────
  // DISCOVER — "Who's here?"
  // Called by hooks/any process to see who's alive.
  // ──────────────────────────────────────────────────────────

  /**
   * Discover all live processes.
   * Reads the registry, prunes stale entries, returns active peers.
   *
   * @param {Object} [options]
   * @param {boolean} [options.includeStale=false] - Include stale entries (for diagnostics)
   * @param {string} [options.mode] - Filter by mode (mcp, daemon, hook)
   * @returns {{ processes: Object[], stale: Object[], self: string|null }}
   */
  discover(options = {}) {
    const registry = this._readRegistry();
    const now = Date.now();
    const stalenessThreshold = this._heartbeatInterval * STALENESS_FACTOR;

    const processes = [];
    const stale = [];
    let hardPruned = 0;

    for (const [id, entry] of Object.entries(registry)) {
      const age = now - (entry.lastHeartbeat || entry.startedAt || 0);

      // Hard prune: remove entries older than MAX_AGE_MS
      if (age > MAX_AGE_MS) {
        hardPruned++;
        continue; // Skip completely
      }

      // Filter by mode if requested (does NOT affect pruning)
      if (options.mode && entry.mode !== options.mode) {
        continue;
      }

      if (age > stalenessThreshold) {
        stale.push({ id, ...entry, staleFor: age - stalenessThreshold });
      } else {
        processes.push({ id, ...entry });
      }
    }

    // Only prune if we hard-pruned entries (don't prune mode-filtered entries)
    if (hardPruned > 0) {
      const surviving = {};
      for (const [id, entry] of Object.entries(registry)) {
        const age = now - (entry.lastHeartbeat || entry.startedAt || 0);
        if (age <= MAX_AGE_MS) surviving[id] = entry;
      }
      this._writeRegistry(surviving);
    }

    return {
      processes: options.includeStale ? [...processes, ...stale] : processes,
      stale,
      self: this._processId,
    };
  }

  /**
   * Check if a specific mode has an active process.
   * Fast path for hooks checking "is MCP running?"
   *
   * @param {string} mode - ExecutionMode to check (e.g., 'mcp')
   * @returns {{ available: boolean, endpoint: string|null, pid: number|null }}
   */
  isAvailable(mode) {
    const { processes } = this.discover({ mode });

    if (processes.length === 0) {
      return { available: false, endpoint: null, pid: null };
    }

    // Return the most recently heartbeated process of this mode
    const sorted = processes.sort((a, b) =>
      (b.lastHeartbeat || 0) - (a.lastHeartbeat || 0)
    );

    return {
      available: true,
      endpoint: sorted[0].endpoint || null,
      pid: sorted[0].pid || null,
    };
  }

  /**
   * Get summary of all known peers (for topology snapshot).
   *
   * @returns {{ total: number, byMode: Object, healthy: number, stale: number }}
   */
  getPeerSummary() {
    const { processes, stale } = this.discover({ includeStale: true });
    const byMode = {};

    for (const p of processes) {
      const mode = p.mode || 'unknown';
      if (!byMode[mode]) byMode[mode] = [];
      byMode[mode].push({
        id: p.id,
        pid: p.pid,
        endpoint: p.endpoint,
        uptime: Date.now() - (p.startedAt || Date.now()),
      });
    }

    return {
      total: processes.length,
      healthy: processes.length - stale.length,
      stale: stale.length,
      byMode,
      selfId: this._processId,
    };
  }

  // ──────────────────────────────────────────────────────────
  // DEPART — "I'm leaving"
  // Called on process shutdown. Removes our entry.
  // ──────────────────────────────────────────────────────────

  /**
   * Remove this process from the registry.
   * Called automatically on process exit, or manually.
   */
  depart() {
    if (!this._processId) return;

    this._stopHeartbeat();

    try {
      const registry = this._readRegistry();
      delete registry[this._processId];
      this._writeRegistry(registry);
      log.info(`Departed: ${this._processId}`);
    } catch {
      // Best-effort — process is dying anyway
    }

    this._processId = null;
    this._processInfo = null;
  }

  // ──────────────────────────────────────────────────────────
  // HEARTBEAT — keep our entry fresh
  // ──────────────────────────────────────────────────────────

  /** @private */
  _startHeartbeat() {
    this._stopHeartbeat();
    this._heartbeatTimer = setInterval(() => {
      if (this._processId && this._processInfo) {
        this._processInfo.lastHeartbeat = Date.now();
        this._writeEntry(this._processId, this._processInfo);
      }
    }, this._heartbeatInterval);

    // Don't let heartbeat prevent process exit
    if (this._heartbeatTimer.unref) {
      this._heartbeatTimer.unref();
    }
  }

  /** @private */
  _stopHeartbeat() {
    if (this._heartbeatTimer) {
      clearInterval(this._heartbeatTimer);
      this._heartbeatTimer = null;
    }
  }

  // ──────────────────────────────────────────────────────────
  // SHUTDOWN CLEANUP — auto-depart on process exit
  // ──────────────────────────────────────────────────────────

  /** @private */
  _registerShutdownCleanup() {
    // Clean up previous handlers (in case of re-announce)
    this._removeShutdownHandlers();

    const onExit = () => this.depart();
    const onSignal = (signal) => {
      this.depart();
      process.exit(signal === 'SIGTERM' ? 0 : 1);
    };

    process.on('exit', onExit);
    process.on('SIGINT', onSignal);
    process.on('SIGTERM', onSignal);

    this._shutdownHandlers = [
      { event: 'exit', handler: onExit },
      { event: 'SIGINT', handler: onSignal },
      { event: 'SIGTERM', handler: onSignal },
    ];
  }

  /** @private */
  _removeShutdownHandlers() {
    for (const { event, handler } of this._shutdownHandlers) {
      process.removeListener(event, handler);
    }
    this._shutdownHandlers = [];
  }

  // ──────────────────────────────────────────────────────────
  // FILE I/O — read/write ~/.cynic/processes/registry.json
  // ──────────────────────────────────────────────────────────

  /** @private */
  _ensureDir() {
    if (!fs.existsSync(this._registryDir)) {
      fs.mkdirSync(this._registryDir, { recursive: true });
    }
  }

  /** @private */
  _readRegistry() {
    try {
      if (!fs.existsSync(this._registryFile)) return {};
      const raw = fs.readFileSync(this._registryFile, 'utf8');
      return JSON.parse(raw) || {};
    } catch {
      return {};
    }
  }

  /** @private */
  _writeRegistry(data) {
    this._ensureDir();
    fs.writeFileSync(this._registryFile, JSON.stringify(data, null, 2));
  }

  /** @private Write a single entry (read-modify-write) */
  _writeEntry(id, info) {
    try {
      const registry = this._readRegistry();
      registry[id] = info;
      this._writeRegistry(registry);
    } catch (err) {
      log.debug(`Write entry failed: ${err.message}`);
    }
  }

  /** @private Prune to only include given entries */
  _pruneRegistry(active, stale) {
    try {
      const registry = {};
      for (const entry of [...active, ...stale]) {
        const { id, ...rest } = entry;
        registry[id] = rest;
      }
      this._writeRegistry(registry);
    } catch {
      // Best-effort
    }
  }

  // ──────────────────────────────────────────────────────────
  // RESET (for testing)
  // ──────────────────────────────────────────────────────────

  /**
   * Use a custom registry path (for test isolation).
   * Call BEFORE announce/discover.
   * @param {string} filePath - Absolute path to registry JSON file
   */
  _setRegistryPath(filePath) {
    this._registryFile = filePath;
    this._registryDir = path.dirname(filePath);
  }

  _resetForTesting() {
    this._stopHeartbeat();
    this._removeShutdownHandlers();
    this._processId = null;
    this._processInfo = null;
    this._heartbeatInterval = DEFAULT_HEARTBEAT_MS;

    // Clean up registry file
    try {
      if (fs.existsSync(this._registryFile)) {
        fs.unlinkSync(this._registryFile);
      }
    } catch {
      // Best-effort
    }

    // Reset to default path
    this._registryFile = REGISTRY_FILE;
    this._registryDir = REGISTRY_DIR;
  }

  /**
   * Get the registry file path (for testing/diagnostics)
   */
  get registryPath() {
    return this._registryFile;
  }
}

// ──────────────────────────────────────────────────────────────
// SINGLETON EXPORT
// ──────────────────────────────────────────────────────────────

export const processRegistry = new ProcessRegistry();
export default processRegistry;
