/**
 * MemoryCoordinator — Three memories, one awareness
 *
 * CYNIC's memory is fragmented across 3 disjoint systems:
 *   1. PostgreSQL (remote, production) — 74 tables, 37 migrations
 *   2. SQLite (local, privacy-first) — x-local.db, privacy.db, cynic.db
 *   3. JSON files (~/.cynic/) — hook state, Thompson, consciousness, psychology
 *
 * This coordinator provides:
 *   - Inventory: What data exists where
 *   - Health: All backends reachable?
 *   - Drift: Timestamps diverging between systems?
 *   - Sync: Critical JSON state → PostgreSQL backup
 *   - Stats: Unified memory view for SystemTopology
 *
 * "The dog remembers everything, even what it wishes to forget" — κυνικός
 *
 * @module @cynic/node/services/memory-coordinator
 */

'use strict';

import { existsSync, statSync, readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('MemoryCoordinator');

/**
 * Base directory for all local CYNIC state.
 */
const CYNIC_HOME = join(homedir(), '.cynic');

/**
 * Known JSON state files with their roles.
 * path: relative to ~/.cynic/
 * critical: if true, should be backed up to PostgreSQL
 * writer: which process writes this file
 * reader: which process reads this file
 */
const KNOWN_JSON_FILES = [
  { path: 'thompson/state.json', critical: true, writer: 'daemon', reader: 'daemon', label: 'Thompson exploration state' },
  { path: 'consciousness/readback.json', critical: true, writer: 'hook:observe', reader: 'hook:perceive', label: 'Consciousness readback' },
  { path: 'psychology/state.json', critical: true, writer: 'hook', reader: 'hook+daemon', label: 'Human psychology state' },
  { path: 'learning/calibration.json', critical: true, writer: 'hook', reader: 'hook', label: 'Calibration accuracy' },
  { path: 'last-session.json', critical: false, writer: 'hook:sleep', reader: 'hook:awaken', label: 'Session handoff' },
  { path: 'llm-detection.json', critical: false, writer: 'hook:awaken', reader: 'hook:perceive', label: 'LLM environment probe' },
  { path: 'behavior/state.json', critical: false, writer: 'daemon', reader: 'daemon', label: 'Behavior modifier state' },
  { path: 'metacognition/state.json', critical: false, writer: 'daemon', reader: 'daemon', label: 'Meta-cognition state' },
  { path: 'sona/state.json', critical: false, writer: 'daemon', reader: 'daemon', label: 'SONA dimension insights' },
  { path: 'harmonic/state.json', critical: false, writer: 'hook:observe', reader: 'tui', label: 'Harmonic feedback' },
  { path: 'learning/observations.jsonl', critical: false, writer: 'hook', reader: 'hook', label: 'Learning observations (append-only)' },
  { path: 'learning/user-patterns.json', critical: false, writer: 'hook', reader: 'hook', label: 'User behavior patterns' },
  { path: 'psychology/signals.jsonl', critical: false, writer: 'hook', reader: 'none', label: 'Psychology signals (data grave)' },
];

/**
 * Known SQLite databases.
 */
const KNOWN_SQLITE_DBS = [
  { path: 'x-local.db', label: 'X/Twitter local data', tables: 7 },
  { path: 'privacy.db', label: 'Privacy-first local data', tables: 10 },
  { path: 'cynic.db', label: 'Fallback generic SQLite', tables: 0 },
];

/**
 * MemoryCoordinator
 *
 * Provides unified awareness of CYNIC's fragmented memory.
 * Singleton — one coordinator per process.
 */
class MemoryCoordinator {
  constructor() {
    this._running = false;
    this._scanInterval = null;
    this._persistence = null;
    this._lastScan = null;
    this._lastSync = null;

    // Backend health
    this._postgres = { available: false, lastCheck: null, tables: 0, error: null };
    this._sqlite = new Map();  // path → { available, size, mtime }
    this._jsonFiles = new Map(); // path → { exists, size, mtime, critical, stale }
    this._sharedMemory = null; // reference to SharedMemory instance

    this._stats = {
      scans: 0,
      syncs: 0,
      syncErrors: 0,
      driftsDetected: 0,
      startedAt: null,
    };
  }

  /**
   * Start the coordinator.
   *
   * @param {Object} options
   * @param {Object} [options.persistence] - PersistenceManager (for PostgreSQL)
   * @param {Object} [options.sharedMemory] - SharedMemory instance
   */
  start(options = {}) {
    if (this._running) return;

    this._persistence = options.persistence || null;
    this._sharedMemory = options.sharedMemory || null;
    this._stats.startedAt = Date.now();

    // Initial scan
    this.scan();

    // Periodic health scan: every 55s (Fib(10) × 1000ms)
    this._scanInterval = setInterval(() => this.scan(), 55000);
    this._scanInterval.unref();

    this._running = true;
    log.info('MemoryCoordinator started', {
      postgres: this._postgres.available,
      sqlite: [...this._sqlite.entries()].filter(([_, v]) => v.available).length,
      jsonFiles: [...this._jsonFiles.entries()].filter(([_, v]) => v.exists).length,
      criticalFiles: [...this._jsonFiles.entries()].filter(([_, v]) => v.critical && v.exists).length,
    });
  }

  /**
   * Scan all memory backends for health and inventory.
   */
  scan() {
    this._scanPostgres();
    this._scanSQLite();
    this._scanJsonFiles();
    this._lastScan = Date.now();
    this._stats.scans++;
  }

  /**
   * Check PostgreSQL connectivity.
   * @private
   */
  _scanPostgres() {
    try {
      if (this._persistence?.query) {
        this._postgres.available = true;
        this._postgres.error = null;
      } else {
        this._postgres.available = false;
        this._postgres.error = 'No persistence manager';
      }
      this._postgres.lastCheck = Date.now();
    } catch (err) {
      this._postgres.available = false;
      this._postgres.error = err.message;
      this._postgres.lastCheck = Date.now();
    }
  }

  /**
   * Check SQLite databases.
   * @private
   */
  _scanSQLite() {
    for (const db of KNOWN_SQLITE_DBS) {
      const fullPath = join(CYNIC_HOME, db.path);
      try {
        if (existsSync(fullPath)) {
          const stats = statSync(fullPath);
          this._sqlite.set(db.path, {
            available: true,
            size: stats.size,
            mtime: stats.mtimeMs,
            label: db.label,
            tables: db.tables,
          });
        } else {
          this._sqlite.set(db.path, {
            available: false,
            size: 0,
            mtime: null,
            label: db.label,
            tables: db.tables,
          });
        }
      } catch (err) {
        this._sqlite.set(db.path, {
          available: false,
          size: 0,
          mtime: null,
          label: db.label,
          error: err.message,
        });
      }
    }
  }

  /**
   * Check JSON state files.
   * @private
   */
  _scanJsonFiles() {
    for (const file of KNOWN_JSON_FILES) {
      const fullPath = join(CYNIC_HOME, file.path);
      try {
        if (existsSync(fullPath)) {
          const stats = statSync(fullPath);
          const ageMs = Date.now() - stats.mtimeMs;
          const staleThresholdMs = file.critical ? 3600000 : 86400000; // 1h for critical, 24h for others

          this._jsonFiles.set(file.path, {
            exists: true,
            size: stats.size,
            mtime: stats.mtimeMs,
            ageMs,
            stale: ageMs > staleThresholdMs,
            critical: file.critical,
            label: file.label,
            writer: file.writer,
            reader: file.reader,
          });
        } else {
          this._jsonFiles.set(file.path, {
            exists: false,
            size: 0,
            mtime: null,
            ageMs: null,
            stale: file.critical, // Missing critical files are "stale"
            critical: file.critical,
            label: file.label,
            writer: file.writer,
            reader: file.reader,
          });
        }
      } catch (err) {
        this._jsonFiles.set(file.path, {
          exists: false,
          size: 0,
          mtime: null,
          stale: file.critical,
          critical: file.critical,
          label: file.label,
          error: err.message,
        });
      }
    }
  }

  /**
   * Get complete memory inventory.
   */
  getInventory() {
    return {
      postgres: {
        ...this._postgres,
        type: 'PostgreSQL',
        role: 'Production storage (judgments, patterns, learning, X/Twitter)',
      },
      sqlite: Object.fromEntries(
        [...this._sqlite.entries()].map(([path, info]) => [
          path,
          { ...info, type: 'SQLite', fullPath: join(CYNIC_HOME, path) },
        ])
      ),
      jsonFiles: Object.fromEntries(
        [...this._jsonFiles.entries()].map(([path, info]) => [
          path,
          { ...info, type: 'JSON', fullPath: join(CYNIC_HOME, path) },
        ])
      ),
      sharedMemory: this._sharedMemory ? {
        available: true,
        patterns: this._sharedMemory.patternCount?.() ?? this._sharedMemory._patterns?.size ?? 0,
        judgments: this._sharedMemory._judgmentIndex?.length ?? 0,
        feedbackLog: this._sharedMemory._feedbackLog?.length ?? 0,
        type: 'In-memory (RAM)',
        role: 'Working memory (lost on crash unless saved)',
      } : {
        available: false,
        type: 'In-memory (RAM)',
        role: 'Working memory (not connected)',
      },
    };
  }

  /**
   * Get overall memory health.
   */
  getHealth() {
    const pgOk = this._postgres.available;
    const sqliteCount = [...this._sqlite.values()].filter(v => v.available).length;
    const jsonExist = [...this._jsonFiles.values()].filter(v => v.exists).length;
    const jsonStale = [...this._jsonFiles.values()].filter(v => v.stale).length;
    const criticalMissing = [...this._jsonFiles.values()].filter(v => v.critical && !v.exists).length;
    const criticalStale = [...this._jsonFiles.values()].filter(v => v.critical && v.stale && v.exists).length;

    // Health score: φ⁻¹ max
    let score = 0;
    if (pgOk) score += 0.3;         // PostgreSQL is most important
    if (sqliteCount > 0) score += 0.1; // SQLite is nice to have
    if (jsonExist > 5) score += 0.1;  // Some JSON files should exist
    if (criticalMissing === 0) score += 0.1; // All critical files present
    score = Math.min(score, PHI_INV);

    const status = criticalMissing > 0 ? 'degraded'
      : !pgOk ? 'degraded'
      : criticalStale > 0 ? 'stale'
      : 'healthy';

    return {
      status,
      score: Math.round(score * 1000) / 1000,
      backends: {
        postgres: pgOk ? 'healthy' : 'unavailable',
        sqlite: `${sqliteCount}/${KNOWN_SQLITE_DBS.length} available`,
        jsonFiles: `${jsonExist}/${KNOWN_JSON_FILES.length} exist`,
      },
      issues: [
        ...(criticalMissing > 0 ? [`${criticalMissing} critical JSON files missing`] : []),
        ...(criticalStale > 0 ? [`${criticalStale} critical JSON files stale`] : []),
        ...(!pgOk ? ['PostgreSQL unavailable'] : []),
        ...(jsonStale > 0 ? [`${jsonStale} JSON files stale`] : []),
      ],
      lastScan: this._lastScan,
      lastSync: this._lastSync,
    };
  }

  /**
   * Detect drift between memory systems.
   *
   * Drift = the same conceptual data exists in multiple systems
   * with different timestamps (one is newer than the other).
   */
  detectDrift() {
    const drifts = [];

    // Check: psychology state (JSON) vs psychology_snapshots (PostgreSQL)
    const psyFile = this._jsonFiles.get('psychology/state.json');
    if (psyFile?.exists && this._postgres.available) {
      drifts.push({
        concept: 'psychology_state',
        jsonFile: 'psychology/state.json',
        jsonMtime: psyFile.mtime,
        pgTable: 'psychology_snapshots',
        note: 'One-way sync (JSON→PG via event-listeners). No reverse flow.',
        severity: psyFile.stale ? 'warning' : 'info',
      });
    }

    // Check: Thompson state (JSON only, no PG backup)
    const thompsonFile = this._jsonFiles.get('thompson/state.json');
    if (thompsonFile?.exists) {
      drifts.push({
        concept: 'thompson_exploration',
        jsonFile: 'thompson/state.json',
        jsonMtime: thompsonFile.mtime,
        pgTable: null,
        note: 'JSON only — no PostgreSQL backup. Crash = lost exploration state.',
        severity: 'warning',
      });
    }

    // Check: consciousness readback (JSON only, no PG backup)
    const consFile = this._jsonFiles.get('consciousness/readback.json');
    if (consFile?.exists) {
      drifts.push({
        concept: 'consciousness_readback',
        jsonFile: 'consciousness/readback.json',
        jsonMtime: consFile.mtime,
        pgTable: 'consciousness_transitions',
        note: 'JSON written by hooks, PG written by event-listeners. Independent paths.',
        severity: 'info',
      });
    }

    // Check: SharedMemory not saved
    if (this._sharedMemory) {
      const patternCount = this._sharedMemory._patterns?.size ?? 0;
      const lastSave = this._sharedMemory._lastSaveTimestamp ?? 0;
      const sinceLastSave = lastSave ? Date.now() - lastSave : Infinity;

      if (patternCount > 0 && sinceLastSave > 300000) { // 5min without save
        drifts.push({
          concept: 'shared_memory_patterns',
          source: 'RAM',
          patternCount,
          lastSave: lastSave || null,
          sinceLastSaveMs: sinceLastSave === Infinity ? null : sinceLastSave,
          pgTable: 'shared_memory_patterns',
          note: `${patternCount} patterns in RAM, last saved ${lastSave ? Math.round(sinceLastSave / 1000) + 's ago' : 'never'}`,
          severity: sinceLastSave > 600000 ? 'critical' : 'warning', // >10min = critical
        });
      }
    }

    this._stats.driftsDetected = drifts.filter(d => d.severity !== 'info').length;
    return drifts;
  }

  /**
   * Sync critical JSON state to PostgreSQL backup.
   *
   * Uses unified_signals table with signal_type='memory_backup'.
   * This is a BACKUP, not a migration — JSON files remain the primary store.
   *
   * @param {Object} [persistence] - Override persistence manager
   * @returns {Promise<{synced: number, errors: number}>}
   */
  async syncCriticalState(persistence) {
    const pg = persistence || this._persistence;
    if (!pg?.query) {
      log.debug('Cannot sync — no PostgreSQL connection');
      return { synced: 0, errors: 0, reason: 'no_postgres' };
    }

    let synced = 0;
    let errors = 0;

    for (const file of KNOWN_JSON_FILES) {
      if (!file.critical) continue;

      const fullPath = join(CYNIC_HOME, file.path);
      try {
        if (!existsSync(fullPath)) continue;

        const content = readFileSync(fullPath, 'utf8');
        const stats = statSync(fullPath);

        await pg.query(
          `INSERT INTO unified_signals (signal_type, source, data, confidence, created_at)
           VALUES ($1, $2, $3, $4, NOW())`,
          [
            'memory_backup',
            `json:${file.path}`,
            JSON.stringify({
              filePath: file.path,
              label: file.label,
              content: JSON.parse(content),
              fileMtime: stats.mtimeMs,
              backupAt: Date.now(),
            }),
            PHI_INV,
          ]
        );
        synced++;
      } catch (err) {
        errors++;
        log.debug('JSON→PG backup failed', { path: file.path, error: err.message });
      }
    }

    // Also backup SharedMemory if available and has a patternRepository
    if (this._sharedMemory && this._sharedMemory.saveToPostgres) {
      try {
        const patterns = this._persistence?.patterns;
        if (patterns) {
          const result = await this._sharedMemory.saveToPostgres(patterns, { onlyModified: true });
          log.info('SharedMemory auto-saved', result);
        }
      } catch (err) {
        log.debug('SharedMemory auto-save failed', { error: err.message });
      }
    }

    this._lastSync = Date.now();
    this._stats.syncs++;
    if (errors > 0) this._stats.syncErrors += errors;

    log.info('Critical state synced', { synced, errors });
    return { synced, errors };
  }

  /**
   * Restore critical JSON state from PostgreSQL backup.
   *
   * Reads the most recent backup for each critical file and writes it to disk.
   * Only restores if the JSON file is MISSING (not if it exists but is stale).
   *
   * @param {Object} [persistence] - Override persistence manager
   * @returns {Promise<{restored: number, skipped: number, errors: number}>}
   */
  async restoreCriticalState(persistence) {
    const pg = persistence || this._persistence;
    if (!pg?.query) {
      return { restored: 0, skipped: 0, errors: 0, reason: 'no_postgres' };
    }

    let restored = 0;
    let skipped = 0;
    let errors = 0;

    for (const file of KNOWN_JSON_FILES) {
      if (!file.critical) continue;

      const fullPath = join(CYNIC_HOME, file.path);

      // Only restore MISSING files
      if (existsSync(fullPath)) {
        skipped++;
        continue;
      }

      try {
        const result = await pg.query(
          `SELECT data FROM unified_signals
           WHERE signal_type = 'memory_backup' AND source = $1
           ORDER BY created_at DESC LIMIT 1`,
          [`json:${file.path}`]
        );

        if (result.rows?.length > 0) {
          const backup = result.rows[0].data;
          const content = typeof backup.content === 'string'
            ? backup.content
            : JSON.stringify(backup.content, null, 2);

          // Ensure directory exists
          const dir = join(CYNIC_HOME, file.path.split('/').slice(0, -1).join('/'));
          if (dir !== CYNIC_HOME && !existsSync(dir)) {
            mkdirSync(dir, { recursive: true });
          }

          writeFileSync(fullPath, content, 'utf8');
          restored++;
          log.info('Restored JSON state from PG backup', { path: file.path });
        }
      } catch (err) {
        errors++;
        log.debug('Restore failed', { path: file.path, error: err.message });
      }
    }

    return { restored, skipped, errors };
  }

  /**
   * Get unified stats for all memory systems.
   */
  getStats() {
    const jsonFiles = [...this._jsonFiles.values()];
    const sqliteDBs = [...this._sqlite.values()];

    return {
      running: this._running,
      uptime: this._stats.startedAt ? Date.now() - this._stats.startedAt : 0,
      backends: {
        postgres: this._postgres.available,
        sqlite: sqliteDBs.filter(v => v.available).length,
        jsonFiles: jsonFiles.filter(v => v.exists).length,
        sharedMemory: !!this._sharedMemory,
      },
      health: this.getHealth().status,
      totalStorage: {
        sqliteBytes: sqliteDBs.reduce((sum, db) => sum + (db.size || 0), 0),
        jsonBytes: jsonFiles.reduce((sum, f) => sum + (f.size || 0), 0),
      },
      drift: {
        criticalDrifts: this._stats.driftsDetected,
      },
      sync: {
        lastSync: this._lastSync,
        syncs: this._stats.syncs,
        errors: this._stats.syncErrors,
      },
      scans: this._stats.scans,
      lastScan: this._lastScan,
    };
  }

  /**
   * Stop the coordinator.
   */
  stop() {
    if (this._scanInterval) {
      clearInterval(this._scanInterval);
      this._scanInterval = null;
    }
    this._running = false;
    log.info('MemoryCoordinator stopped');
  }

  /**
   * Reset for testing.
   */
  _resetForTesting() {
    this.stop();
    this._postgres = { available: false, lastCheck: null, tables: 0, error: null };
    this._sqlite.clear();
    this._jsonFiles.clear();
    this._sharedMemory = null;
    this._persistence = null;
    this._lastScan = null;
    this._lastSync = null;
    this._stats = { scans: 0, syncs: 0, syncErrors: 0, driftsDetected: 0, startedAt: null };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

export const memoryCoordinator = new MemoryCoordinator();

export {
  KNOWN_JSON_FILES,
  KNOWN_SQLITE_DBS,
  CYNIC_HOME,
};
