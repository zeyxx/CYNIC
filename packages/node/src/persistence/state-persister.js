/**
 * State Persister - Crash Resilience Heartbeat
 *
 * Saves daemon state to PostgreSQL every 30 seconds.
 * Enables recovery after BSOD, power loss, process kill.
 *
 * "φ survives the crash" - κυνικός
 *
 * @module @cynic/node/persistence/state-persister
 */

'use strict';

import { createLogger, PHI_INV_2 } from '@cynic/core';
import { getPool } from '@cynic/persistence';
import { execFileNoThrow } from '../../../core/src/utils/execFileNoThrow.js';

const log = createLogger('StatePersister');

/**
 * Heartbeat interval (30 seconds, φ⁻² × 100s = 38.2s, rounded to 30s for cleaner timing)
 */
const HEARTBEAT_INTERVAL_MS = 30_000; // 30 seconds

/**
 * StatePersister - Periodic state snapshots for crash recovery
 *
 * Saves state to migration 041 tables:
 * - session_state: Conversation context
 * - watcher_state: Polling offsets (Solana, FileWatcher)
 * - loop_persistence_state: Q-tables, Thompson distributions
 *
 * @example
 * const persister = new StatePersister({
 *   daemonServices,
 *   sessionId: 'daemon-main',
 * });
 * await persister.start();
 */
export class StatePersister {
  /**
   * @param {Object} options
   * @param {Object} options.daemonServices - DaemonServices instance
   * @param {string} [options.sessionId='daemon'] - Session ID
   * @param {number} [options.intervalMs=30000] - Heartbeat interval (ms)
   */
  constructor(options = {}) {
    this.daemonServices = options.daemonServices;
    this.sessionId = options.sessionId || 'daemon';
    this.intervalMs = options.intervalMs || HEARTBEAT_INTERVAL_MS;

    this.pool = null;
    this.heartbeatTimer = null;
    this._isRunning = false;
    this._heartbeatCount = 0;
    this._lastHeartbeatTime = null;
    this._lastError = null;
  }

  /**
   * Start periodic heartbeats
   *
   * @returns {Promise<void>}
   */
  async start() {
    if (this._isRunning) {
      log.warn('StatePersister already running');
      return;
    }

    this.pool = getPool();
    this._isRunning = true;

    // Initial heartbeat immediately
    await this._heartbeat();

    // Schedule periodic heartbeats
    this.heartbeatTimer = setInterval(async () => {
      await this._heartbeat();
    }, this.intervalMs);

    // Don't block process exit on heartbeat timer
    this.heartbeatTimer.unref();

    log.info('StatePersister started', {
      sessionId: this.sessionId,
      intervalMs: this.intervalMs,
    });
  }

  /**
   * Stop periodic heartbeats
   *
   * @returns {Promise<void>}
   */
  async stop() {
    if (!this._isRunning) {
      return;
    }

    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }

    // Final heartbeat before shutdown
    await this._heartbeat();

