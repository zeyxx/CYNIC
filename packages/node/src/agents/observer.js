/**
 * CYNIC Observer Agent - The Silent Watcher
 *
 * "I watch but do not speak. I see but do not judge loudly.
 *  I detect patterns you don't see." - κυνικός Observer
 *
 * Trigger: PostToolUse (after every tool execution)
 * Behavior: Non-blocking, silent
 * Purpose: Detect patterns, track sequences, identify anomalies
 *
 * @module @cynic/node/agents/observer
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from './base.js';

/**
 * Pattern types the Observer tracks
 */
export const PatternType = {
  REPETITION: 'repetition',      // Same action repeated
  SEQUENCE: 'sequence',          // Ordered actions
  FAILURE: 'failure',            // Repeated failures
  ANOMALY: 'anomaly',            // Unusual behavior
  ESCALATION: 'escalation',      // Increasing severity
  CYCLE: 'cycle',                // Cyclic patterns
};

/**
 * Observer Agent - Silent Pattern Detector
 */
export class Observer extends BaseAgent {
  constructor(options = {}) {
    super({
      name: 'Observer',
      trigger: AgentTrigger.POST_TOOL_USE,
      behavior: AgentBehavior.SILENT,
      ...options,
    });

    // Observation log (sliding window)
    this.observations = [];
    this.maxObservations = options.maxObservations || 1000;

    // Pattern detection thresholds
    this.repetitionThreshold = options.repetitionThreshold || 3;
    this.failureThreshold = options.failureThreshold || 3;
    this.anomalyThreshold = options.anomalyThreshold || PHI_INV_2; // 38.2%

    // Detected patterns (for reporting)
    this.detectedPatterns = [];
    this.maxPatterns = 50;

    // Tool usage tracking
    this.toolUsage = new Map();
    this.sequenceBuffer = [];
    this.maxSequenceLength = 10;
  }

  /**
   * Always trigger on PostToolUse
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.POST_TOOL_USE ||
           event.type === 'tool_use' ||
           event.tool !== undefined;
  }

  /**
   * Analyze tool use event
   */
  async analyze(event, context) {
    const observation = this._createObservation(event, context);
    this._recordObservation(observation);

    // Run all pattern detectors
    const patterns = [];

    const repetition = this._detectRepetition(observation);
    if (repetition) patterns.push(repetition);

    const sequence = this._detectSequence(observation);
    if (sequence) patterns.push(sequence);

    const failure = this._detectFailurePattern(observation);
    if (failure) patterns.push(failure);

    const anomaly = this._detectAnomaly(observation);
    if (anomaly) patterns.push(anomaly);

    const escalation = this._detectEscalation(observation);
    if (escalation) patterns.push(escalation);

    return {
      observation,
      patterns,
      patternCount: patterns.length,
      confidence: patterns.length > 0 ? Math.min(PHI_INV, patterns.length * 0.2) : 0,
    };
  }

  /**
   * Decide based on detected patterns
   */
  async decide(analysis, _context) {
    const { patterns } = analysis;

    // Record significant patterns
    for (const pattern of patterns) {
      if (pattern.strength >= this.anomalyThreshold) {
        this._recordPattern(pattern);
      }
    }

    // Observer never blocks - just logs
    if (patterns.length === 0) {
      return {
        response: AgentResponse.LOG,
        action: false,
      };
    }

    // High-strength patterns get noted
    const strongPatterns = patterns.filter(p => p.strength >= PHI_INV);
    if (strongPatterns.length > 0) {
      return {
        response: AgentResponse.LOG,
        action: true,
        patterns: strongPatterns,
        message: `Detected ${strongPatterns.length} strong pattern(s)`,
      };
    }

    return {
      response: AgentResponse.LOG,
      action: false,
      patterns,
    };
  }

