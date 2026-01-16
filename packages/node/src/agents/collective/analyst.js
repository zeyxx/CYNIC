/**
 * @cynic/node - Collective Analyst Agent
 *
 * ANALYST (Binah - Understanding): The Observer & Auditor
 *
 * "Je vois, j'analyse. Patterns révèlent vérités cachées.
 *  Le silence observe ce que les mots masquent." - κυνικός Analyst
 *
 * Philosophy: Binah (Understanding) - Deep comprehension through observation.
 * Trigger: PostToolUse (after every tool execution)
 * Behavior: Silent (non-blocking, observes patterns)
 *
 * Merged from:
 * - Observer: Silent watcher, pattern detection
 * - Auditor: Code review, quality assessment
 *
 * Enhanced collective features:
 * - Organic profile signal collection (feeds ProfileCalculator)
 * - Pattern detection and emission
 * - Anomaly detection for GUARDIAN
 * - Profile update broadcasting
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/analyst
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from '../base.js';
import {
  AgentEvent,
  AgentId,
  EventPriority,
  PatternDetectedEvent,
  AnomalyDetectedEvent,
  ProfileUpdatedEvent,
} from '../events.js';
import {
  OrganicSignals,
  SignalType,
  SIGNAL_CONSTANTS,
} from '../../profile/organic-signals.js';
import {
  ProfileCalculator,
  ProfileLevel,
  PROFILE_CONSTANTS,
} from '../../profile/calculator.js';

/**
 * φ-aligned constants for Analyst
 */
export const ANALYST_CONSTANTS = {
  /** Max pattern history (Fib(10) = 55) */
  MAX_PATTERNS: 55,

  /** Pattern frequency threshold (Fib(5) = 5) */
  PATTERN_THRESHOLD: 5,

  /** Anomaly detection window (Fib(8) = 21 events) */
  ANOMALY_WINDOW: 21,

  /** Re-evaluation interval (Fib(8) = 21 interactions) */
  REEVALUATION_INTERVAL: 21,

  /** Min confidence for pattern emission (φ⁻²) */
  MIN_PATTERN_CONFIDENCE: PHI_INV_2,

  /** Session timeout for pattern tracking (Fib(13) = 233 seconds) */
  SESSION_TIMEOUT_MS: 233000,
};

/**
 * Pattern categories detected by Analyst
 */
export const PatternCategory = {
  TOOL_USAGE: 'tool_usage',
  CODE_STYLE: 'code_style',
  ERROR_PATTERN: 'error_pattern',
  LEARNING: 'learning',
  WORKFLOW: 'workflow',
  COMMUNICATION: 'communication',
};

/**
 * Anomaly types detected by Analyst
 */
export const AnomalyType = {
  UNUSUAL_COMMAND: 'unusual_command',
  RAPID_ERRORS: 'rapid_errors',
  SUSPICIOUS_PATTERN: 'suspicious_pattern',
  BEHAVIOR_CHANGE: 'behavior_change',
  RESOURCE_ABUSE: 'resource_abuse',
};

/**
 * Collective Analyst Agent - Observer & Auditor
 */
