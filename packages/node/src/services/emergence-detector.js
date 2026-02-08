/**
 * CYNIC Emergence Detector
 *
 * Detects emergent patterns across sessions:
 * - Recurring mistakes (to prevent)
 * - Successful strategies (to replicate)
 * - User preferences (to remember)
 * - Codebase evolution (to understand)
 *
 * "From chaos, patterns emerge" - κυνικός
 *
 * @module @cynic/node/services/emergence-detector
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { createLogger } from '@cynic/core';

const log = createLogger('EmergenceDetector');

/**
 * Pattern categories for emergence
 */
export const PatternCategory = {
  RECURRING_MISTAKE: 'recurring_mistake',
  SUCCESSFUL_STRATEGY: 'successful_strategy',
  USER_PREFERENCE: 'user_preference',
  CODE_EVOLUTION: 'code_evolution',
  WORKFLOW_PATTERN: 'workflow_pattern',
  TIMING_PATTERN: 'timing_pattern',
  DOG_BEHAVIOR: 'dog_behavior',
  CONSENSUS_QUALITY: 'consensus_quality',
  COLLECTIVE_TREND: 'collective_trend',
};

/**
 * Emergence significance levels
 */
export const SignificanceLevel = {
  LOW: 'low',           // 3+ occurrences
  MEDIUM: 'medium',     // 5+ occurrences
  HIGH: 'high',         // 8+ occurrences (Fibonacci)
  CRITICAL: 'critical', // 13+ occurrences (Fibonacci)
};

/**
 * Default configuration
 */
const DEFAULT_CONFIG = {
  // Minimum occurrences for pattern detection
  minOccurrences: 3,
  // Time window for pattern detection (7 days)
  windowMs: 7 * 24 * 60 * 60 * 1000,
  // Minimum confidence for storing as fact
  minConfidence: PHI_INV_2, // 38.2%
  // Analysis interval (1 hour)
  analysisIntervalMs: 60 * 60 * 1000,
  // Significance thresholds (Fibonacci)
  significanceThresholds: {
    [SignificanceLevel.LOW]: 3,
    [SignificanceLevel.MEDIUM]: 5,
    [SignificanceLevel.HIGH]: 8,
    [SignificanceLevel.CRITICAL]: 13,
  },
};

/**
 * Emergence Detector - Cross-session pattern analysis
 */