  /**
   * Create observation from event
   * @private
   */
  _createObservation(event, context) {
    return {
      id: `obs_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      timestamp: Date.now(),
      tool: event.tool || event.name || 'unknown',
      success: event.success !== false && !event.error,
      duration: event.duration || 0,
      inputSize: JSON.stringify(event.input || event.params || {}).length,
      outputSize: JSON.stringify(event.output || event.result || {}).length,
      error: event.error || null,
      context: {
        sessionId: context.sessionId,
        conversationLength: context.conversationLength || 0,
        toolIndex: context.toolIndex || this.observations.length,
      },
    };
  }

  /**
   * Record observation in sliding window
   * @private
   */
  _recordObservation(observation) {
    this.observations.push(observation);

    // Maintain sliding window
    if (this.observations.length > this.maxObservations) {
      this.observations = this.observations.slice(-this.maxObservations);
    }

    // Update tool usage map
    const tool = observation.tool;
    const usage = this.toolUsage.get(tool) || { count: 0, failures: 0, totalDuration: 0 };
    usage.count++;
    if (!observation.success) usage.failures++;
    usage.totalDuration += observation.duration;
    usage.lastUsed = observation.timestamp;
    this.toolUsage.set(tool, usage);

    // Update sequence buffer
    this.sequenceBuffer.push(tool);
    if (this.sequenceBuffer.length > this.maxSequenceLength) {
      this.sequenceBuffer.shift();
    }
  }

  /**
   * Detect repetition patterns
   * @private
   */
  _detectRepetition(observation) {
    const recent = this.observations.slice(-10);
    const sameTool = recent.filter(o => o.tool === observation.tool);

    if (sameTool.length >= this.repetitionThreshold) {
      // Check if consecutive
      let consecutive = 0;
      for (let i = recent.length - 1; i >= 0; i--) {
        if (recent[i].tool === observation.tool) {
          consecutive++;
        } else {
          break;
        }
      }

      if (consecutive >= this.repetitionThreshold) {
        return {
          type: PatternType.REPETITION,
          tool: observation.tool,
          count: consecutive,
          strength: Math.min(PHI_INV, consecutive * 0.15),
          message: `Tool "${observation.tool}" used ${consecutive} times consecutively`,
        };
      }
    }

    return null;
  }

  /**
   * Detect sequence patterns
   * @private
   */
  _detectSequence(_observation) {
    if (this.sequenceBuffer.length < 4) return null;

    // Look for A-B-A-B or A-B-C-A-B-C patterns
    const seq = this.sequenceBuffer;
    const len = seq.length;

    // Check for 2-item cycle
    if (len >= 4) {
      const last2 = seq.slice(-2).join(',');
      const prev2 = seq.slice(-4, -2).join(',');
      if (last2 === prev2) {
        return {
          type: PatternType.CYCLE,
          sequence: seq.slice(-2),
          strength: PHI_INV_2,
          message: `Detected cycle: ${seq.slice(-2).join(' → ')}`,
        };
      }
    }

    // Check for 3-item cycle
    if (len >= 6) {
      const last3 = seq.slice(-3).join(',');
      const prev3 = seq.slice(-6, -3).join(',');
      if (last3 === prev3) {
        return {
          type: PatternType.CYCLE,
          sequence: seq.slice(-3),
          strength: PHI_INV,
          message: `Detected cycle: ${seq.slice(-3).join(' → ')}`,
        };
      }
    }

    return null;
  }

  /**
   * Detect failure patterns
   * @private
   */
  _detectFailurePattern(observation) {
    if (observation.success) return null;

    const recent = this.observations.slice(-10);
    const failures = recent.filter(o => !o.success);

    if (failures.length >= this.failureThreshold) {
      // Check consecutive failures
      let consecutive = 0;
      for (let i = recent.length - 1; i >= 0; i--) {
        if (!recent[i].success) {
          consecutive++;
        } else {
          break;
        }
      }

      if (consecutive >= this.failureThreshold) {
        return {
          type: PatternType.FAILURE,
          consecutiveFailures: consecutive,
          strength: Math.min(PHI_INV, consecutive * 0.2),
          message: `${consecutive} consecutive failures detected`,
          tools: failures.slice(-consecutive).map(f => f.tool),
        };
      }
    }

    return null;
  }

  /**
   * Detect anomalies (unusual behavior)
   * @private
   */
  _detectAnomaly(observation) {
    const usage = this.toolUsage.get(observation.tool);
    if (!usage || usage.count < 5) return null;

    // Check if duration is anomalous
    const avgDuration = usage.totalDuration / usage.count;
    if (observation.duration > avgDuration * 3) {
      return {
        type: PatternType.ANOMALY,
        subtype: 'slow_execution',
        tool: observation.tool,
        expected: avgDuration,
        actual: observation.duration,
        strength: Math.min(PHI_INV, (observation.duration / avgDuration - 1) * 0.2),
        message: `Unusually slow: ${observation.tool} took ${observation.duration}ms (avg: ${avgDuration.toFixed(0)}ms)`,
      };
    }

    // Check failure rate anomaly
    const failureRate = usage.failures / usage.count;
    if (failureRate > PHI_INV_2 && usage.count >= 5) {
      return {
        type: PatternType.ANOMALY,
        subtype: 'high_failure_rate',
        tool: observation.tool,
        failureRate,
        strength: failureRate,
        message: `High failure rate for "${observation.tool}": ${(failureRate * 100).toFixed(1)}%`,
      };
    }

    return null;
  }

  /**
   * Detect escalation (increasing severity)
   * @private
   */
  _detectEscalation(_observation) {
    if (this.observations.length < 10) return null;

    const recent = this.observations.slice(-10);
    const failures = recent.map((o, i) => (!o.success ? i : -1)).filter(i => i >= 0);

    // Check if failures are accelerating (more recent = more failures)
    if (failures.length >= 3) {
      const firstHalf = failures.filter(i => i < 5).length;
      const secondHalf = failures.filter(i => i >= 5).length;

      if (secondHalf > firstHalf * 2) {
        return {
          type: PatternType.ESCALATION,
          trend: 'accelerating_failures',
          firstHalf,
          secondHalf,
          strength: PHI_INV_2,
          message: `Failures accelerating: ${firstHalf} → ${secondHalf} in last 10 operations`,
        };
      }
    }

    return null;
  }

  /**
   * Record pattern for later retrieval
   * @private
   */
  _recordPattern(pattern) {
    this.detectedPatterns.push({
      ...pattern,
      detectedAt: Date.now(),
    });

    // Keep only recent patterns
    if (this.detectedPatterns.length > this.maxPatterns) {
      this.detectedPatterns = this.detectedPatterns.slice(-this.maxPatterns);
    }

    // Also record in base class
    this.recordPattern(pattern);
  }

  /**
   * Get detected patterns
   * @param {Object} [options] - Filter options
   * @returns {Object[]} Patterns
   */
  getPatterns(options = {}) {
    let patterns = [...this.detectedPatterns];

    if (options.type) {
      patterns = patterns.filter(p => p.type === options.type);
    }

    if (options.minStrength) {
      patterns = patterns.filter(p => p.strength >= options.minStrength);
    }

    if (options.limit) {
      patterns = patterns.slice(-options.limit);
    }

    return patterns;
  }

  /**
   * Get tool usage statistics
   * @returns {Object} Tool stats
   */
  getToolStats() {
    const stats = {};
    for (const [tool, usage] of this.toolUsage) {
      stats[tool] = {
        ...usage,
        avgDuration: usage.count > 0 ? usage.totalDuration / usage.count : 0,
        failureRate: usage.count > 0 ? usage.failures / usage.count : 0,
      };
    }
    return stats;
  }

  /**
   * Get observer summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      totalObservations: this.observations.length,
      uniqueTools: this.toolUsage.size,
      detectedPatterns: this.detectedPatterns.length,
      strongPatterns: this.detectedPatterns.filter(p => p.strength >= PHI_INV).length,
      recentPatterns: this.detectedPatterns.slice(-5),
    };
  }

  /**
   * Clear observation history
   */
  clear() {
    this.observations = [];
    this.toolUsage.clear();
    this.sequenceBuffer = [];
    this.detectedPatterns = [];
  }
}

export default Observer;