export class CollectiveAnalyst extends BaseAgent {
  /**
   * @param {Object} options - Agent options
   * @param {Object} [options.eventBus] - Event bus for inter-agent communication
   * @param {Object} [options.profileCalculator] - Profile calculator instance
   * @param {Object} [options.signalCollector] - Signal collector instance
   */
  constructor(options = {}) {
    super({
      name: 'Analyst',
      trigger: AgentTrigger.POST_TOOL_USE,
      behavior: AgentBehavior.SILENT,
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Profile calculation
    this.profileCalculator = options.profileCalculator || new ProfileCalculator();
    this.signalCollector = options.signalCollector || new OrganicSignals();

    // Pattern tracking
    this.patterns = new Map();
    this.patternHistory = [];

    // Anomaly detection
    this.recentEvents = [];
    this.anomalyBaseline = new Map();

    // Tool usage stats
    this.toolStats = new Map();

    // Error tracking
    this.errorHistory = [];
    this.errorRate = 0;

    // Session tracking
    this.sessionStart = Date.now();
    this.interactionCount = 0;
    this.lastProfileLevel = ProfileLevel.PRACTITIONER;

    // Code analysis cache
    this.codeAnalysisCache = new Map();
  }

  /**
   * Trigger on PostToolUse
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.POST_TOOL_USE ||
           event.type === 'post_tool_use' ||
           event.tool !== undefined;
  }

  /**
   * Analyze tool use and extract patterns
   */
  async analyze(event, context) {
    this.interactionCount++;

    const tool = event.tool || event.name || 'unknown';
    const input = event.input || event.params || {};
    const output = event.output || event.result || {};
    const success = !event.error && !output.error;

    // Track tool usage
    this._trackToolUsage(tool, input, success);

    // Collect behavioral signals
    const behavioralSignals = this._collectBehavioralSignals(tool, input, output, success);

    // Collect linguistic signals if message present
    let linguisticSignals = null;
    if (context.message) {
      linguisticSignals = this.signalCollector.collectLinguistic(context.message);
      this.profileCalculator.processMessage(context.message);
    }

    // Collect code signals if code present
    let codeSignals = null;
    if (input.content && this._looksLikeCode(input.content)) {
      codeSignals = this.signalCollector.collectCode(input.content);
      this.profileCalculator.processCode(input.content);
    }

    // Track errors
    if (!success) {
      this._trackError(tool, event.error || output.error);
    }

    // Detect patterns
    const detectedPatterns = this._detectPatterns(tool, input, output, context);

    // Detect anomalies
    const anomalies = this._detectAnomalies(tool, input, output, context);

    // Check if profile re-evaluation needed
    const profileUpdate = this._checkProfileUpdate();

    return {
      tool,
      success,
      patterns: detectedPatterns,
      anomalies,
      profileUpdate,
      signals: {
        behavioral: behavioralSignals,
        linguistic: linguisticSignals,
        code: codeSignals,
      },
      interactionCount: this.interactionCount,
      confidence: Math.min(PHI_INV, this._calculateConfidence()),
    };
  }

  /**
   * Decide actions based on analysis
   */
  async decide(analysis, context) {
    const { patterns, anomalies, profileUpdate, confidence } = analysis;

    // Emit pattern events
    for (const pattern of patterns) {
      if (pattern.confidence >= ANALYST_CONSTANTS.MIN_PATTERN_CONFIDENCE) {
        this._emitPatternDetected(pattern);
      }
    }

    // Emit anomaly events (these go to Guardian)
    for (const anomaly of anomalies) {
      this._emitAnomalyDetected(anomaly);
    }

    // Emit profile update if changed
    if (profileUpdate) {
      this._emitProfileUpdated(profileUpdate);
    }

    // Record summary pattern
    this.recordPattern({
      type: 'analysis_complete',
      patterns: patterns.length,
      anomalies: anomalies.length,
      profileUpdated: !!profileUpdate,
      confidence,
    });

    return {
      response: AgentResponse.LOG,
      action: patterns.length > 0 || anomalies.length > 0 || profileUpdate,
      patterns,
      anomalies,
      profileUpdate,
    };
  }

  /**
   * Track tool usage statistics
   * @private
   */
  _trackToolUsage(tool, input, success) {
    const stats = this.toolStats.get(tool) || {
      count: 0,
      successes: 0,
      failures: 0,
      lastUsed: null,
      avgInterval: 0,
    };

    const now = Date.now();
    if (stats.lastUsed) {
      const interval = now - stats.lastUsed;
      stats.avgInterval = (stats.avgInterval * stats.count + interval) / (stats.count + 1);
    }

    stats.count++;
    if (success) {
      stats.successes++;
    } else {
      stats.failures++;
    }
    stats.lastUsed = now;

    this.toolStats.set(tool, stats);

    // Update profile calculator with tool call
    this.profileCalculator.processToolCall(tool, input, success);
  }

  /**
   * Collect behavioral signals from tool usage
   * @private
   */
  _collectBehavioralSignals(tool, input, output, success) {
    return this.signalCollector.collectBehavioral({
      tool,
      input,
      output,
      success,
      timestamp: Date.now(),
      sessionDuration: Date.now() - this.sessionStart,
    });
  }

  /**
   * Track errors for pattern detection
   * @private
   */
  _trackError(tool, error) {
    const errorRecord = {
      tool,
      error: typeof error === 'string' ? error : error?.message || 'Unknown error',
      timestamp: Date.now(),
    };

    // Keep bounded error history
    while (this.errorHistory.length >= ANALYST_CONSTANTS.MAX_PATTERNS) {
      this.errorHistory.shift();
    }
    this.errorHistory.push(errorRecord);

    // Calculate error rate over recent window
    const windowStart = Date.now() - ANALYST_CONSTANTS.SESSION_TIMEOUT_MS;
    const recentErrors = this.errorHistory.filter(e => e.timestamp > windowStart);
    this.errorRate = recentErrors.length / Math.max(1, this.interactionCount);
  }

  /**
   * Detect patterns in usage
   * @private
   */
  _detectPatterns(tool, input, output, context) {
    const patterns = [];

    // Tool sequence pattern
    const sequencePattern = this._detectToolSequence(tool);
    if (sequencePattern) {
      patterns.push(sequencePattern);
    }

    // Error pattern
    const errorPattern = this._detectErrorPattern();
    if (errorPattern) {
      patterns.push(errorPattern);
    }

    // Workflow pattern
    const workflowPattern = this._detectWorkflowPattern(tool, input);
    if (workflowPattern) {
      patterns.push(workflowPattern);
    }

    // Code style pattern (if code present)
    if (input.content && this._looksLikeCode(input.content)) {
      const stylePattern = this._detectCodeStylePattern(input.content);
      if (stylePattern) {
        patterns.push(stylePattern);
      }
    }

    // Learning pattern
    const learningPattern = this._detectLearningPattern();
    if (learningPattern) {
      patterns.push(learningPattern);
    }

    // Record patterns
    for (const pattern of patterns) {
      this._recordPattern(pattern);
    }

    return patterns;
  }

  /**
   * Detect tool sequence patterns
   * @private
   */
  _detectToolSequence(currentTool) {
    // Keep recent tools for sequence detection
    while (this.recentEvents.length >= ANALYST_CONSTANTS.ANOMALY_WINDOW) {
      this.recentEvents.shift();
    }
    this.recentEvents.push({ tool: currentTool, timestamp: Date.now() });

    if (this.recentEvents.length < 3) return null;

    // Get recent tool sequence
    const recentTools = this.recentEvents.slice(-5).map(e => e.tool);
    const sequenceKey = recentTools.join('→');

    // Check if this sequence is common
    const count = this.patterns.get(sequenceKey) || 0;
    this.patterns.set(sequenceKey, count + 1);

    if (count + 1 >= ANALYST_CONSTANTS.PATTERN_THRESHOLD) {
      return {
        type: 'tool_sequence',
        category: PatternCategory.WORKFLOW,
        sequence: recentTools,
        count: count + 1,
        confidence: Math.min(PHI_INV, (count + 1) / 10),
        hash: this._hashString(sequenceKey),
        context: { tools: recentTools },
      };
    }

    return null;
  }

  /**
   * Detect error patterns
   * @private
   */
  _detectErrorPattern() {
    if (this.errorHistory.length < 3) return null;

    // Check for rapid errors
    const windowStart = Date.now() - 60000; // Last minute
    const recentErrors = this.errorHistory.filter(e => e.timestamp > windowStart);

    if (recentErrors.length >= 3) {
      // Group by tool
      const errorsByTool = new Map();
      for (const err of recentErrors) {
        const count = errorsByTool.get(err.tool) || 0;
        errorsByTool.set(err.tool, count + 1);
      }

      // Find tool with most errors
      let maxTool = null;
      let maxCount = 0;
      for (const [tool, count] of errorsByTool) {
        if (count > maxCount) {
          maxTool = tool;
          maxCount = count;
        }
      }

      if (maxCount >= 3) {
        return {
          type: 'error_cluster',
          category: PatternCategory.ERROR_PATTERN,
          tool: maxTool,
          count: maxCount,
          confidence: Math.min(PHI_INV, maxCount / 5),
          hash: this._hashString(`error:${maxTool}:${maxCount}`),
          context: { errors: recentErrors.slice(-5) },
        };
      }
    }

    return null;
  }

  /**
   * Detect workflow patterns
   * @private
   */
  _detectWorkflowPattern(tool, input) {
    // Detect common workflow patterns
    const workflows = {
      'edit-then-test': ['Edit', 'Write'].includes(tool) &&
                        this.recentEvents.some(e => e.tool === 'Bash'),
      'read-then-edit': tool === 'Edit' &&
                        this.recentEvents.slice(-3).some(e => e.tool === 'Read'),
      'search-then-read': tool === 'Read' &&
                         this.recentEvents.slice(-3).some(e =>
                           e.tool === 'Grep' || e.tool === 'Glob'),
    };

    for (const [pattern, detected] of Object.entries(workflows)) {
      if (detected) {
        const key = `workflow:${pattern}`;
        const count = (this.patterns.get(key) || 0) + 1;
        this.patterns.set(key, count);

        if (count >= ANALYST_CONSTANTS.PATTERN_THRESHOLD) {
          return {
            type: pattern,
            category: PatternCategory.WORKFLOW,
            count,
            confidence: Math.min(PHI_INV, count / 10),
            hash: this._hashString(key),
            context: { workflow: pattern },
          };
        }
      }
    }

    return null;
  }

  /**
   * Detect code style patterns
   * @private
   */
  _detectCodeStylePattern(code) {
    const signals = this.signalCollector.collectCode(code);
    if (!signals) return null;

    // Check for consistent style patterns
    const styleKey = `style:${signals.hasTypeAnnotations}:${signals.hasDocstrings}:${signals.hasTests}`;
    const count = (this.patterns.get(styleKey) || 0) + 1;
    this.patterns.set(styleKey, count);

    if (count >= ANALYST_CONSTANTS.PATTERN_THRESHOLD) {
      return {
        type: 'code_style',
        category: PatternCategory.CODE_STYLE,
        style: {
          typed: signals.hasTypeAnnotations,
          documented: signals.hasDocstrings,
          tested: signals.hasTests,
        },
        count,
        confidence: Math.min(PHI_INV, count / 10),
        hash: this._hashString(styleKey),
        context: { signals },
      };
    }

    return null;
  }

  /**
   * Detect learning patterns
   * @private
   */
  _detectLearningPattern() {
    const profile = this.profileCalculator.getProfile();
    const history = profile.signalHistory;

    if (history.length < 5) return null;

    // Calculate improvement trend
    const recentSignals = history.slice(-5);
    const trend = this._calculateTrend(recentSignals.map(s => s.score));

    if (trend > 0.1) {
      return {
        type: 'improving',
        category: PatternCategory.LEARNING,
        trend,
        recentScores: recentSignals.map(s => s.score),
        confidence: Math.min(PHI_INV, trend),
        hash: this._hashString(`learning:improving:${Math.round(trend * 100)}`),
        context: { trend, scores: recentSignals.map(s => s.score) },
      };
    }

    if (trend < -0.1) {
      return {
        type: 'struggling',
        category: PatternCategory.LEARNING,
        trend,
        recentScores: recentSignals.map(s => s.score),
        confidence: Math.min(PHI_INV, Math.abs(trend)),
        hash: this._hashString(`learning:struggling:${Math.round(trend * 100)}`),
        context: { trend, scores: recentSignals.map(s => s.score) },
      };
    }

    return null;
  }

  /**
   * Detect anomalies in behavior
   * @private
   */
  _detectAnomalies(tool, input, output, context) {
    const anomalies = [];

    // Rapid error anomaly
    if (this.errorRate > 0.3) {
      anomalies.push({
        type: AnomalyType.RAPID_ERRORS,
        severity: this.errorRate > 0.5 ? 'critical' : 'high',
        confidence: Math.min(PHI_INV, this.errorRate),
        description: `High error rate: ${Math.round(this.errorRate * 100)}%`,
        context: { errorRate: this.errorRate, recentErrors: this.errorHistory.slice(-5) },
      });
    }

    // Unusual command detection (for bash)
    if (tool === 'Bash' || tool === 'bash') {
      const cmd = input.command || '';
      const unusualScore = this._scoreCommandUnusualness(cmd);

      if (unusualScore > 0.7) {
        anomalies.push({
          type: AnomalyType.UNUSUAL_COMMAND,
          severity: unusualScore > 0.9 ? 'critical' : 'high',
          confidence: Math.min(PHI_INV, unusualScore),
          description: `Unusual command pattern detected`,
          context: { command: cmd.slice(0, 100), score: unusualScore },
        });
      }
    }

    // Behavior change detection
    const behaviorChange = this._detectBehaviorChange();
    if (behaviorChange) {
      anomalies.push(behaviorChange);
    }

    return anomalies;
  }

  /**
   * Score how unusual a command is
   * @private
   */
  _scoreCommandUnusualness(cmd) {
    let score = 0;

    // Check for unusual patterns
    const unusualPatterns = [
      { pattern: /base64/, weight: 0.3 },
      { pattern: /eval/, weight: 0.4 },
      { pattern: /\$\([^)]*\)/, weight: 0.2 }, // Command substitution
      { pattern: /[|&]{2,}/, weight: 0.2 },    // Complex pipes
      { pattern: /\/dev\/null/, weight: 0.1 },
      { pattern: /2>&1/, weight: 0.1 },
      { pattern: /nohup|disown/, weight: 0.3 },
      { pattern: /export\s+[A-Z_]+=/, weight: 0.2 },
    ];

    for (const { pattern, weight } of unusualPatterns) {
      if (pattern.test(cmd)) {
        score += weight;
      }
    }

    // Check command length (very long commands are unusual)
    if (cmd.length > 200) {
      score += 0.2;
    }
    if (cmd.length > 500) {
      score += 0.3;
    }

    return Math.min(1, score);
  }

  /**
   * Detect significant behavior changes
   * @private
   */
  _detectBehaviorChange() {
    const profile = this.profileCalculator.getProfile();
    const history = profile.signalHistory;

    if (history.length < 10) return null;

    // Compare recent vs historical behavior
    const recent = history.slice(-5);
    const historical = history.slice(-15, -5);

    if (historical.length < 5) return null;

    const recentAvg = recent.reduce((s, h) => s + h.score, 0) / recent.length;
    const historicalAvg = historical.reduce((s, h) => s + h.score, 0) / historical.length;
    const change = Math.abs(recentAvg - historicalAvg);

    if (change > 20) {
      return {
        type: AnomalyType.BEHAVIOR_CHANGE,
        severity: change > 30 ? 'high' : 'medium',
        confidence: Math.min(PHI_INV, change / 50),
        description: `Significant behavior change: ${change > 0 ? '+' : ''}${Math.round(change)} points`,
        context: { recentAvg, historicalAvg, change },
      };
    }

    return null;
  }

  /**
   * Check if profile needs update
   * @private
   */
  _checkProfileUpdate() {
    // Only check at re-evaluation intervals
    if (this.interactionCount % ANALYST_CONSTANTS.REEVALUATION_INTERVAL !== 0) {
      return null;
    }

    const profile = this.profileCalculator.getProfile();
    const newLevel = profile.level;

    if (newLevel !== this.lastProfileLevel) {
      const update = {
        previousLevel: this.lastProfileLevel,
        newLevel,
        levelName: PROFILE_CONSTANTS.LEVEL_NAMES[newLevel],
        confidence: Math.min(PHI_INV, profile.confidence),
        reason: this._explainLevelChange(this.lastProfileLevel, newLevel, profile),
        adaptationHints: this._getAdaptationHints(newLevel),
      };

      this.lastProfileLevel = newLevel;
      return update;
    }

    return null;
  }

  /**
   * Explain why profile level changed
   * @private
   */
  _explainLevelChange(oldLevel, newLevel, profile) {
    const direction = newLevel > oldLevel ? 'increased' : 'decreased';
    const signals = profile.recentSignals;

    const reasons = [];
    if (signals.linguistic > 60) reasons.push('sophisticated communication');
    if (signals.behavioral > 60) reasons.push('advanced tool usage');
    if (signals.code > 60) reasons.push('complex code patterns');
    if (signals.linguistic < 40) reasons.push('basic communication');
    if (signals.behavioral < 40) reasons.push('simple tool usage');

    return `Level ${direction}: ${reasons.join(', ') || 'cumulative signal changes'}`;
  }

  /**
   * Get adaptation hints for agents
   * @private
   */
  _getAdaptationHints(level) {
    const hints = {
      [ProfileLevel.NOVICE]: {
        verbosity: 'high',
        examples: true,
        warnings: 'frequent',
        complexity: 'low',
      },
      [ProfileLevel.APPRENTICE]: {
        verbosity: 'medium-high',
        examples: true,
        warnings: 'normal',
        complexity: 'low-medium',
      },
      [ProfileLevel.PRACTITIONER]: {
        verbosity: 'medium',
        examples: 'on-request',
        warnings: 'normal',
        complexity: 'medium',
      },
      [ProfileLevel.EXPERT]: {
        verbosity: 'low',
        examples: false,
        warnings: 'minimal',
        complexity: 'high',
      },
      [ProfileLevel.MASTER]: {
        verbosity: 'minimal',
        examples: false,
        warnings: 'critical-only',
        complexity: 'expert',
      },
    };

    return hints[level] || hints[ProfileLevel.PRACTITIONER];
  }

  /**
   * Emit pattern detected event
   * @private
   */
  _emitPatternDetected(pattern) {
    if (!this.eventBus) return;

    const event = new PatternDetectedEvent(
      AgentId.ANALYST,
      pattern
    );

    this.eventBus.publish(event);
  }

  /**
   * Emit anomaly detected event
   * @private
   */
  _emitAnomalyDetected(anomaly) {
    if (!this.eventBus) return;

    const event = new AnomalyDetectedEvent(
      AgentId.ANALYST,
      anomaly
    );

    this.eventBus.publish(event);
  }

  /**
   * Emit profile updated event
   * @private
   */
  _emitProfileUpdated(update) {
    if (!this.eventBus) return;

    const event = new ProfileUpdatedEvent(
      AgentId.ANALYST,
      update
    );

    this.eventBus.publish(event);
  }

  /**
   * Record pattern for history
   * @private
   */
  _recordPattern(pattern) {
    while (this.patternHistory.length >= ANALYST_CONSTANTS.MAX_PATTERNS) {
      this.patternHistory.shift();
    }
    this.patternHistory.push({
      ...pattern,
      timestamp: Date.now(),
    });
  }

  /**
   * Check if content looks like code
   * @private
   */
  _looksLikeCode(content) {
    if (!content || typeof content !== 'string') return false;

    const codeIndicators = [
      /function\s+\w+/,
      /class\s+\w+/,
      /const\s+\w+\s*=/,
      /let\s+\w+\s*=/,
      /var\s+\w+\s*=/,
      /import\s+.*from/,
      /export\s+(default\s+)?/,
      /def\s+\w+\(/,
      /async\s+/,
      /=>\s*[{(]/,
    ];

    return codeIndicators.some(pattern => pattern.test(content));
  }

  /**
   * Calculate trend from values
   * @private
   */
  _calculateTrend(values) {
    if (values.length < 2) return 0;

    // Simple linear regression slope
    const n = values.length;
    let sumX = 0, sumY = 0, sumXY = 0, sumXX = 0;

    for (let i = 0; i < n; i++) {
      sumX += i;
      sumY += values[i];
      sumXY += i * values[i];
      sumXX += i * i;
    }

    const slope = (n * sumXY - sumX * sumY) / (n * sumXX - sumX * sumX);
    return slope / 10; // Normalize
  }

  /**
   * Calculate overall confidence
   * @private
   */
  _calculateConfidence() {
    const profile = this.profileCalculator.getProfile();
    return profile.confidence;
  }

  /**
   * Hash string for pattern identification
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
   * Set event bus for inter-agent communication
   * @param {Object} eventBus - Event bus instance
   */
  setEventBus(eventBus) {
    this.eventBus = eventBus;
  }

  /**
   * Get current profile
   * @returns {Object} Profile data
   */
  getProfile() {
    return this.profileCalculator.getProfile();
  }

  /**
   * Get tool usage statistics
   * @returns {Map} Tool stats
   */
  getToolStats() {
    return new Map(this.toolStats);
  }

  /**
   * Get detected patterns
   * @returns {Object[]} Pattern history
   */
  getPatterns() {
    return [...this.patternHistory];
  }

  /**
   * Get analyst summary
   * @returns {Object} Summary
   */
  getSummary() {
    const profile = this.profileCalculator.getProfile();

    return {
      ...this.getStats(),
      profileLevel: profile.level,
      profileLevelName: PROFILE_CONSTANTS.LEVEL_NAMES[profile.level],
      interactionCount: this.interactionCount,
      patternsDetected: this.patternHistory.length,
      errorRate: this.errorRate,
      toolsUsed: this.toolStats.size,
      topTools: Array.from(this.toolStats.entries())
        .sort((a, b) => b[1].count - a[1].count)
        .slice(0, 5)
        .map(([tool, stats]) => ({ tool, ...stats })),
      recentPatterns: this.patternHistory.slice(-5),
    };
  }

  /**
   * Reset session
   */
  resetSession() {
    this.sessionStart = Date.now();
    this.interactionCount = 0;
    this.recentEvents = [];
    this.errorHistory = [];
    this.errorRate = 0;
  }

  /**
   * Clear all data
   */
  clear() {
    this.patterns.clear();
    this.patternHistory = [];
    this.recentEvents = [];
    this.anomalyBaseline.clear();
    this.toolStats.clear();
    this.errorHistory = [];
    this.errorRate = 0;
    this.codeAnalysisCache.clear();
    this.resetSession();
  }
}

export default CollectiveAnalyst;
