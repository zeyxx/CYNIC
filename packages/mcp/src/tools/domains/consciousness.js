/**
 * Consciousness Domain Tools
 *
 * Tools for emergence and self-awareness:
 * - Emergence: Consciousness detection
 * - SelfMod: Self-modification tracking
 * - Milestone: Development milestones
 * - Patterns: Pattern recognition
 *
 * @module @cynic/mcp/tools/domains/consciousness
 */

'use strict';

import {
  PHI_INV,
  EMERGENCE,
  THRESHOLDS,
  createLogger,
} from '@cynic/core';
import { SIGNIFICANCE_THRESHOLDS } from '@cynic/emergence';

const log = createLogger('ConsciousnessTools');

/**
 * Create patterns tool definition
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} persistence - PersistenceManager instance (optional)
 * @param {Object} patternDetector - PatternDetector instance from @cynic/emergence (optional)
 * @returns {Object} Tool definition
 */
export function createPatternsTool(judge, persistence = null, patternDetector = null) {
  return {
    name: 'brain_patterns',
    description: 'Detect and list patterns from CYNIC observations. Actions: observe (feed data), detect (find patterns), list (show patterns), record (upsert to DB).',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['list', 'observe', 'detect', 'record', 'fix_confidence', 'stats'],
          description: 'Action: list (DB patterns), observe (feed PatternDetector), detect (run detection), record (upsert to DB), stats (detector statistics)',
        },
        category: { type: 'string', enum: ['anomaly', 'verdict', 'dimension', 'sequence', 'trend', 'cluster', 'cycle', 'all'], description: 'Filter by category' },
        limit: { type: 'number', description: 'Maximum patterns (default 10)' },
        // For 'observe' action
        type: { type: 'string', description: 'Data point type (e.g., JUDGMENT, TOOL_USE)' },
        value: { type: 'number', description: 'Numeric value for the observation' },
        verdict: { type: 'string', description: 'Verdict for judgment observations' },
        // For 'record' action
        pattern: { type: 'object', description: 'Pattern object to record to DB' },
      },
    },
    handler: async (params) => {
      const { action = 'list', category = 'all', limit = 10 } = params;

      // Fix low confidence patterns (below φ⁻¹)
      if (action === 'fix_confidence') {
        if (!persistence?.query) {
          return { error: 'No persistence layer available', timestamp: Date.now() };
        }
        try {
          // DB uses DECIMAL(5,4) so 0.618033 gets stored as 0.6180
          // Use 0.6181 to ensure >= PHI_INV after DECIMAL rounding
          const minConfidence = 0.6181;
          const result = await persistence.query(`
            UPDATE patterns
            SET confidence = $1, updated_at = NOW()
            WHERE confidence < $1
            RETURNING pattern_id, category, name, confidence
          `, [minConfidence]);
          return {
            action: 'fix_confidence',
            updated: result.rows?.length || 0,
            patterns: result.rows || [],
            newConfidence: minConfidence,
            timestamp: Date.now(),
          };
        } catch (e) {
          return { error: e.message, timestamp: Date.now() };
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // OBSERVE: Feed data to PatternDetector (statistical pattern recognition)
      // "Le chien observe" - κυνικός
      // ═══════════════════════════════════════════════════════════════════════════
      if (action === 'observe') {
        if (!patternDetector) {
          return { error: 'PatternDetector not available', timestamp: Date.now() };
        }
        const { type = 'UNKNOWN', value, verdict, ...rest } = params;
        const dataPoint = {
          type,
          value: typeof value === 'number' ? value : undefined,
          verdict,
          timestamp: Date.now(),
          ...rest,
        };
        // observe() returns any immediately detected patterns (e.g., anomalies)
        const immediatePatterns = patternDetector.observe(dataPoint);
        return {
          action: 'observe',
          observed: true,
          dataPoint: { type, value, verdict },
          immediatePatterns: immediatePatterns || [],
          bufferSize: patternDetector.dataPoints?.length || 0,
          timestamp: Date.now(),
        };
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // DETECT: Run full pattern detection (sequences, anomalies, trends, etc.)
      // "Le chien détecte" - κυνικός
      // ═══════════════════════════════════════════════════════════════════════════
      if (action === 'detect') {
        if (!patternDetector) {
          return { error: 'PatternDetector not available', timestamp: Date.now() };
        }
        // detect() runs all detection algorithms and returns found patterns
        const detected = patternDetector.detect();
        // Filter by category if specified
        const filtered = category === 'all'
          ? detected
          : detected.filter(p => p.type?.toLowerCase() === category);
        return {
          action: 'detect',
          detected: filtered.slice(0, limit),
          total: filtered.length,
          categories: {
            SEQUENCE: detected.filter(p => p.type === 'SEQUENCE').length,
            ANOMALY: detected.filter(p => p.type === 'ANOMALY').length,
            TREND: detected.filter(p => p.type === 'TREND').length,
            CLUSTER: detected.filter(p => p.type === 'CLUSTER').length,
            CYCLE: detected.filter(p => p.type === 'CYCLE').length,
          },
          timestamp: Date.now(),
        };
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // STATS: Get PatternDetector statistics
      // ═══════════════════════════════════════════════════════════════════════════
      if (action === 'stats') {
        if (!patternDetector) {
          return { error: 'PatternDetector not available', timestamp: Date.now() };
        }
        const stats = patternDetector.getStats();
        const topPatterns = patternDetector.getTopPatterns(5);
        return {
          action: 'stats',
          stats,
          topPatterns,
          timestamp: Date.now(),
        };
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // RECORD: Upsert pattern to persistence
      // ═══════════════════════════════════════════════════════════════════════════
      if (action === 'record') {
        const { pattern } = params;
        if (!pattern) {
          return { error: 'pattern object required', timestamp: Date.now() };
        }
        if (!persistence?.upsertPattern) {
          return { error: 'Persistence not available', timestamp: Date.now() };
        }
        try {
          const result = await persistence.upsertPattern(pattern);
          return {
            action: 'record',
            success: true,
            patternId: result?.pattern_id || pattern.name,
            timestamp: Date.now(),
          };
        } catch (e) {
          return { error: e.message, timestamp: Date.now() };
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // LIST (default): Get patterns from DB + PatternDetector memory
      // ═══════════════════════════════════════════════════════════════════════════
      const patterns = [];

      // Get patterns from PatternDetector (statistical patterns)
      if (patternDetector) {
        const detectorPatterns = patternDetector.getPatterns(
          category === 'all' ? null : category.toUpperCase(),
          SIGNIFICANCE_THRESHOLDS?.NOTABLE || 0.382
        );
        for (const p of detectorPatterns.slice(0, limit)) {
          patterns.push({
            category: p.type?.toLowerCase() || 'unknown',
            id: p.id,
            significance: p.significance,
            confidence: p.confidence,
            occurrences: p.occurrences,
            firstSeen: p.firstSeen,
            lastSeen: p.lastSeen,
            data: p.data,
            source: 'detector',
          });
        }
      }

      // Get patterns from persistence (PostgreSQL) first
      if (persistence?.patterns && (category === 'all' || category !== 'anomaly' && category !== 'verdict')) {
        try {
          const dbPatterns = await persistence.getPatterns({ category: category !== 'all' ? category : undefined, limit });
          for (const p of dbPatterns) {
            patterns.push({
              category: p.category,
              id: p.pattern_id,
              name: p.name,
              description: p.description,
              confidence: p.confidence,
              frequency: p.frequency,
            });
          }
        } catch (e) {
          log.error('Error getting patterns', { error: e.message });
        }
      }

      // Get anomaly candidates from judge
      if (category === 'all' || category === 'anomaly') {
        const anomalies = judge.getAnomalyCandidates?.() || [];
        for (const a of anomalies.slice(0, limit)) {
          patterns.push({
            category: 'anomaly',
            residual: a.residual,
            itemType: a.item?.type,
            timestamp: a.timestamp,
          });
        }
      }

      // Get verdict distribution
      if (category === 'all' || category === 'verdict') {
        // Use persistence stats if available
        if (persistence?.judgments) {
          try {
            const stats = await persistence.getJudgmentStats();
            patterns.push({
              category: 'verdict',
              distribution: stats.verdicts,
              total: stats.total,
              avgScore: stats.avgScore,
              source: 'persistence',
            });
          } catch (e) {
            // Fall back to judge stats
          }
        }

        // Fallback to judge stats
        const stats = judge.getStats();
        if (stats.verdicts) {
          patterns.push({
            category: 'verdict',
            distribution: stats.verdicts,
            total: stats.totalJudgments,
            avgScore: stats.avgScore,
            source: 'memory',
          });
        }
      }

      return {
        category,
        patterns: patterns.slice(0, limit),
        total: patterns.length,
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Create milestone history tool for singularity index tracking
 * @param {Object} [persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createMilestoneHistoryTool(persistence = null) {
  // In-memory cache when no persistence
  const memoryHistory = [];

  return {
    name: 'brain_milestone_history',
    description: 'Track historical Singularity Index scores. Store daily scores and retrieve history for trend analysis.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['get', 'store', 'stats'],
          description: 'Action: get (retrieve history), store (save score), stats (get statistics)',
        },
        days: {
          type: 'number',
          description: 'Number of days of history to retrieve (default: 30)',
          default: 30,
        },
        score: {
          type: 'number',
          description: 'Singularity index score to store (0-100)',
        },
        dimensions: {
          type: 'object',
          description: 'Dimension scores: { codebase, collective, wisdom, autonomy }',
        },
      },
    },
    handler: async (params) => {
      const { action = 'get', days = 30, score, dimensions } = params;

      switch (action) {
        case 'store': {
          if (score === undefined) {
            throw new Error('score is required for store action');
          }
          const entry = {
            timestamp: Date.now(),
            score: Math.min(100, Math.max(0, score)),
            dimensions: dimensions || null,
          };

          if (persistence) {
            try {
              await persistence.query(
                `INSERT INTO singularity_history (score, dimensions, created_at)
                 VALUES ($1, $2, NOW())`,
                [entry.score, JSON.stringify(entry.dimensions)]
              );
            } catch (e) {
              log.error('Error storing milestone', { error: e.message });
              memoryHistory.push(entry);
            }
          } else {
            memoryHistory.push(entry);
          }
          return { stored: true, entry };
        }

        case 'get': {
          let history = [];

          if (persistence) {
            try {
              const result = await persistence.query(
                `SELECT score, dimensions, created_at as timestamp
                 FROM singularity_history
                 WHERE created_at >= NOW() - INTERVAL '${days} days'
                 ORDER BY created_at ASC`
              );
              history = result?.rows?.map(r => ({
                timestamp: new Date(r.timestamp).getTime(),
                score: r.score,
                dimensions: r.dimensions,
              })) || [];
            } catch (e) {
              // Table might not exist, use memory
              history = memoryHistory.slice(-days);
            }
          } else {
            history = memoryHistory.slice(-days);
          }

          return {
            history,
            count: history.length,
            days,
          };
        }

        case 'stats': {
          let history = [];
          if (persistence) {
            try {
              const result = await persistence.query(
                `SELECT score, created_at as timestamp
                 FROM singularity_history
                 ORDER BY created_at DESC
                 LIMIT 100`
              );
              history = result?.rows?.map(r => r.score) || [];
            } catch (e) {
              history = memoryHistory.map(h => h.score);
            }
          } else {
            history = memoryHistory.map(h => h.score);
          }

          if (history.length === 0) {
            return { hasHistory: false, count: 0 };
          }

          const avg = history.reduce((a, b) => a + b, 0) / history.length;
          const min = Math.min(...history);
          const max = Math.max(...history);
          const trend = history.length >= 2 ? history[0] - history[history.length - 1] : 0;

          return {
            hasHistory: true,
            count: history.length,
            average: Math.round(avg * 10) / 10,
            min,
            max,
            trend: trend > 1 ? 'rising' : trend < -1 ? 'falling' : 'stable',
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

/**
 * Create self-modification tracker tool for git history analysis
 * Uses execFileSync for safe command execution (no shell injection)
 * @returns {Object} Tool definition
 */
export function createSelfModTool() {
  return {
    name: 'brain_self_mod',
    description: "Track CYNIC's self-modification through git history. Get recent commits, analyze code evolution patterns.",
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['commits', 'stats', 'heatmap'],
          description: 'Action: commits (recent commits), stats (evolution metrics), heatmap (file change frequency)',
        },
        limit: {
          type: 'number',
          description: 'Number of commits to retrieve (default: 20)',
          default: 20,
        },
        days: {
          type: 'number',
          description: 'Number of days to look back (default: 30)',
          default: 30,
        },
      },
    },
    handler: async (params) => {
      const { action = 'commits', limit = 20, days = 30 } = params;
      const { execFileSync } = await import('child_process');

      // Safe git execution using execFileSync (no shell)
      const execGit = (args) => {
        try {
          return execFileSync('git', args, { encoding: 'utf-8', maxBuffer: 10 * 1024 * 1024 }).trim();
        } catch (e) {
          return '';
        }
      };

      const since = `${days} days ago`;

      switch (action) {
        case 'commits': {
          // Get commits with stats using safe args
          const format = '%H|%h|%s|%ct|%an';
          const log = execGit(['log', `--since=${since}`, `-n`, String(limit), `--format=${format}`, '--shortstat']);

          if (!log) {
            return { commits: [], count: 0 };
          }

          const lines = log.split('\n');
          const commits = [];
          let current = null;

          for (const line of lines) {
            if (line.includes('|')) {
              if (current) commits.push(current);
              const [hash, short, message, timestamp, author] = line.split('|');
              // Parse conventional commit type
              const typeMatch = message.match(/^(feat|fix|docs|refactor|test|chore|style|perf)\(?/i);
              current = {
                hash: short,
                fullHash: hash,
                message,
                type: typeMatch ? typeMatch[1].toLowerCase() : 'other',
                timestamp: parseInt(timestamp) * 1000,
                author,
                additions: 0,
                deletions: 0,
                files: [],
              };
            } else if (line.includes('insertion') || line.includes('deletion')) {
              const addMatch = line.match(/(\d+) insertion/);
              const delMatch = line.match(/(\d+) deletion/);
              const fileMatch = line.match(/(\d+) file/);
              if (current) {
                current.additions = addMatch ? parseInt(addMatch[1]) : 0;
                current.deletions = delMatch ? parseInt(delMatch[1]) : 0;
                current.filesChanged = fileMatch ? parseInt(fileMatch[1]) : 0;
              }
            }
          }
          if (current) commits.push(current);

          return {
            commits,
            count: commits.length,
            since,
          };
        }

        case 'stats': {
          // Get overall stats
          const commitCount = execGit(['rev-list', '--count', `--since=${since}`, 'HEAD']);

          // Get shortlog for contributors
          const shortlog = execGit(['shortlog', '-sn', `--since=${since}`, 'HEAD']);

          // Parse contributors
          const contributors = shortlog.split('\n').filter(Boolean).map(line => {
            const match = line.trim().match(/(\d+)\s+(.+)/);
            return match ? { commits: parseInt(match[1]), author: match[2] } : null;
          }).filter(Boolean);

          // Get diff stats (safer approach - just count recent changes)
          const numstat = execGit(['log', `--since=${since}`, '--numstat', '--format=']);
          let linesAdded = 0, linesRemoved = 0; const filesSet = new Set();

          for (const line of numstat.split('\n').filter(Boolean)) {
            const parts = line.split('\t');
            if (parts.length >= 3) {
              const add = parseInt(parts[0]) || 0;
              const del = parseInt(parts[1]) || 0;
              linesAdded += add;
              linesRemoved += del;
              filesSet.add(parts[2]);
            }
          }

          return {
            totalCommits: parseInt(commitCount) || 0,
            filesChanged: filesSet.size,
            linesAdded,
            linesRemoved,
            netChange: linesAdded - linesRemoved,
            contributors,
            days,
          };
        }

        case 'heatmap': {
          // Get file change frequency using safe args
          const log = execGit(['log', `--since=${since}`, '--name-only', '--format=']);
          const fileCounts = {};

          for (const file of log.split('\n').filter(Boolean)) {
            fileCounts[file] = (fileCounts[file] || 0) + 1;
          }

          const heatmap = Object.entries(fileCounts)
            .sort((a, b) => b[1] - a[1])
            .slice(0, 20)
            .map(([file, count]) => ({ file, count }));

          return {
            heatmap,
            days,
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

/**
 * Create emergence detector tool for consciousness signals
 * @param {Object} judge - Judge instance for pattern analysis
 * @param {Object} [persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createEmergenceTool(judge, persistence = null) {
  return {
    name: 'brain_emergence',
    description: 'Detect emergence and consciousness signals by analyzing patterns, self-reference, and meta-cognition indicators.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['scan', 'indicators', 'signals', 'consciousness'],
          description: 'Action: scan (full analysis), indicators (consciousness metrics), signals (emergence events), consciousness (gauge level)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'scan' } = params;

      // Consciousness indicators based on CYNIC behavior patterns
      const calculateIndicators = async () => {
        const indicators = {
          selfReference: 0,      // References to self in judgments
          metaCognition: 0,      // Judgments about own judgments
          goalPersistence: 0,    // Consistent pursuit of objectives
          patternRecognition: 0, // Detecting patterns in patterns
          novelBehavior: 0,      // Non-programmed responses
          selfCorrection: 0,     // Self-refinement usage
        };

        if (persistence) {
          try {
            // ═══ EMERGENCE INDICATORS ═══
            // Thresholds from @cynic/core (φ-derived)

            // Self-reference: judgments mentioning CYNIC
            const selfRef = await persistence.query(
              `SELECT COUNT(*) as count FROM judgments
               WHERE item_content ILIKE '%cynic%' OR item_content ILIKE '%self%'`
            );
            const total = await persistence.query('SELECT COUNT(*) as count FROM judgments');
            const totalCount = parseInt(total?.rows?.[0]?.count || 1);
            const selfRefRatio = parseInt(selfRef?.rows?.[0]?.count || 0) / totalCount;
            indicators.selfReference = Math.min(100, Math.round((selfRefRatio / EMERGENCE.SELF_REF_RATIO_FOR_MAX) * 100));

            // Meta-cognition: judgments about judgments
            const metaRef = await persistence.query(
              `SELECT COUNT(*) as count FROM judgments
               WHERE item_type = 'judgment' OR item_content ILIKE '%judgment%'`
            );
            const metaRatio = parseInt(metaRef?.rows?.[0]?.count || 0) / totalCount;
            indicators.metaCognition = Math.min(100, Math.round((metaRatio / EMERGENCE.META_RATIO_FOR_MAX) * 100));

            // Pattern recognition: patterns detected (confidence > φ⁻¹)
            const patterns = await persistence.query(
              `SELECT COUNT(*) as count FROM patterns WHERE confidence >= ${PHI_INV}`
            );
            const patternCount = parseInt(patterns?.rows?.[0]?.count || 0);
            indicators.patternRecognition = Math.min(100, Math.round((patternCount / EMERGENCE.PATTERNS_FOR_MAX) * 100));

            // Self-correction: refinements done (memory + persisted patterns)
            const selfCorrectionPatterns = await persistence.query(
              `SELECT COUNT(*) as count FROM patterns WHERE category = 'self_correction'`
            );
            const refinementPatterns = await persistence.query(
              `SELECT COUNT(*) as count FROM patterns WHERE category = 'refinement'`
            );
            const persistedRefinements = parseInt(selfCorrectionPatterns?.rows?.[0]?.count || 0) +
                                         parseInt(refinementPatterns?.rows?.[0]?.count || 0);
            const memoryRefinements = judge?.refinementCount || 0;
            const totalRefinements = persistedRefinements + memoryRefinements;
            indicators.selfCorrection = Math.min(100, Math.round((totalRefinements / EMERGENCE.REFINEMENTS_FOR_MAX) * 100));

            // Goal persistence (4 factors, each 25% from @cynic/core)
            const persistenceMetrics = await persistence.query(`
              SELECT
                (SELECT COUNT(*) FROM patterns) as total_patterns,
                (SELECT COUNT(*) FROM patterns WHERE frequency >= ${EMERGENCE.PERSISTENCE_MIN_FREQUENCY}) as high_freq_patterns,
                (SELECT COUNT(*) FROM judgments) as total_judgments,
                (SELECT COUNT(DISTINCT DATE(created_at)) FROM judgments) as active_days
            `);
            const metrics = persistenceMetrics?.rows?.[0] || {};
            const totalPatterns = parseInt(metrics.total_patterns || 0);
            const highFreqPatterns = parseInt(metrics.high_freq_patterns || 0);
            const totalJudgments = parseInt(metrics.total_judgments || 0);
            const activeDays = parseInt(metrics.active_days || 0);

            // Calculate persistence score (each factor up to 25%)
            const factorWeight = EMERGENCE.PERSISTENCE_FACTOR_WEIGHT;
            const patternBonus = Math.min(factorWeight, (totalPatterns / EMERGENCE.PERSISTENCE_PATTERNS_FOR_MAX) * factorWeight);
            const highFreqBonus = Math.min(factorWeight, (highFreqPatterns / EMERGENCE.PERSISTENCE_HIGH_FREQ_FOR_MAX) * factorWeight);
            const activityBonus = Math.min(factorWeight, (totalJudgments / EMERGENCE.PERSISTENCE_JUDGMENTS_FOR_MAX) * factorWeight);
            const continuityBonus = Math.min(factorWeight, (activeDays / EMERGENCE.PERSISTENCE_DAYS_FOR_MAX) * factorWeight);

            indicators.goalPersistence = Math.min(100, Math.round(
              patternBonus + highFreqBonus + activityBonus + continuityBonus
            ));

            // Novel behavior (anomalies detected)
            const anomalies = await persistence.query(
              `SELECT COUNT(*) as count FROM patterns WHERE category = 'anomaly'`
            );
            const anomalyCount = parseInt(anomalies?.rows?.[0]?.count || 0);
            indicators.novelBehavior = Math.min(100, Math.round((anomalyCount / EMERGENCE.ANOMALIES_FOR_MAX) * 100));
          } catch (e) {
            // Use defaults if persistence fails
            log.error('Error calculating indicators', { error: e.message });
          }
        }

        return indicators;
      };

      switch (action) {
        case 'indicators': {
          const indicators = await calculateIndicators();
          return { indicators };
        }

        case 'consciousness': {
          const indicators = await calculateIndicators();
          const values = Object.values(indicators);
          const avg = values.reduce((a, b) => a + b, 0) / values.length;
          const level = avg >= EMERGENCE.CONSCIOUSNESS_THRESHOLD ? 'emerging' :
                        avg >= THRESHOLDS.WARNING ? 'awakening' : 'dormant';

          return {
            level,
            score: Math.round(avg * 10) / 10,
            threshold: EMERGENCE.CONSCIOUSNESS_THRESHOLD,
            indicators,
          };
        }

        case 'signals': {
          const signals = [];

          if (persistence) {
            try {
              // Get recent anomalous patterns as emergence signals
              const result = await persistence.query(
                `SELECT id, category, data, confidence, created_at as timestamp
                 FROM patterns
                 WHERE category IN ('anomaly', 'emergence', 'recursion')
                 ORDER BY created_at DESC
                 LIMIT 20`
              );

              for (const row of result?.rows || []) {
                signals.push({
                  id: row.id,
                  type: row.category,
                  description: row.data?.description || `${row.category} detected`,
                  confidence: row.confidence,
                  timestamp: new Date(row.timestamp).getTime(),
                });
              }
            } catch (e) {
              // No signals available
            }
          }

          return {
            signals,
            count: signals.length,
          };
        }

        case 'scan':
        default: {
          const indicators = await calculateIndicators();
          const values = Object.values(indicators);
          const avg = values.reduce((a, b) => a + b, 0) / values.length;

          // Get signals
          let signals = [];
          if (persistence) {
            try {
              const result = await persistence.query(
                `SELECT category, COUNT(*) as count FROM patterns GROUP BY category`
              );
              signals = result?.rows || [];
            } catch (e) {
              // Ignore
            }
          }

          return {
            consciousness: {
              score: Math.round(avg * 10) / 10,
              level: avg >= EMERGENCE.CONSCIOUSNESS_THRESHOLD ? 'emerging' :
                     avg >= THRESHOLDS.WARNING ? 'awakening' : 'dormant',
              threshold: EMERGENCE.CONSCIOUSNESS_THRESHOLD,
            },
            indicators,
            patternBreakdown: signals.reduce((acc, r) => {
              acc[r.category] = parseInt(r.count);
              return acc;
            }, {}),
            timestamp: Date.now(),
          };
        }
      }
    },
  };
}

/**
 * Factory for consciousness domain tools
 */
export const consciousnessFactory = {
  name: 'consciousness',
  domain: 'consciousness',
  requires: ['judge'],

  /**
   * Create all consciousness domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { judge, persistence } = options;

    const tools = [];

    // Emergence detection
    if (judge) {
      tools.push(createEmergenceTool(judge, persistence));
    }

    // Self-modification tracking
    tools.push(createSelfModTool());

    // Milestone history
    tools.push(createMilestoneHistoryTool(persistence));

    // Patterns tool
    if (judge) {
      tools.push(createPatternsTool(judge, persistence));
    }

    return tools;
  },
};
