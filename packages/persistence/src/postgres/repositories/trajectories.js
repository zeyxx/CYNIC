/**
 * Trajectories Repository (M3 - ReasoningBank)
 *
 * Stores reasoning trajectories: State → Action → Outcome → Reward
 * Enables success replay and trajectory learning.
 *
 * Inspired by Claude-flow SAFLA's ReasoningBank.
 *
 * @module @cynic/persistence/repositories/trajectories
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

// φ constants
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

/**
 * Trajectory outcomes
 */
export const TrajectoryOutcome = Object.freeze({
  SUCCESS: 'success',
  PARTIAL: 'partial',
  FAILURE: 'failure',
  ABANDONED: 'abandoned',
});

/**
 * Generate trajectory ID
 */
function generateTrajectoryId() {
  return 'traj_' + crypto.randomBytes(8).toString('hex');
}

/**
 * Trajectories Repository (ReasoningBank)
 *
 * Stores State → Action → Outcome chains for learning.
 *
 * @extends BaseRepository
 */
export class TrajectoriesRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Ensure trajectories table exists
   */
  async ensureTable() {
    await this.db.query(`
      CREATE TABLE IF NOT EXISTS trajectories (
        id SERIAL PRIMARY KEY,
        trajectory_id TEXT UNIQUE NOT NULL,
        user_id TEXT,
        session_id TEXT,
        dog_id TEXT,
        task_type TEXT,

        -- State: Initial context
        initial_state JSONB NOT NULL,

        -- Action: What was done
        action_sequence JSONB NOT NULL DEFAULT '[]',

        -- Outcome: Result
        outcome TEXT NOT NULL,
        outcome_details JSONB DEFAULT '{}',

        -- Reward: Learning signal
        reward REAL DEFAULT 0,

        -- Metadata
        duration_ms INTEGER,
        tool_count INTEGER DEFAULT 0,
        error_count INTEGER DEFAULT 0,
        switch_count INTEGER DEFAULT 0,

        -- Learning
        similarity_hash TEXT,
        replay_count INTEGER DEFAULT 0,
        success_after_replay BOOLEAN,
        confidence REAL DEFAULT 0.5,

        -- Searchable
        tags TEXT[] DEFAULT '{}',
        search_vector TSVECTOR,

        created_at TIMESTAMPTZ DEFAULT NOW(),
        updated_at TIMESTAMPTZ DEFAULT NOW()
      );

      CREATE INDEX IF NOT EXISTS idx_traj_user_id ON trajectories(user_id);
      CREATE INDEX IF NOT EXISTS idx_traj_session_id ON trajectories(session_id);
      CREATE INDEX IF NOT EXISTS idx_traj_dog_id ON trajectories(dog_id);
      CREATE INDEX IF NOT EXISTS idx_traj_task_type ON trajectories(task_type);
      CREATE INDEX IF NOT EXISTS idx_traj_outcome ON trajectories(outcome);
      CREATE INDEX IF NOT EXISTS idx_traj_reward ON trajectories(reward DESC);
      CREATE INDEX IF NOT EXISTS idx_traj_similarity ON trajectories(similarity_hash);
      CREATE INDEX IF NOT EXISTS idx_traj_search ON trajectories USING GIN(search_vector);
    `);

    // Search trigger
    await this.db.query(`
      CREATE OR REPLACE FUNCTION traj_search_trigger() RETURNS trigger AS $$
      BEGIN
        NEW.search_vector :=
          setweight(to_tsvector('english', COALESCE(NEW.task_type, '')), 'A') ||
          setweight(to_tsvector('english', COALESCE(NEW.dog_id, '')), 'B') ||
          setweight(to_tsvector('english', COALESCE(array_to_string(NEW.tags, ' '), '')), 'C');
        RETURN NEW;
      END
      $$ LANGUAGE plpgsql;

      DROP TRIGGER IF EXISTS traj_search_update ON trajectories;
      CREATE TRIGGER traj_search_update BEFORE INSERT OR UPDATE
        ON trajectories FOR EACH ROW EXECUTE FUNCTION traj_search_trigger();
    `);
  }

  /**
   * Start a new trajectory
   */
  async start(trajectory) {
    const trajectoryId = generateTrajectoryId();

    const { rows } = await this.db.query(`
      INSERT INTO trajectories (
        trajectory_id, user_id, session_id, dog_id, task_type,
        initial_state, action_sequence, outcome, tags
      ) VALUES ($1, $2, $3, $4, $5, $6, '[]', 'pending', $7)
      RETURNING *
    `, [
      trajectoryId,
      trajectory.userId || null,
      trajectory.sessionId || null,
      trajectory.dogId || null,
      trajectory.taskType || null,
      JSON.stringify(trajectory.initialState || {}),
      trajectory.tags || [],
    ]);

    return this._mapRow(rows[0]);
  }

  /**
   * Record an action in the trajectory
   */
  async recordAction(trajectoryId, action) {
    const actionEntry = {
      timestamp: Date.now(),
      tool: action.tool,
      input: action.input || {},
      output: action.output || null,
      success: action.success !== false,
      durationMs: action.durationMs || 0,
      dogId: action.dogId || null,
    };

    const { rows } = await this.db.query(`
      UPDATE trajectories
      SET action_sequence = action_sequence || $1::jsonb,
          tool_count = tool_count + 1,
          error_count = error_count + CASE WHEN $2 THEN 0 ELSE 1 END,
          updated_at = NOW()
      WHERE trajectory_id = $3
      RETURNING *
    `, [JSON.stringify([actionEntry]), action.success !== false, trajectoryId]);

    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Record a dog switch in the trajectory
   */
  async recordSwitch(trajectoryId, fromDog, toDog, reason) {
    const switchEntry = {
      timestamp: Date.now(),
      type: 'dog_switch',
      from: fromDog,
      to: toDog,
      reason,
    };

    const { rows } = await this.db.query(`
      UPDATE trajectories
      SET action_sequence = action_sequence || $1::jsonb,
          switch_count = switch_count + 1,
          dog_id = $2,
          updated_at = NOW()
      WHERE trajectory_id = $3
      RETURNING *
    `, [JSON.stringify([switchEntry]), toDog, trajectoryId]);

    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Complete a trajectory with outcome
   */
  async complete(trajectoryId, result) {
    const reward = this._calculateReward(result);

    const { rows } = await this.db.query(`
      UPDATE trajectories
      SET outcome = $1,
          outcome_details = $2,
          reward = $3,
          duration_ms = $4,
          confidence = LEAST($5, $6),
          similarity_hash = $7,
          updated_at = NOW()
      WHERE trajectory_id = $8
      RETURNING *
    `, [
      result.outcome || TrajectoryOutcome.SUCCESS,
      JSON.stringify(result.details || {}),
      reward,
      result.durationMs || 0,
      PHI_INV, // Max confidence
      result.confidence || reward,
      this._computeSimilarityHash(result),
      trajectoryId,
    ]);

    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Calculate reward from result
   */
  _calculateReward(result) {
    let reward = 0;

    switch (result.outcome) {
      case TrajectoryOutcome.SUCCESS:
        reward = PHI_INV; // 0.618 base reward
        break;
      case TrajectoryOutcome.PARTIAL:
        reward = PHI_INV_2; // 0.382 partial reward
        break;
      case TrajectoryOutcome.FAILURE:
        reward = -PHI_INV_2; // -0.382 penalty
        break;
      case TrajectoryOutcome.ABANDONED:
        reward = -PHI_INV; // -0.618 penalty
        break;
    }

    // Bonus for efficiency (fewer errors, switches)
    if (result.errorCount === 0) reward += 0.05;
    if (result.switchCount === 0) reward += 0.05;

    // Cap at φ⁻¹
    return Math.max(-1, Math.min(PHI_INV, reward));
  }

  /**
   * Compute similarity hash for finding similar problems
   */
  _computeSimilarityHash(result) {
    const key = [
      result.taskType || '',
      result.dogId || '',
      result.outcome || '',
    ].join(':');

    return crypto.createHash('md5').update(key).digest('hex').slice(0, 16);
  }

  /**
   * Find trajectory by ID
   */
  async findById(trajectoryId) {
    const { rows } = await this.db.query(
      'SELECT * FROM trajectories WHERE trajectory_id = $1',
      [trajectoryId]
    );
    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Find successful trajectories for a task type
   */
  async findSuccessful(taskType, options = {}) {
    const { dogId, limit = 5, minReward = 0.3 } = options;

    let sql = `
      SELECT * FROM trajectories
      WHERE task_type = $1
        AND outcome = 'success'
        AND reward >= $2
    `;
    const params = [taskType, minReward];
    let paramIndex = 3;

    if (dogId) {
      sql += ` AND dog_id = $${paramIndex++}`;
      params.push(dogId);
    }

    sql += ` ORDER BY reward DESC, replay_count ASC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._mapRow(r));
  }

  /**
   * Find similar trajectories by hash
   */
  async findSimilar(similarityHash, limit = 5) {
    const { rows } = await this.db.query(`
      SELECT * FROM trajectories
      WHERE similarity_hash = $1
        AND outcome = 'success'
      ORDER BY reward DESC
      LIMIT $2
    `, [similarityHash, limit]);

    return rows.map(r => this._mapRow(r));
  }

  /**
   * Search trajectories
   */
  async search(query, options = {}) {
    const { userId, taskType, outcome, limit = 10 } = options;

    let sql = `
      SELECT *,
        ts_rank(search_vector, websearch_to_tsquery('english', $1)) as rank
      FROM trajectories
      WHERE search_vector @@ websearch_to_tsquery('english', $1)
    `;
    const params = [query];
    let paramIndex = 2;

    if (userId) {
      sql += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (taskType) {
      sql += ` AND task_type = $${paramIndex++}`;
      params.push(taskType);
    }

    if (outcome) {
      sql += ` AND outcome = $${paramIndex++}`;
      params.push(outcome);
    }

    sql += ` ORDER BY rank DESC, reward DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._mapRow(r));
  }

  /**
   * Record that a trajectory was replayed
   */
  async recordReplay(trajectoryId, success) {
    const { rows } = await this.db.query(`
      UPDATE trajectories
      SET replay_count = replay_count + 1,
          success_after_replay = $1,
          confidence = CASE
            WHEN $1 THEN LEAST(1.0, confidence + 0.05)
            ELSE GREATEST(0, confidence - 0.1)
          END,
          updated_at = NOW()
      WHERE trajectory_id = $2
      RETURNING *
    `, [success, trajectoryId]);

    return rows[0] ? this._mapRow(rows[0]) : null;
  }

  /**
   * Get top performing trajectories
   */
  async getTopPerformers(options = {}) {
    const { taskType, dogId, limit = 10 } = options;

    let sql = `
      SELECT * FROM trajectories
      WHERE outcome = 'success'
        AND reward > 0
    `;
    const params = [];
    let paramIndex = 1;

    if (taskType) {
      sql += ` AND task_type = $${paramIndex++}`;
      params.push(taskType);
    }

    if (dogId) {
      sql += ` AND dog_id = $${paramIndex++}`;
      params.push(dogId);
    }

    sql += ` ORDER BY reward DESC, confidence DESC LIMIT $${paramIndex}`;
    params.push(limit);

    const { rows } = await this.db.query(sql, params);
    return rows.map(r => this._mapRow(r));
  }

  /**
   * Get statistics
   */
  async getStats(options = {}) {
    const { userId, taskType } = options;

    let where = '1=1';
    const params = [];
    let paramIndex = 1;

    if (userId) {
      where += ` AND user_id = $${paramIndex++}`;
      params.push(userId);
    }

    if (taskType) {
      where += ` AND task_type = $${paramIndex++}`;
      params.push(taskType);
    }

    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE outcome = 'success') as successes,
        COUNT(*) FILTER (WHERE outcome = 'failure') as failures,
        AVG(reward) as avg_reward,
        AVG(duration_ms) as avg_duration,
        AVG(tool_count) as avg_tools,
        AVG(error_count) as avg_errors,
        AVG(switch_count) as avg_switches,
        SUM(replay_count) as total_replays
      FROM trajectories
      WHERE ${where}
    `, params);

    const stats = rows[0];
    return {
      total: parseInt(stats.total),
      successes: parseInt(stats.successes),
      failures: parseInt(stats.failures),
      successRate: stats.total > 0 ? stats.successes / stats.total : 0,
      avgReward: parseFloat(stats.avg_reward) || 0,
      avgDurationMs: parseFloat(stats.avg_duration) || 0,
      avgTools: parseFloat(stats.avg_tools) || 0,
      avgErrors: parseFloat(stats.avg_errors) || 0,
      avgSwitches: parseFloat(stats.avg_switches) || 0,
      totalReplays: parseInt(stats.total_replays) || 0,
    };
  }

  /**
   * Map database row to trajectory object
   */
  _mapRow(row) {
    if (!row) return null;
    return {
      trajectoryId: row.trajectory_id,
      userId: row.user_id,
      sessionId: row.session_id,
      dogId: row.dog_id,
      taskType: row.task_type,
      initialState: row.initial_state,
      actionSequence: row.action_sequence,
      outcome: row.outcome,
      outcomeDetails: row.outcome_details,
      reward: parseFloat(row.reward),
      durationMs: row.duration_ms,
      toolCount: row.tool_count,
      errorCount: row.error_count,
      switchCount: row.switch_count,
      similarityHash: row.similarity_hash,
      replayCount: row.replay_count,
      successAfterReplay: row.success_after_replay,
      confidence: parseFloat(row.confidence),
      tags: row.tags,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      rank: row.rank ? parseFloat(row.rank) : undefined,
    };
  }
}

export default TrajectoriesRepository;
