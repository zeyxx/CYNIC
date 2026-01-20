/**
 * Trigger Repository
 *
 * Auto-judgment triggers for CYNIC's watchdog system.
 * Triggers are the eyes that never sleep - watching for patterns worth judging.
 *
 * Tables:
 * - triggers_registry: Trigger definitions
 * - trigger_executions: Execution history
 * - trigger_events: Recent events for pattern matching
 *
 * @module @cynic/persistence/repositories/triggers
 */

'use strict';

import { getPool } from '../client.js';

export class TriggerRepository {
  constructor(db = null) {
    this.db = db || getPool();
  }

  // ===========================================================================
  // TRIGGER REGISTRY METHODS
  // ===========================================================================

  /**
   * Create a new trigger
   */
  async create(trigger) {
    const { rows } = await this.db.query(`
      INSERT INTO triggers_registry (
        trigger_id, name, description, trigger_type,
        condition, action, action_config,
        enabled, priority, rate_limit, cooldown_ms, created_by
      ) VALUES (
        COALESCE($1, generate_trigger_id()), $2, $3, $4,
        $5, $6, $7,
        COALESCE($8, TRUE), COALESCE($9, 50), COALESCE($10, 5), COALESCE($11, 10946), $12
      )
      RETURNING *
    `, [
      trigger.triggerId || null,
      trigger.name,
      trigger.description || null,
      trigger.triggerType || trigger.type,
      JSON.stringify(trigger.condition || {}),
      trigger.action,
      JSON.stringify(trigger.actionConfig || trigger.config || {}),
      trigger.enabled,
      trigger.priority,
      trigger.rateLimit,
      trigger.cooldownMs,
      trigger.createdBy || null,
    ]);
    return this._mapTrigger(rows[0]);
  }

  /**
   * Find trigger by ID
   */
  async findById(triggerId) {
    const { rows } = await this.db.query(
      'SELECT * FROM triggers_registry WHERE trigger_id = $1',
      [triggerId]
    );
    return rows[0] ? this._mapTrigger(rows[0]) : null;
  }

  /**
   * Find all enabled triggers
   */
  async findEnabled() {
    const { rows } = await this.db.query(`
      SELECT * FROM triggers_registry
      WHERE enabled = TRUE
      ORDER BY priority DESC, created_at ASC
    `);
    return rows.map(r => this._mapTrigger(r));
  }

  /**
   * Find all triggers
   */
  async findAll() {
    const { rows } = await this.db.query(`
      SELECT * FROM triggers_registry
      ORDER BY priority DESC, created_at ASC
    `);
    return rows.map(r => this._mapTrigger(r));
  }

  /**
   * Find triggers by type
   */
  async findByType(triggerType) {
    const { rows } = await this.db.query(`
      SELECT * FROM triggers_registry
      WHERE trigger_type = $1 AND enabled = TRUE
      ORDER BY priority DESC
    `, [triggerType]);
    return rows.map(r => this._mapTrigger(r));
  }

  /**
   * Find triggers by action
   */
  async findByAction(action) {
    const { rows } = await this.db.query(`
      SELECT * FROM triggers_registry
      WHERE action = $1 AND enabled = TRUE
      ORDER BY priority DESC
    `, [action]);
    return rows.map(r => this._mapTrigger(r));
  }

  /**
   * Update a trigger
   */
  async update(triggerId, updates) {
    const setClauses = [];
    const values = [triggerId];
    let paramIndex = 2;

    if (updates.name !== undefined) {
      setClauses.push(`name = $${paramIndex++}`);
      values.push(updates.name);
    }
    if (updates.description !== undefined) {
      setClauses.push(`description = $${paramIndex++}`);
      values.push(updates.description);
    }
    if (updates.condition !== undefined) {
      setClauses.push(`condition = $${paramIndex++}`);
      values.push(JSON.stringify(updates.condition));
    }
    if (updates.action !== undefined) {
      setClauses.push(`action = $${paramIndex++}`);
      values.push(updates.action);
    }
    if (updates.actionConfig !== undefined) {
      setClauses.push(`action_config = $${paramIndex++}`);
      values.push(JSON.stringify(updates.actionConfig));
    }
    if (updates.enabled !== undefined) {
      setClauses.push(`enabled = $${paramIndex++}`);
      values.push(updates.enabled);
    }
    if (updates.priority !== undefined) {
      setClauses.push(`priority = $${paramIndex++}`);
      values.push(updates.priority);
    }
    if (updates.rateLimit !== undefined) {
      setClauses.push(`rate_limit = $${paramIndex++}`);
      values.push(updates.rateLimit);
    }
    if (updates.cooldownMs !== undefined) {
      setClauses.push(`cooldown_ms = $${paramIndex++}`);
      values.push(updates.cooldownMs);
    }

    if (setClauses.length === 0) {
      return this.findById(triggerId);
    }

    const { rows } = await this.db.query(`
      UPDATE triggers_registry
      SET ${setClauses.join(', ')}, updated_at = NOW()
      WHERE trigger_id = $1
      RETURNING *
    `, values);

    return rows[0] ? this._mapTrigger(rows[0]) : null;
  }

