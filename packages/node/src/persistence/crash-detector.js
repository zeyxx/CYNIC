/**
 * Crash Detector - Detects and recovers from daemon crashes
 *
 * "φ survives even its own death" - κυνικός
 *
 * Detects crash by checking if previous session ended cleanly.
 * Restores state from PostgreSQL if crash detected.
 *
 * @module @cynic/node/persistence/crash-detector
 */

'use strict';

import { getPool } from '@cynic/persistence';
import { globalEventBus } from '@cynic/core';
import { createLogger } from '@cynic/core';

const log = createLogger('CrashDetector');

// Consider session crashed if no heartbeat for 5 minutes
const CRASH_THRESHOLD_MS = 5 * 60 * 1000;

/**
 * Crash Detector
 *
 * Detects daemon crashes and restores state.
 */
export class CrashDetector {
  constructor(options = {}) {
    this.pool = options.pool || getPool();
    this.sessionId = options.sessionId || 'daemon';
  }

  /**
   * Check for previous crash and restore state
   *
   * @returns {Promise<Object>} Crash detection result
   */
  async detectAndRecover() {
    log.info('Checking for previous crash...');

    try {
      // Get last session heartbeat (timestamp = heartbeat)
      const { rows } = await this.pool.query(`
        SELECT
          session_id,
          timestamp as last_heartbeat,
          working_directory,
          git_branch,
          metadata
        FROM session_state
        WHERE session_id = $1
        ORDER BY timestamp DESC
        LIMIT 1
      `, [this.sessionId]);

      if (rows.length === 0) {
        log.info('No previous session found (first run)');
        return {
          crashed: false,
          firstRun: true,
        };
      }

      const lastSession = rows[0];
      const now = new Date();
      const lastHeartbeat = new Date(lastSession.last_heartbeat);
      const timeSinceHeartbeat = now - lastHeartbeat;

      // Check if crashed (no heartbeat for >5 minutes)
      const crashed = timeSinceHeartbeat > CRASH_THRESHOLD_MS;

      if (crashed) {
        log.warn('🔥 CRASH DETECTED!', {
          lastHeartbeat: lastHeartbeat.toISOString(),
          timeSinceHeartbeat: Math.round(timeSinceHeartbeat / 1000) + 's',
          sessionStarted: lastSession.started_at,
        });

        // Restore state
        const restoredState = await this._restoreState();

        // Emit recovery event
        globalEventBus.emit('daemon:crash:recovered', {
          sessionId: this.sessionId,
          lastHeartbeat,
          timeSinceHeartbeat,
          restoredState,
          timestamp: now,
        });

        // Log crash in crash_log table
        await this.pool.query(`
          INSERT INTO crash_log (
            crash_type,
            last_session_id,
            last_heartbeat,
            time_offline_ms,
            recovery_timestamp,
            recovery_success,
            metadata
          ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        `, [
          'unknown', // Crash type (could be refined)
          this.sessionId,
          lastHeartbeat,
          timeSinceHeartbeat,
          now,
          true,
          JSON.stringify(restoredState),
        ]);

        return {
          crashed: true,
          lastHeartbeat,
          timeSinceHeartbeat,
          restoredState,
        };
      } else {
        log.info('Previous session ended cleanly', {
          lastHeartbeat: lastHeartbeat.toISOString(),
          timeSinceHeartbeat: Math.round(timeSinceHeartbeat / 1000) + 's',
        });

        return {
          crashed: false,
          lastSession: {
            lastHeartbeat,
            timeSinceHeartbeat,
          },
        };
      }
    } catch (error) {
      log.error('Failed to detect crash', { error: error.message });
      throw error;
    }
  }

  /**
   * Restore state from database after crash
   * @private
   */
  async _restoreState() {
    log.info('Restoring state from database...');

    const state = {
      watchers: {},
      learning: {},
      metadata: {},
    };

    try {
      // Restore watcher state (per-watcher, not per-session)
      const { rows: watcherRows } = await this.pool.query(`
        SELECT
          watcher_name,
          last_polled_signature,
          last_polled_slot,
          file_checksums,
          state_snapshot
        FROM watcher_state
        ORDER BY timestamp DESC
      `);

      for (const row of watcherRows) {
        state.watchers[row.watcher_name] = {
          lastPolledSignature: row.last_polled_signature,
          lastPolledSlot: row.last_polled_slot,
          fileChecksums: row.file_checksums,
          stateSnapshot: row.state_snapshot,
        };
      }

      log.info(`Restored ${watcherRows.length} watcher states`);

      // Restore learning state (Q-tables from loop_persistence_state)
      const { rows: learningRows } = await this.pool.query(`
        SELECT loop_name, state_type, weights, episode_count
        FROM loop_persistence_state
        ORDER BY timestamp DESC
      `);

      for (const row of learningRows) {
        if (!state.learning[row.loop_name]) {
          state.learning[row.loop_name] = {};
        }
        state.learning[row.loop_name][row.state_type] = {
          weights: row.weights,
          episodeCount: row.episode_count,
        };
      }

      log.info(`Restored ${learningRows.length} learning loop states`);

      // Restore session metadata
      const { rows: sessionRows } = await this.pool.query(`
        SELECT metadata
        FROM session_state
        WHERE session_id = $1
        ORDER BY timestamp DESC
        LIMIT 1
      `, [this.sessionId]);

      if (sessionRows.length > 0) {
        state.metadata = sessionRows[0].metadata || {};
        log.info('Restored session metadata');
      }

      return state;
    } catch (error) {
      log.error('Failed to restore state', { error: error.message });
      throw error;
    }
  }

  /**
   * Get crash history
   *
   * @param {number} [limit=10] - Number of crashes to retrieve
   * @returns {Promise<Array>} Crash history
   */
  async getCrashHistory(limit = 10) {
    try {
      const { rows } = await this.pool.query(`
        SELECT
          last_session_id,
          crash_type,
          last_heartbeat,
          time_offline_ms,
          recovery_timestamp,
          recovery_success,
          metadata
        FROM crash_log
        ORDER BY timestamp DESC
        LIMIT $1
      `, [limit]);

      return rows.map(row => ({
        sessionId: row.last_session_id,
        crashType: row.crash_type,
        lastHeartbeat: row.last_heartbeat,
        timeOfflineMs: parseInt(row.time_offline_ms),
        uptimeSeconds: null, // Not tracked in crash_log
        recoveryTimestamp: row.recovery_timestamp,
        recoverySuccess: row.recovery_success,
        metadata: row.metadata,
      }));
    } catch (error) {
      log.error('Failed to get crash history', { error: error.message });
      throw error;
    }
  }

  /**
   * Get crash statistics
   *
   * @returns {Promise<Object>} Crash statistics
   */
  async getCrashStats() {
    try {
      const { rows } = await this.pool.query(`
        SELECT
          COUNT(*) as total_crashes,
          AVG(time_offline_ms) as avg_offline_ms,
          MAX(recovery_timestamp) as last_crash_at
        FROM crash_log
      `);

      if (rows.length === 0 || rows[0].total_crashes === '0') {
        return {
          totalCrashes: 0,
          avgOfflineMs: null,
          lastCrashAt: null,
        };
      }

      return {
        totalCrashes: parseInt(rows[0].total_crashes),
        avgOfflineMs: parseFloat(rows[0].avg_offline_ms),
        lastCrashAt: rows[0].last_crash_at,
      };
    } catch (error) {
      log.error('Failed to get crash stats', { error: error.message });
      throw error;
    }
  }
}

export default CrashDetector;
