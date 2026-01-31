/**
 * SecurityAuditTrail - Persistent security audit logging
 *
 * Records all permission requests/responses and security-relevant events
 * for compliance, learning, and threat detection.
 *
 * P3.3: Complete security audit trail with persistence.
 *
 * "Le chien n'oublie jamais" - κυνικός
 *
 * @module @cynic/node/orchestration/security-audit
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { promises as fs } from 'fs';
import { existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { createHash } from 'crypto';

const log = createLogger('SecurityAudit');

/**
 * Audit event types
 */
export const AuditEventType = {
  PERMISSION_REQUEST: 'permission_request',
  PERMISSION_GRANTED: 'permission_granted',
  PERMISSION_DENIED: 'permission_denied',
  SECURITY_ALERT: 'security_alert',
  BLOCKED_OPERATION: 'blocked_operation',
  SENSITIVE_ACCESS: 'sensitive_access',
  SESSION_START: 'session_start',
  SESSION_END: 'session_end',
  TOOL_EXECUTION: 'tool_execution',
  PATTERN_DETECTED: 'pattern_detected',
};

/**
 * Sensitivity levels
 */
export const SensitivityLevel = {
  LOW: 'low',
  MEDIUM: 'medium',
  HIGH: 'high',
  CRITICAL: 'critical',
};

/**
 * Default configuration
 */
export const AUDIT_CONFIG = {
  enabled: true,
  maxInMemoryEvents: 1000,
  persistToFile: true,
  persistPath: null,           // Auto-detect from ~/.cynic/audit/
  rotateAfterDays: 30,
  hashEvents: true,            // Create tamper-evident hash chain
  sensitivityThreshold: SensitivityLevel.LOW,  // Log all by default
};

/**
 * Single audit event
 */