    this._isRunning = false;
    log.info('StatePersister stopped', {
      totalHeartbeats: this._heartbeatCount,
    });
  }

  /**
   * Execute a heartbeat (save all state)
   *
   * @private
   * @returns {Promise<void>}
   */
  async _heartbeat() {
    const startTime = Date.now();

    try {
      // Save all state in parallel (non-blocking)
      await Promise.all([
        this._saveSessionState(),
        this._saveWatcherState(),
        this._saveLearningState(),
      ]);

      this._heartbeatCount++;
      this._lastHeartbeatTime = Date.now();
      this._lastError = null;

      const durationMs = Date.now() - startTime;

      log.debug('Heartbeat saved', {
        count: this._heartbeatCount,
        durationMs,
      });

    } catch (error) {
      this._lastError = error.message;
      log.error('Heartbeat failed', {
        error: error.message,
        count: this._heartbeatCount,
      });
    }
  }

  /**
   * Save session state (conversation context)
   *
   * @private
   * @returns {Promise<void>}
   */
  async _saveSessionState() {
    if (!this.pool) return;

    try {
      // Get git state (safe: no user input)
      const gitBranch = await this._getGitBranch();
      const gitCommitSha = await this._getGitCommitSha();
      const workingDirectory = process.cwd();

      // Get daemon status
      const daemonStatus = this.daemonServices?.getStatus?.() || {};

      await this.pool.query(`
        INSERT INTO session_state (
          session_id,
          turn_number,
          working_directory,
          git_branch,
          git_commit_sha,
          metadata
        ) VALUES ($1, $2, $3, $4, $5, $6)
      `, [
        this.sessionId,
        this._heartbeatCount,
        workingDirectory,
        gitBranch,
        gitCommitSha,
        JSON.stringify({
          daemonUptime: daemonStatus.uptime || 0,
          servicesRunning: daemonStatus.isRunning || false,
          heartbeatCount: this._heartbeatCount,
        }),
      ]);
    } catch (error) {
      log.debug('Failed to save session state', { error: error.message });
    }
  }

  /**
   * Save watcher state (polling offsets)
   *
   * @private
   * @returns {Promise<void>}
   */
  async _saveWatcherState() {
    if (!this.pool) return;

    try {
      const services = this.daemonServices;
      if (!services) return;

      // FileWatcher state
      if (services.filesystemWatcher?._isRunning) {
        const stats = services.filesystemWatcher.getStats?.() || {};
        await this._upsertWatcherState('filesystem', {
          state_snapshot: JSON.stringify({
            eventsEmitted: stats.eventsEmitted || 0,
            filesWatched: stats.filesWatched || 0,
            startedAt: stats.startedAt || null,
          }),
        });
      }

      // SolanaWatcher state (if exists)
      if (services.solanaWatcher?._isRunning) {
        const lastSignature = services.solanaWatcher._lastSignature || null;
        const lastSlot = services.solanaWatcher._lastSlot || null;
        await this._upsertWatcherState('solana', {
          last_polled_signature: lastSignature,
          last_polled_slot: lastSlot,
          last_polled_timestamp: new Date(),
        });
      }

      // MarketWatcher state (if exists)
      if (services.marketWatcher?._isRunning) {
        const lastPrice = services.marketWatcher._lastPrice || null;
        await this._upsertWatcherState('market', {
          state_snapshot: JSON.stringify({
            lastPrice,
            lastUpdate: services.marketWatcher._lastUpdate || null,
          }),
        });
      }
    } catch (error) {
      log.debug('Failed to save watcher state', { error: error.message });
    }
  }

  /**
   * Upsert watcher state (INSERT or UPDATE)
   *
   * @private
   * @param {string} watcherName - Watcher name
   * @param {Object} data - State data
   */
  async _upsertWatcherState(watcherName, data) {
    const fields = [];
    const values = [watcherName];
    let paramIndex = 2;

    // Build dynamic field list
    for (const [key, value] of Object.entries(data)) {
      if (value !== undefined && value !== null) {
        fields.push(`${key} = $${paramIndex}`);
        values.push(value);
        paramIndex++;
      }
    }

    const setClause = fields.join(', ');

    await this.pool.query(`
      INSERT INTO watcher_state (watcher_name, ${Object.keys(data).join(', ')})
      VALUES ($1, ${values.slice(1).map((_, i) => `$${i + 2}`).join(', ')})
      ON CONFLICT (watcher_name)
      DO UPDATE SET ${setClause}, timestamp = NOW()
    `, values);
  }

  /**
   * Save learning state (Q-tables, Thompson Sampling)
   *
   * @private
   * @returns {Promise<void>}
   */
  async _saveLearningState() {
    if (!this.pool) return;

    try {
      const services = this.daemonServices;
      if (!services) return;

      // Q-Learning state
      if (services.learningService) {
        const stats = services.learningService.getStats?.() || {};
        const qTableStats = stats.qTableStats || {};

        if (qTableStats.states > 0) {
          // Get Q-table weights (state-action values)
          const weights = services.learningService.getQTable?.() || {};

          await this._upsertLearningState('q-learning', 'q_table', {
            weights: JSON.stringify(weights),
            episode_count: stats.episodes || 0,
          });
        }
      }

      // Thompson Sampling state
      if (services.kabbalisticRouter?.thompsonSampler) {
        const thompsonState = services.kabbalisticRouter.thompsonSampler.exportState?.() || {};
        const arms = thompsonState.arms || {};

        if (Object.keys(arms).length > 0) {
          await this._upsertLearningState('thompson-sampling', 'thompson_beta', {
            weights: JSON.stringify(arms), // { dog: { alpha, beta, pulls } }
            episode_count: thompsonState.totalPulls || 0,
          });
        }
      }

      // SONA weights (if exists)
      if (services.sonaService) {
        const sonaWeights = services.sonaService.getWeights?.() || {};
        if (Object.keys(sonaWeights).length > 0) {
          await this._upsertLearningState('sona', 'sona_weights', {
            weights: JSON.stringify(sonaWeights),
            episode_count: 0,
          });
        }
      }
    } catch (error) {
      log.debug('Failed to save learning state', { error: error.message });
    }
  }

  /**
   * Upsert learning state (INSERT or UPDATE)
   *
   * @private
   * @param {string} loopName - Learning loop name
   * @param {string} stateType - State type (q_table, thompson_beta, etc.)
   * @param {Object} data - State data { weights, episode_count }
   */
  async _upsertLearningState(loopName, stateType, data) {
    await this.pool.query(`
      INSERT INTO loop_persistence_state (loop_name, state_type, weights, episode_count)
      VALUES ($1, $2, $3, $4)
      ON CONFLICT (loop_name, state_type)
      DO UPDATE SET
        weights = $3,
        episode_count = $4,
        timestamp = NOW()
    `, [
      loopName,
      stateType,
      data.weights,
      data.episode_count,
    ]);
  }

  /**
   * Get current git branch (using safe execFileNoThrow)
   *
   * @private
   * @returns {Promise<string|null>}
   */
  async _getGitBranch() {
    try {
      const result = await execFileNoThrow('git', ['rev-parse', '--abbrev-ref', 'HEAD']);
      return result.code === 0 ? result.stdout.trim() : null;
    } catch {
      return null;
    }
  }

  /**
   * Get current git commit SHA (using safe execFileNoThrow)
   *
   * @private
   * @returns {Promise<string|null>}
   */
  async _getGitCommitSha() {
    try {
      const result = await execFileNoThrow('git', ['rev-parse', 'HEAD']);
      return result.code === 0 ? result.stdout.trim() : null;
    } catch {
      return null;
    }
  }

  /**
   * Get persister status
   *
   * @returns {Object}
   */
  getStatus() {
    return {
      isRunning: this._isRunning,
      sessionId: this.sessionId,
      heartbeatCount: this._heartbeatCount,
      lastHeartbeat: this._lastHeartbeatTime,
      intervalMs: this.intervalMs,
      lastError: this._lastError,
    };
  }
}

export default StatePersister;