export class EmergenceDetector extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} options.persistence - Persistence manager with pool and repositories
   * @param {Object} [options.memoryRetriever] - MemoryRetriever for storing facts
   * @param {Object} [options.embedder] - Embedder for semantic analysis
   * @param {Object} [options.config] - Configuration overrides
   */
  constructor(options = {}) {
    super();
    this.persistence = options.persistence;
    this.memoryRetriever = options.memoryRetriever || null;
    this.embedder = options.embedder || null;
    this.config = { ...DEFAULT_CONFIG, ...options.config };

    // State
    this._running = false;
    this._interval = null;
    this._detectedPatterns = new Map(); // patternKey -> pattern
    this._lastAnalysis = null;

    // Statistics
    this.stats = {
      analysisCount: 0,
      patternsDetected: 0,
      factsStored: 0,
      notificationsSent: 0,
      startedAt: null,
    };
  }

  /**
   * Start the emergence detector
   */
  start() {
    if (this._running) return;
    this._running = true;
    this.stats.startedAt = Date.now();

    // Initial analysis
    this._runAnalysis().catch(e => {
      console.error('[EmergenceDetector] Initial analysis failed:', e.message);
    });

    // Schedule periodic analysis
    this._interval = setInterval(() => {
      this._runAnalysis().catch(e => {
        console.error('[EmergenceDetector] Periodic analysis failed:', e.message);
      });
    }, this.config.analysisIntervalMs);

    this.emit('started');
  }

  /**
   * Stop the emergence detector
   */
  stop() {
    if (!this._running) return;
    this._running = false;

    if (this._interval) {
      clearInterval(this._interval);
      this._interval = null;
    }

    this.emit('stopped', this.stats);
  }

  /**
   * Run emergence analysis
   * @private
   */
  async _runAnalysis() {
    if (!this.persistence?.pool) return;

    this._lastAnalysis = Date.now();
    this.stats.analysisCount++;

    try {
      // Analyze different pattern categories in parallel
      const [
        mistakes,
        strategies,
        preferences,
        workflows,
        dataGraves,
      ] = await Promise.all([
        this._analyzeRecurringMistakes(),
        this._analyzeSuccessfulStrategies(),
        this._analyzeUserPreferences(),
        this._analyzeWorkflowPatterns(),
        this._analyzeDataGraves(),
      ]);

      // Process and store significant patterns
      const allPatterns = [...mistakes, ...strategies, ...preferences, ...workflows, ...dataGraves];

      for (const pattern of allPatterns) {
        await this._processPattern(pattern);
      }

      this.emit('analysis_complete', {
        timestamp: Date.now(),
        patternsFound: allPatterns.length,
        byCategory: {
          mistakes: mistakes.length,
          strategies: strategies.length,
          preferences: preferences.length,
          workflows: workflows.length,
          dataGraves: dataGraves.length,
        },
      });
    } catch (e) {
      this.emit('analysis_error', { error: e.message });
    }
  }

  /**
   * Analyze recurring mistakes from lessons_learned
   * @private
   */
  async _analyzeRecurringMistakes() {
    const patterns = [];
    const cutoff = new Date(Date.now() - this.config.windowMs);

    try {
      const { rows } = await this.persistence.pool.query(`
        SELECT
          mistake,
          category,
          COUNT(*) as occurrences,
          MAX(severity) as max_severity,
          array_agg(DISTINCT correction) as corrections
        FROM lessons_learned
        WHERE created_at >= $1
        GROUP BY mistake, category
        HAVING COUNT(*) >= $2
        ORDER BY occurrences DESC
        LIMIT 20
      `, [cutoff, this.config.minOccurrences]);

      for (const row of rows) {
        const significance = this._getSignificance(parseInt(row.occurrences));
        patterns.push({
          category: PatternCategory.RECURRING_MISTAKE,
          key: `mistake:${this._hashString(row.mistake)}`,
          subject: row.mistake.substring(0, 100),
          content: row.mistake,
          occurrences: parseInt(row.occurrences),
          significance,
          severity: row.max_severity,
          corrections: row.corrections,
          confidence: Math.min(PHI_INV, parseInt(row.occurrences) / 20),
        });
      }
    } catch (e) {
      // Table might not exist
    }

    return patterns;
  }

  /**
   * Analyze successful strategies from judgments
   * @private
   */
  async _analyzeSuccessfulStrategies() {
    const patterns = [];
    const cutoff = new Date(Date.now() - this.config.windowMs);

    try {
      const { rows } = await this.persistence.pool.query(`
        SELECT
          item_type,
          verdict,
          COUNT(*) as occurrences,
          AVG(q_score) as avg_score
        FROM judgments
        WHERE created_at >= $1
          AND verdict IN ('HOWL', 'WAG')
          AND q_score >= 70
        GROUP BY item_type, verdict
        HAVING COUNT(*) >= $2
        ORDER BY occurrences DESC
        LIMIT 20
      `, [cutoff, this.config.minOccurrences]);

      for (const row of rows) {
        const significance = this._getSignificance(parseInt(row.occurrences));
        patterns.push({
          category: PatternCategory.SUCCESSFUL_STRATEGY,
          key: `strategy:${row.item_type}:${row.verdict}`,
          subject: `High-quality ${row.item_type} items`,
          content: `${row.item_type} items consistently score well (avg: ${Math.round(row.avg_score)})`,
          occurrences: parseInt(row.occurrences),
          significance,
          avgScore: parseFloat(row.avg_score),
          confidence: Math.min(PHI_INV, parseFloat(row.avg_score) / 100),
        });
      }
    } catch (e) {
      // Table might not exist
    }

    return patterns;
  }

  /**
   * Analyze user preferences from conversation memories
   * @private
   */
  async _analyzeUserPreferences() {
    const patterns = [];
    const cutoff = new Date(Date.now() - this.config.windowMs);

    try {
      const { rows } = await this.persistence.pool.query(`
        SELECT
          memory_type,
          COUNT(*) as occurrences,
          AVG(importance) as avg_importance
        FROM conversation_memories
        WHERE created_at >= $1
          AND memory_type = 'preference'
        GROUP BY memory_type
        HAVING COUNT(*) >= $2
      `, [cutoff, this.config.minOccurrences]);

      for (const row of rows) {
        patterns.push({
          category: PatternCategory.USER_PREFERENCE,
          key: `preference:${row.memory_type}`,
          subject: `User preference patterns`,
          content: `${row.occurrences} preferences recorded`,
          occurrences: parseInt(row.occurrences),
          significance: this._getSignificance(parseInt(row.occurrences)),
          avgImportance: parseFloat(row.avg_importance),
          confidence: Math.min(PHI_INV, parseFloat(row.avg_importance)),
        });
      }
    } catch (e) {
      // Table might not exist
    }

    return patterns;
  }

  /**
   * Analyze workflow patterns from sessions
   * @private
   */
  async _analyzeWorkflowPatterns() {
    const patterns = [];
    const cutoff = new Date(Date.now() - this.config.windowMs);

    try {
      // Analyze session timing patterns
      const { rows } = await this.persistence.pool.query(`
        SELECT
          EXTRACT(HOUR FROM created_at) as hour,
          COUNT(*) as session_count,
          AVG(EXTRACT(EPOCH FROM (last_active_at - created_at)) / 60) as avg_duration_min
        FROM sessions
        WHERE created_at >= $1
          AND last_active_at IS NOT NULL
        GROUP BY EXTRACT(HOUR FROM created_at)
        HAVING COUNT(*) >= $2
        ORDER BY session_count DESC
        LIMIT 10
      `, [cutoff, this.config.minOccurrences]);

      for (const row of rows) {
        const hour = parseInt(row.hour);
        const significance = this._getSignificance(parseInt(row.session_count));
        patterns.push({
          category: PatternCategory.TIMING_PATTERN,
          key: `timing:hour:${hour}`,
          subject: `Peak activity at ${hour}:00`,
          content: `${row.session_count} sessions, avg ${Math.round(row.avg_duration_min)} min`,
          occurrences: parseInt(row.session_count),
          significance,
          hour,
          avgDurationMin: parseFloat(row.avg_duration_min),
          confidence: Math.min(PHI_INV, parseInt(row.session_count) / 50),
        });
      }
    } catch (e) {
      // Table might not exist
    }

    return patterns;
  }

  /**
   * Analyze data grave tables for emergent patterns.
   * Queries dog_events, consensus_votes, collective_snapshots, and tool_usage.
   * @private
   */
  async _analyzeDataGraves() {
    const patterns = [];
    if (!this.persistence?.pool) return patterns;

    try {
      const pool = this.persistence.pool;
      const hours = Math.round(this.config.windowMs / (3600 * 1000));

      // Dog behavioral patterns
      try {
        const { rows } = await pool.query(
          `SELECT dog_name, event_type, COUNT(*) AS count
           FROM dog_events
           WHERE created_at > NOW() - ($1 || ' hours')::INTERVAL
           GROUP BY dog_name, event_type
           HAVING COUNT(*) >= $2
           ORDER BY count DESC LIMIT 10`,
          [String(hours), this.config.minOccurrences]
        );
        for (const row of rows) {
          const count = parseInt(row.count);
          patterns.push({
            category: PatternCategory.DOG_BEHAVIOR,
            key: `dog:${row.dog_name}:${row.event_type}`,
            subject: `${row.dog_name} frequently performs ${row.event_type}`,
            content: `Dog ${row.dog_name} has ${count} ${row.event_type} events in ${hours}h`,
            occurrences: count,
            significance: this._getSignificance(count),
            confidence: Math.min(PHI_INV, count / 50),
          });
        }
      } catch (e) { /* table may not exist */ }

      // Consensus quality patterns
      try {
        const { rows: [summary] } = await pool.query(
          `SELECT COUNT(*) AS total,
                  COUNT(*) FILTER (WHERE approved = true) AS approved,
                  AVG(agreement) AS avg_agreement,
                  COUNT(*) FILTER (WHERE guardian_veto = true) AS vetoes
           FROM consensus_votes
           WHERE created_at > NOW() - ($1 || ' hours')::INTERVAL`,
          [String(hours)]
        );
        if (summary && parseInt(summary.total) >= this.config.minOccurrences) {
          const total = parseInt(summary.total);
          const approvalRate = parseInt(summary.approved) / total;
          const vetoRate = parseInt(summary.vetoes) / total;

          if (approvalRate < PHI_INV_2) {
            patterns.push({
              category: PatternCategory.CONSENSUS_QUALITY,
              key: 'consensus:low_approval',
              subject: 'Consensus approval rate below phi-2',
              content: `Approval rate ${Math.round(approvalRate * 100)}% across ${total} votes`,
              occurrences: total,
              significance: this._getSignificance(total),
              confidence: Math.min(PHI_INV, total / 100),
            });
          }
          if (vetoRate > PHI_INV_2) {
            patterns.push({
              category: PatternCategory.CONSENSUS_QUALITY,
              key: 'consensus:high_veto',
              subject: 'Guardian veto rate above phi-2',
              content: `Veto rate ${Math.round(vetoRate * 100)}% across ${total} votes`,
              occurrences: parseInt(summary.vetoes),
              significance: this._getSignificance(parseInt(summary.vetoes)),
              confidence: Math.min(PHI_INV, vetoRate),
            });
          }
        }
      } catch (e) { /* table may not exist */ }

      // Collective health trends
      try {
        const { rows: [agg] } = await pool.query(
          `SELECT COUNT(*) AS total,
                  AVG(average_health) AS avg_health,
                  AVG(memory_load) AS avg_load
           FROM collective_snapshots
           WHERE created_at > NOW() - ($1 || ' hours')::INTERVAL`,
          [String(hours)]
        );
        if (agg && parseInt(agg.total) >= this.config.minOccurrences) {
          const avgHealth = parseFloat(agg.avg_health) || 0;
          const avgLoad = parseFloat(agg.avg_load) || 0;

          if (avgHealth < PHI_INV_2) {
            patterns.push({
              category: PatternCategory.COLLECTIVE_TREND,
              key: 'collective:degraded_health',
              subject: 'Collective health below phi-2',
              content: `Average health ${Math.round(avgHealth * 100)}% over ${parseInt(agg.total)} snapshots`,
              occurrences: parseInt(agg.total),
              significance: this._getSignificance(parseInt(agg.total)),
              confidence: Math.min(PHI_INV, 1 - avgHealth),
            });
          }
          if (avgLoad > PHI_INV) {
            patterns.push({
              category: PatternCategory.COLLECTIVE_TREND,
              key: 'collective:high_memory_load',
              subject: 'Memory load above phi-1',
              content: `Average memory load ${Math.round(avgLoad * 100)}%`,
              occurrences: parseInt(agg.total),
              significance: this._getSignificance(parseInt(agg.total)),
              confidence: Math.min(PHI_INV, avgLoad),
            });
          }
        }
      } catch (e) { /* table may not exist */ }

      // Tool error hotspots
      try {
        const { rows } = await pool.query(
          `SELECT tool_name, COUNT(*) AS total,
                  COUNT(*) FILTER (WHERE success = false) AS failures
           FROM tool_usage
           WHERE created_at > NOW() - ($1 || ' hours')::INTERVAL
           GROUP BY tool_name
           HAVING COUNT(*) >= $2
           ORDER BY total DESC LIMIT 10`,
          [String(hours), this.config.minOccurrences]
        );
        for (const row of rows) {
          const total = parseInt(row.total);
          const failures = parseInt(row.failures);
          const failRate = failures / total;
          if (failRate > PHI_INV_2) {
            patterns.push({
              category: PatternCategory.RECURRING_MISTAKE,
              key: `tool:failing:${row.tool_name}`,
              subject: `Tool ${row.tool_name} has high failure rate`,
              content: `${Math.round(failRate * 100)}% failure rate across ${total} uses`,
              occurrences: failures,
              significance: this._getSignificance(failures),
              confidence: Math.min(PHI_INV, failRate),
            });
          }
        }
      } catch (e) { /* table may not exist */ }

      if (patterns.length > 0) {
        log.info('Data grave analysis found patterns', { count: patterns.length });
      }
    } catch (e) {
      log.debug('Data grave analysis failed', { error: e.message });
    }

    return patterns;
  }

  /**
   * Process and store a detected pattern
   * @private
   */
  async _processPattern(pattern) {
    const existingPattern = this._detectedPatterns.get(pattern.key);

    // Check if pattern is new or significantly changed
    const isNew = !existingPattern;
    const isEscalated = existingPattern &&
      this._getSignificanceLevel(pattern.significance) >
      this._getSignificanceLevel(existingPattern.significance);

    if (isNew || isEscalated) {
      this._detectedPatterns.set(pattern.key, pattern);
      this.stats.patternsDetected++;

      // Store as fact if memoryRetriever is available
      if (this.memoryRetriever && pattern.confidence >= this.config.minConfidence) {
        try {
          await this.memoryRetriever.rememberFact(null, {
            factType: 'learning',
            subject: `[EMERGENCE] ${pattern.subject}`,
            content: pattern.content,
            context: {
              category: pattern.category,
              occurrences: pattern.occurrences,
              significance: pattern.significance,
              detectedAt: Date.now(),
            },
            confidence: pattern.confidence,
            relevance: pattern.confidence,
            tags: ['emergence', pattern.category, pattern.significance],
          });
          this.stats.factsStored++;
        } catch (e) {
          // Facts storage failed
        }
      }

      // Emit notification for significant patterns
      if (pattern.significance === SignificanceLevel.HIGH ||
          pattern.significance === SignificanceLevel.CRITICAL) {
        this.stats.notificationsSent++;
        this.emit('significant_pattern', {
          pattern,
          isNew,
          isEscalated,
          timestamp: Date.now(),
        });
      }

      this.emit('pattern_detected', { pattern, isNew, isEscalated });
    }
  }

  /**
   * Get significance level based on occurrences
   * @private
   */
  _getSignificance(occurrences) {
    const thresholds = this.config.significanceThresholds;
    if (occurrences >= thresholds[SignificanceLevel.CRITICAL]) {
      return SignificanceLevel.CRITICAL;
    }
    if (occurrences >= thresholds[SignificanceLevel.HIGH]) {
      return SignificanceLevel.HIGH;
    }
    if (occurrences >= thresholds[SignificanceLevel.MEDIUM]) {
      return SignificanceLevel.MEDIUM;
    }
    return SignificanceLevel.LOW;
  }

  /**
   * Get numeric significance level for comparison
   * @private
   */
  _getSignificanceLevel(significance) {
    const levels = {
      [SignificanceLevel.LOW]: 1,
      [SignificanceLevel.MEDIUM]: 2,
      [SignificanceLevel.HIGH]: 3,
      [SignificanceLevel.CRITICAL]: 4,
    };
    return levels[significance] || 0;
  }

  /**
   * Simple string hash
   * @private
   */
  _hashString(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash;
    }
    return Math.abs(hash).toString(36);
  }

  /**
   * Get detected patterns
   */
  getPatterns() {
    return Array.from(this._detectedPatterns.values());
  }

  /**
   * Get patterns by category
   */
  getPatternsByCategory(category) {
    return this.getPatterns().filter(p => p.category === category);
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      running: this._running,
      lastAnalysis: this._lastAnalysis,
      patternCount: this._detectedPatterns.size,
      uptimeMs: this.stats.startedAt ? Date.now() - this.stats.startedAt : 0,
    };
  }

  /**
   * Get status summary
   */
  getStatus() {
    const patterns = this.getPatterns();
    const byCategory = {};
    const bySignificance = {};

    for (const p of patterns) {
      byCategory[p.category] = (byCategory[p.category] || 0) + 1;
      bySignificance[p.significance] = (bySignificance[p.significance] || 0) + 1;
    }

    return {
      running: this._running,
      patternCount: patterns.length,
      byCategory,
      bySignificance,
      stats: this.getStats(),
      critical: patterns.filter(p => p.significance === SignificanceLevel.CRITICAL),
    };
  }
}

/**
 * Create an EmergenceDetector instance
 */
export function createEmergenceDetector(options = {}) {
  return new EmergenceDetector(options);
}

// Singleton
let _detector = null;

/**
 * Get the global EmergenceDetector instance
 */
export function getEmergenceDetector(options) {
  if (!_detector) {
    _detector = createEmergenceDetector(options);
  }
  return _detector;
}

export default {
  EmergenceDetector,
  PatternCategory,
  SignificanceLevel,
  createEmergenceDetector,
  getEmergenceDetector,
};