  /**
   * Enable a trigger
   */
  async enable(triggerId) {
    return this.update(triggerId, { enabled: true });
  }

  /**
   * Disable a trigger
   */
  async disable(triggerId) {
    return this.update(triggerId, { enabled: false });
  }

  /**
   * Delete a trigger
   */
  async delete(triggerId) {
    const { rowCount } = await this.db.query(
      'DELETE FROM triggers_registry WHERE trigger_id = $1',
      [triggerId]
    );
    return rowCount > 0;
  }

  // ===========================================================================
  // TRIGGER EXECUTION METHODS
  // ===========================================================================

  /**
   * Record a trigger execution
   */
  async recordExecution(execution) {
    const { rows } = await this.db.query(`
      INSERT INTO trigger_executions (
        execution_id, trigger_id, session_id,
        event_type, event_data,
        action_taken, action_result, judgment_id,
        status, error_message, duration_ms
      ) VALUES (
        COALESCE($1, generate_execution_id()), $2, $3,
        $4, $5,
        $6, $7, $8,
        COALESCE($9, 'completed'), $10, $11
      )
      RETURNING *
    `, [
      execution.executionId || null,
      execution.triggerId,
      execution.sessionId || null,
      execution.eventType || null,
      JSON.stringify(execution.eventData || {}),
      execution.actionTaken,
      JSON.stringify(execution.actionResult || {}),
      execution.judgmentId || null,
      execution.status,
      execution.errorMessage || null,
      execution.durationMs || null,
    ]);
    return this._mapExecution(rows[0]);
  }

  /**
   * Find recent executions for a trigger
   */
  async findExecutions(triggerId, limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM trigger_executions
      WHERE trigger_id = $1
      ORDER BY executed_at DESC
      LIMIT $2
    `, [triggerId, limit]);
    return rows.map(r => this._mapExecution(r));
  }

  /**
   * Find executions in time window (for rate limiting)
   */
  async findExecutionsInWindow(triggerId, windowMs) {
    const { rows } = await this.db.query(`
      SELECT * FROM trigger_executions
      WHERE trigger_id = $1
        AND executed_at > NOW() - ($2 || ' milliseconds')::INTERVAL
      ORDER BY executed_at DESC
    `, [triggerId, windowMs]);
    return rows.map(r => this._mapExecution(r));
  }

  /**
   * Count executions in last minute (for rate limiting)
   */
  async countExecutionsLastMinute(triggerId) {
    const { rows } = await this.db.query(`
      SELECT COUNT(*) as count
      FROM trigger_executions
      WHERE trigger_id = $1
        AND executed_at > NOW() - INTERVAL '1 minute'
    `, [triggerId]);
    return parseInt(rows[0].count);
  }

  // ===========================================================================
  // TRIGGER EVENTS METHODS
  // ===========================================================================

  /**
   * Store an event for pattern matching
   */
  async storeEvent(event) {
    const { rows } = await this.db.query(`
      INSERT INTO trigger_events (
        event_id, event_type, event_data,
        source, session_id
      ) VALUES (
        COALESCE($1, generate_event_id()), $2, $3, $4, $5
      )
      RETURNING *
    `, [
      event.eventId || null,
      event.eventType || event.type,
      JSON.stringify(event.eventData || event.data || {}),
      event.source || null,
      event.sessionId || null,
    ]);
    return this._mapEvent(rows[0]);
  }

  /**
   * Find unprocessed events
   */
  async findUnprocessedEvents(limit = 100) {
    const { rows } = await this.db.query(`
      SELECT * FROM trigger_events
      WHERE processed = FALSE
        AND expires_at > NOW()
      ORDER BY created_at ASC
      LIMIT $1
    `, [limit]);
    return rows.map(r => this._mapEvent(r));
  }

  /**
   * Mark event as processed
   */
  async markEventProcessed(eventId, matchedTriggers = []) {
    const { rows } = await this.db.query(`
      UPDATE trigger_events
      SET processed = TRUE, matched_triggers = $2
      WHERE event_id = $1
      RETURNING *
    `, [eventId, matchedTriggers]);
    return rows[0] ? this._mapEvent(rows[0]) : null;
  }

  /**
   * Find recent events by type
   */
  async findEventsByType(eventType, limit = 50) {
    const { rows } = await this.db.query(`
      SELECT * FROM trigger_events
      WHERE event_type = $1
        AND expires_at > NOW()
      ORDER BY created_at DESC
      LIMIT $2
    `, [eventType, limit]);
    return rows.map(r => this._mapEvent(r));
  }

  /**
   * Cleanup expired events
   */
  async cleanupExpiredEvents() {
    const { rows } = await this.db.query(`
      SELECT cleanup_expired_trigger_events() as deleted_count
    `);
    return parseInt(rows[0].deleted_count);
  }

  // ===========================================================================
  // STATISTICS & VIEWS
  // ===========================================================================

  /**
   * Get trigger statistics
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_triggers,
        COUNT(*) FILTER (WHERE enabled = TRUE) as enabled_triggers,
        COUNT(*) FILTER (WHERE trigger_type = 'event') as event_triggers,
        COUNT(*) FILTER (WHERE trigger_type = 'periodic') as periodic_triggers,
        COUNT(*) FILTER (WHERE trigger_type = 'pattern') as pattern_triggers,
        COUNT(*) FILTER (WHERE trigger_type = 'threshold') as threshold_triggers,
        COUNT(*) FILTER (WHERE action = 'judge') as judge_actions,
        COUNT(*) FILTER (WHERE action = 'alert') as alert_actions,
        SUM(activation_count) as total_activations
      FROM triggers_registry
    `);