export class AuditEvent {
  /**
   * @param {Object} options
   */
  constructor(options) {
    this.id = options.id || `audit_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    this.type = options.type || AuditEventType.TOOL_EXECUTION;
    this.timestamp = options.timestamp || Date.now();
    this.sessionId = options.sessionId || null;
    this.userId = options.userId || null;

    // Event details
    this.tool = options.tool || null;
    this.category = options.category || null;
    this.sensitivity = options.sensitivity || SensitivityLevel.MEDIUM;
    this.granted = options.granted;
    this.reason = options.reason || null;

    // Context
    this.input = options.input || null;
    this.output = options.output || null;
    this.metadata = options.metadata || {};

    // Hash chain (for tamper evidence)
    this.previousHash = options.previousHash || null;
    this.hash = null;
  }

  /**
   * Compute hash for this event
   */
  computeHash() {
    const data = JSON.stringify({
      id: this.id,
      type: this.type,
      timestamp: this.timestamp,
      tool: this.tool,
      granted: this.granted,
      previousHash: this.previousHash,
    });

    this.hash = createHash('sha256').update(data).digest('hex');
    return this.hash;
  }

  /**
   * Export to JSON
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      timestamp: this.timestamp,
      sessionId: this.sessionId,
      userId: this.userId,
      tool: this.tool,
      category: this.category,
      sensitivity: this.sensitivity,
      granted: this.granted,
      reason: this.reason,
      input: this.input,
      output: this.output,
      metadata: this.metadata,
      previousHash: this.previousHash,
      hash: this.hash,
    };
  }

  /**
   * Create from JSON
   */
  static fromJSON(json) {
    const event = new AuditEvent(json);
    event.hash = json.hash;
    return event;
  }
}

/**
 * Security Audit Trail
 *
 * Maintains tamper-evident log of all security-relevant events.
 */
export class SecurityAuditTrail {
  /**
   * @param {Object} options
   * @param {Object} [options.config] - Override config
   * @param {string} [options.persistPath] - Path for audit files
   */
  constructor(options = {}) {
    this.config = { ...AUDIT_CONFIG, ...options.config };
    this.persistPath = options.persistPath || this.config.persistPath;

    // In-memory event buffer
    this.events = [];
    this.lastHash = null;

    // Indexes for fast lookup
    this._bySession = new Map();
    this._byUser = new Map();
    this._byTool = new Map();

    // Statistics
    this.stats = {
      totalEvents: 0,
      byType: {},
      bySensitivity: {},
      granted: 0,
      denied: 0,
      blocked: 0,
    };

    // Auto-detect persist path
    if (!this.persistPath && this.config.persistToFile) {
      this.persistPath = this._getDefaultPersistPath();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Event Recording
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a permission request
   *
   * @param {Object} data - Request data
   * @returns {AuditEvent} Created event
   */
  recordPermissionRequest(data) {
    return this.record({
      type: AuditEventType.PERMISSION_REQUEST,
      tool: data.tool || data.toolName,
      category: data.category,
      sensitivity: data.sensitivity || SensitivityLevel.MEDIUM,
      sessionId: data.sessionId,
      userId: data.userId,
      input: data.input || data.toolInput,
      metadata: {
        requestId: data.requestId,
        sensitiveReason: data.sensitiveReason,
      },
    });
  }

  /**
   * Record a permission response (granted/denied)
   *
   * @param {Object} data - Response data
   * @returns {AuditEvent} Created event
   */
  recordPermissionResponse(data) {
    const granted = data.granted !== false;
    return this.record({
      type: granted ? AuditEventType.PERMISSION_GRANTED : AuditEventType.PERMISSION_DENIED,
      tool: data.tool || data.toolName,
      category: data.category,
      sensitivity: data.sensitivity || SensitivityLevel.MEDIUM,
      granted,
      reason: data.reason,
      sessionId: data.sessionId,
      userId: data.userId,
      metadata: {
        requestId: data.requestId,
        responseTime: data.responseTime,
      },
    });
  }

  /**
   * Record a security alert
   *
   * @param {Object} data - Alert data
   * @returns {AuditEvent} Created event
   */
  recordSecurityAlert(data) {
    return this.record({
      type: AuditEventType.SECURITY_ALERT,
      tool: data.tool,
      category: 'security',
      sensitivity: data.severity || SensitivityLevel.HIGH,
      reason: data.message || data.reason,
      sessionId: data.sessionId,
      userId: data.userId,
      metadata: {
        alertType: data.alertType,
        details: data.details,
      },
    });
  }

  /**
   * Record a blocked operation
   *
   * @param {Object} data - Block data
   * @returns {AuditEvent} Created event
   */
  recordBlockedOperation(data) {
    this.stats.blocked++;
    return this.record({
      type: AuditEventType.BLOCKED_OPERATION,
      tool: data.tool,
      category: data.category,
      sensitivity: SensitivityLevel.HIGH,
      granted: false,
      reason: data.reason || 'Blocked by Guardian',
      sessionId: data.sessionId,
      userId: data.userId,
      input: data.input,
      metadata: {
        blockedBy: data.blockedBy || 'Guardian',
        riskScore: data.riskScore,
      },
    });
  }

  /**
   * Record a generic audit event
   *
   * @param {Object} data - Event data
   * @returns {AuditEvent} Created event
   */
  record(data) {
    // Check sensitivity threshold
    const sensitivityOrder = { low: 0, medium: 1, high: 2, critical: 3 };
    const threshold = sensitivityOrder[this.config.sensitivityThreshold] || 0;
    const eventLevel = sensitivityOrder[data.sensitivity] || 1;

    if (eventLevel < threshold) {
      return null; // Below threshold, don't log
    }

    // Create event
    const event = new AuditEvent({
      ...data,
      previousHash: this.lastHash,
    });

    // Compute hash if enabled
    if (this.config.hashEvents) {
      event.computeHash();
      this.lastHash = event.hash;
    }

    // Add to in-memory buffer
    this.events.push(event);

    // Update indexes
    this._indexEvent(event);

    // Update stats
    this._updateStats(event);

    // Trim if too large
    if (this.events.length > this.config.maxInMemoryEvents) {
      this._rotateEvents();
    }

    // Persist if enabled
    if (this.config.persistToFile) {
      this._persistEvent(event).catch(err => {
        log.warn('Failed to persist audit event', { error: err.message });
      });
    }

    this.stats.totalEvents++;
    return event;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Indexing
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Index event for fast lookup
   * @private
   */
  _indexEvent(event) {
    // By session
    if (event.sessionId) {
      if (!this._bySession.has(event.sessionId)) {
        this._bySession.set(event.sessionId, []);
      }
      this._bySession.get(event.sessionId).push(event.id);
    }

    // By user
    if (event.userId) {
      if (!this._byUser.has(event.userId)) {
        this._byUser.set(event.userId, []);
      }
      this._byUser.get(event.userId).push(event.id);
    }

    // By tool
    if (event.tool) {
      if (!this._byTool.has(event.tool)) {
        this._byTool.set(event.tool, []);
      }
      this._byTool.get(event.tool).push(event.id);
    }
  }

  /**
   * Update statistics
   * @private
   */
  _updateStats(event) {
    // By type
    this.stats.byType[event.type] = (this.stats.byType[event.type] || 0) + 1;

    // By sensitivity
    this.stats.bySensitivity[event.sensitivity] =
      (this.stats.bySensitivity[event.sensitivity] || 0) + 1;

    // Granted/denied
    if (event.granted === true) this.stats.granted++;
    if (event.granted === false) this.stats.denied++;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Queries
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Query events with filters
   *
   * @param {Object} filters - Query filters
   * @returns {AuditEvent[]} Matching events
   */
  query(filters = {}) {
    let results = [...this.events];

    // Filter by type
    if (filters.type) {
      results = results.filter(e => e.type === filters.type);
    }

    // Filter by sensitivity
    if (filters.sensitivity) {
      results = results.filter(e => e.sensitivity === filters.sensitivity);
    }

    // Filter by session
    if (filters.sessionId) {
      results = results.filter(e => e.sessionId === filters.sessionId);
    }

    // Filter by user
    if (filters.userId) {
      results = results.filter(e => e.userId === filters.userId);
    }

    // Filter by tool
    if (filters.tool) {
      results = results.filter(e => e.tool === filters.tool);
    }

    // Filter by granted status
    if (filters.granted !== undefined) {
      results = results.filter(e => e.granted === filters.granted);
    }

    // Filter by time range
    if (filters.since) {
      const since = typeof filters.since === 'number' ? filters.since : new Date(filters.since).getTime();
      results = results.filter(e => e.timestamp >= since);
    }

    if (filters.until) {
      const until = typeof filters.until === 'number' ? filters.until : new Date(filters.until).getTime();
      results = results.filter(e => e.timestamp <= until);
    }

    // Limit results
    if (filters.limit) {
      results = results.slice(-filters.limit);
    }

    return results;
  }

  /**
   * Get events for a session
   *
   * @param {string} sessionId - Session ID
   * @returns {AuditEvent[]}
   */
  getBySession(sessionId) {
    const eventIds = this._bySession.get(sessionId) || [];
    return eventIds.map(id => this.events.find(e => e.id === id)).filter(Boolean);
  }

  /**
   * Get events for a user
   *
   * @param {string} userId - User ID
   * @returns {AuditEvent[]}
   */
  getByUser(userId) {
    const eventIds = this._byUser.get(userId) || [];
    return eventIds.map(id => this.events.find(e => e.id === id)).filter(Boolean);
  }

  /**
   * Get events for a tool
   *
   * @param {string} tool - Tool name
   * @returns {AuditEvent[]}
   */
  getByTool(tool) {
    const eventIds = this._byTool.get(tool) || [];
    return eventIds.map(id => this.events.find(e => e.id === id)).filter(Boolean);
  }

  /**
   * Get denied permissions
   *
   * @param {Object} [options] - Query options
   * @returns {AuditEvent[]}
   */
  getDenied(options = {}) {
    return this.query({
      ...options,
      granted: false,
    });
  }

  /**
   * Get security alerts
   *
   * @param {Object} [options] - Query options
   * @returns {AuditEvent[]}
   */
  getAlerts(options = {}) {
    return this.query({
      ...options,
      type: AuditEventType.SECURITY_ALERT,
    });
  }

  /**
   * Get high-sensitivity events
   *
   * @param {Object} [options] - Query options
   * @returns {AuditEvent[]}
   */
  getHighSensitivity(options = {}) {
    return [
      ...this.query({ ...options, sensitivity: SensitivityLevel.HIGH }),
      ...this.query({ ...options, sensitivity: SensitivityLevel.CRITICAL }),
    ].sort((a, b) => b.timestamp - a.timestamp);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Persistence
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get default persist path
   * @private
   */
  _getDefaultPersistPath() {
    const home = process.env.HOME || process.env.USERPROFILE || '.';
    return resolve(home, '.cynic', 'audit');
  }

  /**
   * Persist event to file
   * @private
   */
  async _persistEvent(event) {
    if (!this.persistPath) return;

    try {
      // Ensure directory exists
      await fs.mkdir(this.persistPath, { recursive: true });

      // Get today's log file
      const date = new Date(event.timestamp).toISOString().split('T')[0];
      const logFile = resolve(this.persistPath, `audit-${date}.jsonl`);

      // Append event as JSON line
      const line = JSON.stringify(event.toJSON()) + '\n';
      await fs.appendFile(logFile, line);

    } catch (err) {
      log.error('Persist failed', { error: err.message });
    }
  }

  /**
   * Rotate old events out of memory
   * @private
   */
  _rotateEvents() {
    // Keep only the most recent half
    const keepCount = Math.floor(this.config.maxInMemoryEvents / 2);
    this.events = this.events.slice(-keepCount);

    log.debug('Events rotated', { kept: keepCount });
  }

  /**
   * Load events from file
   *
   * @param {string} date - Date string (YYYY-MM-DD)
   * @returns {Promise<AuditEvent[]>}
   */
  async loadFromFile(date) {
    if (!this.persistPath) return [];

    const logFile = resolve(this.persistPath, `audit-${date}.jsonl`);
    if (!existsSync(logFile)) return [];

    try {
      const content = await fs.readFile(logFile, 'utf-8');
      const events = content
        .split('\n')
        .filter(line => line.trim())
        .map(line => AuditEvent.fromJSON(JSON.parse(line)));

      return events;
    } catch (err) {
      log.error('Load failed', { error: err.message });
      return [];
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Verification
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Verify hash chain integrity
   *
   * @returns {Object} Verification result
   */
  verifyIntegrity() {
    if (!this.config.hashEvents || this.events.length === 0) {
      return { valid: true, checked: 0 };
    }

    let previousHash = null;
    let valid = true;
    let firstInvalid = null;

    for (let i = 0; i < this.events.length; i++) {
      const event = this.events[i];

      // Check previous hash matches
      if (event.previousHash !== previousHash) {
        valid = false;
        firstInvalid = i;
        break;
      }

      // Recompute hash
      const originalHash = event.hash;
      event.computeHash();

      if (event.hash !== originalHash) {
        valid = false;
        firstInvalid = i;
        event.hash = originalHash; // Restore
        break;
      }

      previousHash = event.hash;
    }

    return {
      valid,
      checked: this.events.length,
      firstInvalid,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export & Statistics
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      inMemory: this.events.length,
      sessions: this._bySession.size,
      users: this._byUser.size,
      tools: this._byTool.size,
    };
  }

  /**
   * Export audit log
   *
   * @param {Object} [options] - Export options
   * @returns {Object} Exported data
   */
  export(options = {}) {
    const events = options.filters
      ? this.query(options.filters)
      : this.events;

    return {
      version: '1.0.0',
      exported: new Date().toISOString(),
      integrity: this.verifyIntegrity(),
      stats: this.getStats(),
      events: events.map(e => e.toJSON()),
    };
  }

  /**
   * Generate compliance report
   *
   * @param {Object} [options] - Report options
   * @returns {Object} Report
   */
  generateReport(options = {}) {
    const since = options.since || Date.now() - 24 * 60 * 60 * 1000; // Last 24h
    const events = this.query({ since });

    // Analyze patterns
    const denials = events.filter(e => e.granted === false);
    const alerts = events.filter(e => e.type === AuditEventType.SECURITY_ALERT);
    const highSensitivity = events.filter(e =>
      e.sensitivity === SensitivityLevel.HIGH ||
      e.sensitivity === SensitivityLevel.CRITICAL
    );

    // Tool usage breakdown
    const toolUsage = {};
    for (const event of events) {
      if (event.tool) {
        toolUsage[event.tool] = (toolUsage[event.tool] || 0) + 1;
      }
    }

    return {
      period: {
        start: new Date(since).toISOString(),
        end: new Date().toISOString(),
      },
      summary: {
        totalEvents: events.length,
        denials: denials.length,
        alerts: alerts.length,
        highSensitivity: highSensitivity.length,
        grantRate: events.length > 0
          ? ((events.length - denials.length) / events.length * 100).toFixed(1) + '%'
          : 'N/A',
      },
      topDeniedTools: this._getTopItems(denials.map(e => e.tool).filter(Boolean), 5),
      topTools: this._getTopItems(Object.entries(toolUsage).map(([tool, count]) => ({ tool, count })), 10),
      recentAlerts: alerts.slice(-5).map(e => ({
        timestamp: new Date(e.timestamp).toISOString(),
        reason: e.reason,
        tool: e.tool,
      })),
      integrity: this.verifyIntegrity(),
    };
  }

  /**
   * Get top items by count
   * @private
   */
  _getTopItems(items, limit) {
    const counts = {};
    for (const item of items) {
      const key = typeof item === 'string' ? item : item.tool;
      counts[key] = (counts[key] || 0) + (item.count || 1);
    }

    return Object.entries(counts)
      .sort((a, b) => b[1] - a[1])
      .slice(0, limit)
      .map(([name, count]) => ({ name, count }));
  }
}

/**
 * Create SecurityAuditTrail instance
 *
 * @param {Object} options - Options
 * @returns {SecurityAuditTrail}
 */
export function createSecurityAuditTrail(options = {}) {
  return new SecurityAuditTrail(options);
}

// Singleton
let _instance = null;

/**
 * Get or create global SecurityAuditTrail
 *
 * @param {Object} [options] - Options
 * @returns {SecurityAuditTrail}
 */
export function getSecurityAuditTrail(options = {}) {
  if (!_instance) {
    _instance = createSecurityAuditTrail(options);
  }
  return _instance;
}

export default SecurityAuditTrail;