    const stats = rows[0];
    return {
      totalTriggers: parseInt(stats.total_triggers),
      enabledTriggers: parseInt(stats.enabled_triggers),
      byType: {
        event: parseInt(stats.event_triggers),
        periodic: parseInt(stats.periodic_triggers),
        pattern: parseInt(stats.pattern_triggers),
        threshold: parseInt(stats.threshold_triggers),
      },
      byAction: {
        judge: parseInt(stats.judge_actions),
        alert: parseInt(stats.alert_actions),
      },
      totalActivations: parseInt(stats.total_activations) || 0,
    };
  }

  /**
   * Get execution statistics
   */
  async getExecutionStats(hours = 24) {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total,
        COUNT(*) FILTER (WHERE status = 'completed') as completed,
        COUNT(*) FILTER (WHERE status = 'failed') as failed,
        COUNT(*) FILTER (WHERE status = 'skipped') as skipped,
        COUNT(*) FILTER (WHERE status = 'rate_limited') as rate_limited,
        AVG(duration_ms) FILTER (WHERE duration_ms IS NOT NULL) as avg_duration
      FROM trigger_executions
      WHERE executed_at > NOW() - ($1 || ' hours')::INTERVAL
    `, [hours]);

    const stats = rows[0];
    return {
      total: parseInt(stats.total),
      completed: parseInt(stats.completed),
      failed: parseInt(stats.failed),
      skipped: parseInt(stats.skipped),
      rateLimited: parseInt(stats.rate_limited),
      avgDurationMs: parseFloat(stats.avg_duration) || 0,
    };
  }

  /**
   * Get active triggers summary (uses the view)
   */
  async getActiveSummary() {
    const { rows } = await this.db.query(`
      SELECT * FROM active_triggers_summary
    `);
    return rows.map(r => ({
      triggerId: r.trigger_id,
      name: r.name,
      triggerType: r.trigger_type,
      action: r.action,
      enabled: r.enabled,
      priority: r.priority,
      activationCount: parseInt(r.activation_count),
      lastActivatedAt: r.last_activated_at,
      executionsLastHour: parseInt(r.executions_last_hour),
      failuresLastHour: parseInt(r.failures_last_hour),
    }));
  }

  // ===========================================================================
  // PRIVATE HELPERS
  // ===========================================================================

  _mapTrigger(row) {
    if (!row) return null;
    return {
      id: row.id,
      triggerId: row.trigger_id,
      name: row.name,
      description: row.description,
      triggerType: row.trigger_type,
      condition: row.condition,
      action: row.action,
      actionConfig: row.action_config,
      enabled: row.enabled,
      priority: row.priority,
      rateLimit: row.rate_limit,
      cooldownMs: row.cooldown_ms,
      activationCount: row.activation_count,
      lastActivatedAt: row.last_activated_at,
      lastError: row.last_error,
      createdBy: row.created_by,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    };
  }

  _mapExecution(row) {
    if (!row) return null;
    return {
      id: row.id,
      executionId: row.execution_id,
      triggerId: row.trigger_id,
      sessionId: row.session_id,
      eventType: row.event_type,
      eventData: row.event_data,
      actionTaken: row.action_taken,
      actionResult: row.action_result,
      judgmentId: row.judgment_id,
      status: row.status,
      errorMessage: row.error_message,
      durationMs: row.duration_ms,
      executedAt: row.executed_at,
    };
  }

  _mapEvent(row) {
    if (!row) return null;
    return {
      id: row.id,
      eventId: row.event_id,
      eventType: row.event_type,
      eventData: row.event_data,
      source: row.source,
      sessionId: row.session_id,
      processed: row.processed,
      matchedTriggers: row.matched_triggers,
      createdAt: row.created_at,
      expiresAt: row.expires_at,
    };
  }
}

export default TriggerRepository;
