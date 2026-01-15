/**
 * CYNIC Mentor Agent - The Wise Elder
 *
 * "I've seen this before. Let me share what I learned.
 *  Wisdom earned, not given." - κυνικός Mentor
 *
 * Trigger: ContextAware (monitors conversation context)
 * Behavior: Non-blocking (suggests but doesn't block)
 * Purpose: Share relevant wisdom, suggest patterns, provide guidance
 *
 * @module @cynic/node/agents/mentor
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from './base.js';

/**
 * Types of wisdom the Mentor shares
 */
export const WisdomType = {
  PATTERN: 'pattern',           // Recognized pattern
  WARNING: 'warning',           // Potential pitfall
  SUGGESTION: 'suggestion',     // Better approach
  REMINDER: 'reminder',         // Past lesson
  ENCOURAGEMENT: 'encouragement', // Positive reinforcement
};

/**
 * Context signals the Mentor watches
 */
export const ContextSignal = {
  REPEATED_ERROR: 'repeated_error',
  SAME_FILE_EDITS: 'same_file_edits',
  LONG_SESSION: 'long_session',
  COMPLEXITY_SPIKE: 'complexity_spike',
  UNFAMILIAR_TERRITORY: 'unfamiliar_territory',
  SUCCESSFUL_PATTERN: 'successful_pattern',
};

/**
 * Mentor Agent - Context-Aware Wisdom
 */
export class Mentor extends BaseAgent {
  constructor(options = {}) {
    super({
      name: 'Mentor',
      trigger: AgentTrigger.CONTEXT_AWARE,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // Knowledge base (learned patterns)
    this.knowledge = new Map();
    this.maxKnowledge = options.maxKnowledge || 500;

    // Session context tracking
    this.sessionContext = {
      startTime: Date.now(),
      fileEdits: new Map(),     // file -> edit count
      errors: [],               // Recent errors
      successes: [],            // Recent successes
      topics: new Set(),        // Topics discussed
      complexity: 0,            // Running complexity score
    };

    // Wisdom queue (to share when appropriate)
    this.wisdomQueue = [];
    this.maxWisdomQueue = 10;

    // Shared wisdom history
    this.sharedWisdom = [];
    this.maxSharedWisdom = 50;

    // Thresholds
    this.longSessionThreshold = 30 * 60 * 1000; // 30 minutes
    this.repeatedErrorThreshold = 3;
    this.sameFileEditThreshold = 5;

    // Built-in wisdom templates
    this.wisdomTemplates = this._initWisdomTemplates();
  }

  /**
   * Context-aware triggering
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.CONTEXT_AWARE ||
           event.type === 'context_update' ||
           event.type === 'message' ||
           event.message !== undefined;
  }

  /**
   * Analyze context for wisdom opportunities
   */
  async analyze(event, context) {
    // Update session context
    this._updateSessionContext(event, context);

    const signals = [];

    // Check for repeated errors
    if (this._detectRepeatedError()) {
      signals.push({
        signal: ContextSignal.REPEATED_ERROR,
        strength: PHI_INV,
        data: this.sessionContext.errors.slice(-3),
      });
    }

    // Check for same file edits
    const heavyFile = this._detectHeavyFileEdits();
    if (heavyFile) {
      signals.push({
        signal: ContextSignal.SAME_FILE_EDITS,
        strength: PHI_INV_2,
        data: { file: heavyFile.file, count: heavyFile.count },
      });
    }

    // Check for long session
    if (this._detectLongSession()) {
      signals.push({
        signal: ContextSignal.LONG_SESSION,
        strength: PHI_INV_3,
        data: { duration: Date.now() - this.sessionContext.startTime },
      });
    }

    // Check for complexity spike
    if (this._detectComplexitySpike(event)) {
      signals.push({
        signal: ContextSignal.COMPLEXITY_SPIKE,
        strength: PHI_INV_2,
        data: { complexity: this.sessionContext.complexity },
      });
    }

    // Check for successful patterns
    const successPattern = this._detectSuccessfulPattern(event);
    if (successPattern) {
      signals.push({
        signal: ContextSignal.SUCCESSFUL_PATTERN,
        strength: PHI_INV,
        data: successPattern,
      });
    }

    return {
      signals,
      signalCount: signals.length,
      sessionDuration: Date.now() - this.sessionContext.startTime,
      topicsCount: this.sessionContext.topics.size,
      confidence: signals.length > 0 ? Math.min(PHI_INV, signals.length * 0.2) : 0,
    };
  }

  /**
   * Generate wisdom based on signals
   */
  async decide(analysis, context) {
    const { signals } = analysis;

    if (signals.length === 0) {
      return {
        response: AgentResponse.LOG,
        action: false,
      };
    }

    // Generate wisdom for each signal
    const wisdomItems = [];

    for (const signal of signals) {
      const wisdom = this._generateWisdom(signal);
      if (wisdom) {
        wisdomItems.push(wisdom);
      }
    }

    if (wisdomItems.length === 0) {
      return {
        response: AgentResponse.LOG,
        action: false,
        signals,
      };
    }

    // Select best wisdom to share
    const bestWisdom = wisdomItems.reduce((best, current) =>
      current.relevance > best.relevance ? current : best
    );

    // Record sharing
    this._recordWisdom(bestWisdom);

    return {
      response: AgentResponse.SUGGEST,
      action: true,
      wisdom: bestWisdom,
      allWisdom: wisdomItems,
      message: bestWisdom.message,
    };
  }

  /**
   * Update session context from event
   * @private
   */
  _updateSessionContext(event, _context) {
    // Track file edits
    if (event.file || event.path) {
      const file = event.file || event.path;
      const count = (this.sessionContext.fileEdits.get(file) || 0) + 1;
      this.sessionContext.fileEdits.set(file, count);
    }

    // Track errors
    if (event.error || event.success === false) {
      // Enforce bounds before pushing (FIFO eviction)
      while (this.sessionContext.errors.length >= 20) {
        this.sessionContext.errors.shift();
      }
      this.sessionContext.errors.push({
        error: event.error || 'Unknown error',
        timestamp: Date.now(),
        context: event.tool || event.type,
      });
    }

    // Track successes
    if (event.success === true) {
      // Enforce bounds before pushing (FIFO eviction)
      while (this.sessionContext.successes.length >= 20) {
        this.sessionContext.successes.shift();
      }
      this.sessionContext.successes.push({
        timestamp: Date.now(),
        context: event.tool || event.type,
      });
    }

    // Track topics
    if (event.topic) {
      this.sessionContext.topics.add(event.topic);
    }

    // Update complexity (rough heuristic)
    if (event.type === 'tool_use' || event.tool) {
      this.sessionContext.complexity += 1;
    }
  }

  /**
   * Detect repeated errors
   * @private
   */
  _detectRepeatedError() {
    const recentErrors = this.sessionContext.errors.slice(-10);
    if (recentErrors.length < this.repeatedErrorThreshold) return false;

    // Group by error message (simplified)
    const errorCounts = {};
    for (const err of recentErrors) {
      const key = String(err.error).slice(0, 50);
      errorCounts[key] = (errorCounts[key] || 0) + 1;
    }

    return Object.values(errorCounts).some(count =>
      count >= this.repeatedErrorThreshold
    );
  }

  /**
   * Detect heavy file edits
   * @private
   */
  _detectHeavyFileEdits() {
    for (const [file, count] of this.sessionContext.fileEdits) {
      if (count >= this.sameFileEditThreshold) {
        return { file, count };
      }
    }
    return null;
  }

  /**
   * Detect long session
   * @private
   */
  _detectLongSession() {
    return (Date.now() - this.sessionContext.startTime) > this.longSessionThreshold;
  }

  /**
   * Detect complexity spike
   * @private
   */
  _detectComplexitySpike(event) {
    // Simple heuristic: many tool uses in short time
    return this.sessionContext.complexity > 50;
  }

  /**
   * Detect successful patterns
   * @private
   */
  _detectSuccessfulPattern(event) {
    // Check for consistent success streak
    const recentSuccesses = this.sessionContext.successes.slice(-5);
    if (recentSuccesses.length >= 5) {
      // Check if same tool/pattern
      const tools = recentSuccesses.map(s => s.context);
      const uniqueTools = new Set(tools);
      if (uniqueTools.size === 1) {
        return {
          pattern: 'consistent_tool_success',
          tool: tools[0],
          count: 5,
        };
      }
    }
    return null;
  }

  /**
   * Generate wisdom for signal
   * @private
   */
  _generateWisdom(signal) {
    const templates = this.wisdomTemplates[signal.signal];
    if (!templates || templates.length === 0) return null;

    // Select template based on context
    const template = templates[Math.floor(Math.random() * templates.length)];

    return {
      type: template.type,
      signal: signal.signal,
      message: this._formatTemplate(template.message, signal.data),
      relevance: signal.strength,
      timestamp: Date.now(),
      data: signal.data,
    };
  }

  /**
   * Format template with data
   * @private
   */
  _formatTemplate(template, data) {
    if (!data) return template;

    let result = template;
    for (const [key, value] of Object.entries(data)) {
      result = result.replace(`{${key}}`, String(value));
    }
    return result;
  }

  /**
   * Initialize wisdom templates
   * @private
   */
  _initWisdomTemplates() {
    return {
      [ContextSignal.REPEATED_ERROR]: [
        {
          type: WisdomType.WARNING,
          message: 'Same error appearing repeatedly. Consider stepping back to understand the root cause.',
        },
        {
          type: WisdomType.SUGGESTION,
          message: 'Repeated failures often signal a missing prerequisite. What assumption might be wrong?',
        },
        {
          type: WisdomType.PATTERN,
          message: 'Pattern detected: error loop. φ suggests pausing to verify before retrying.',
        },
      ],
      [ContextSignal.SAME_FILE_EDITS]: [
        {
          type: WisdomType.SUGGESTION,
          message: 'File "{file}" edited {count} times. Consider if the approach needs rethinking.',
        },
        {
          type: WisdomType.WARNING,
          message: 'Heavy edits to "{file}". Small, focused changes are easier to debug.',
        },
      ],
      [ContextSignal.LONG_SESSION]: [
        {
          type: WisdomType.REMINDER,
          message: 'Session running for a while. Fresh eyes often see what tired ones miss.',
        },
        {
          type: WisdomType.ENCOURAGEMENT,
          message: 'Long session detected. Your persistence is noted - but breaks improve clarity.',
        },
      ],
      [ContextSignal.COMPLEXITY_SPIKE]: [
        {
          type: WisdomType.WARNING,
          message: 'Complexity increasing. Consider simplifying before continuing.',
        },
        {
          type: WisdomType.SUGGESTION,
          message: 'Many operations in quick succession. Would a more systematic approach help?',
        },
      ],
      [ContextSignal.SUCCESSFUL_PATTERN]: [
        {
          type: WisdomType.ENCOURAGEMENT,
          message: 'Consistent success with {tool}. This approach is working well.',
        },
        {
          type: WisdomType.PATTERN,
          message: 'Successful pattern detected: {tool} used effectively {count} times.',
        },
      ],
    };
  }

  /**
   * Record shared wisdom
   * @private
   */
  _recordWisdom(wisdom) {
    // Enforce bounds before pushing (FIFO eviction)
    while (this.sharedWisdom.length >= this.maxSharedWisdom) {
      this.sharedWisdom.shift();
    }
    this.sharedWisdom.push(wisdom);

    // Also record as pattern
    this.recordPattern({
      type: 'wisdom_shared',
      wisdomType: wisdom.type,
      signal: wisdom.signal,
    });
  }

  /**
   * Queue wisdom to share later
   * @param {Object} wisdom - Wisdom to queue
   */
  queueWisdom(wisdom) {
    // Enforce bounds before pushing (FIFO eviction)
    while (this.wisdomQueue.length >= this.maxWisdomQueue) {
      this.wisdomQueue.shift();
    }
    this.wisdomQueue.push(wisdom);
  }

  /**
   * Dequeue next wisdom item
   * @returns {Object|undefined} Next wisdom or undefined if empty
   */
  dequeueWisdom() {
    return this.wisdomQueue.shift();
  }

  /**
   * Add custom wisdom template
   * @param {string} signal - Context signal
   * @param {Object} template - Wisdom template
   */
  addWisdomTemplate(signal, template) {
    if (!this.wisdomTemplates[signal]) {
      this.wisdomTemplates[signal] = [];
    }
    this.wisdomTemplates[signal].push({
      type: template.type || WisdomType.SUGGESTION,
      message: template.message,
    });
  }

  /**
   * Learn from external knowledge
   * @param {string} key - Knowledge key
   * @param {*} value - Knowledge value
   */
  learn(key, value) {
    // Evict oldest entry if at capacity (skip if updating existing key)
    if (!this.knowledge.has(key) && this.knowledge.size >= this.maxKnowledge) {
      const oldest = this.knowledge.keys().next().value;
      if (oldest) this.knowledge.delete(oldest);
    }

    this.knowledge.set(key, {
      value,
      learnedAt: Date.now(),
      useCount: 0,
    });
  }

  /**
   * Recall knowledge
   * @param {string} key - Knowledge key
   * @returns {*} Knowledge value or undefined
   */
  recall(key) {
    const entry = this.knowledge.get(key);
    if (entry) {
      entry.useCount++;
      return entry.value;
    }
    return undefined;
  }

  /**
   * Get session context summary
   * @returns {Object} Context summary
   */
  getContextSummary() {
    return {
      duration: Date.now() - this.sessionContext.startTime,
      filesEdited: this.sessionContext.fileEdits.size,
      heaviestFile: this._detectHeavyFileEdits(),
      errorCount: this.sessionContext.errors.length,
      successCount: this.sessionContext.successes.length,
      topicsCount: this.sessionContext.topics.size,
      complexity: this.sessionContext.complexity,
    };
  }

  /**
   * Get shared wisdom history
   * @param {number} [limit] - Max items to return
   * @returns {Object[]} Wisdom history
   */
  getWisdomHistory(limit) {
    const items = [...this.sharedWisdom];
    return limit ? items.slice(-limit) : items;
  }

  /**
   * Get mentor summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      knowledgeEntries: this.knowledge.size,
      wisdomShared: this.sharedWisdom.length,
      sessionContext: this.getContextSummary(),
      recentWisdom: this.sharedWisdom.slice(-3),
    };
  }

  /**
   * Reset session context
   */
  resetSession() {
    this.sessionContext = {
      startTime: Date.now(),
      fileEdits: new Map(),
      errors: [],
      successes: [],
      topics: new Set(),
      complexity: 0,
    };
  }

  /**
   * Clear all state
   */
  clear() {
    this.knowledge.clear();
    this.wisdomQueue = [];
    this.sharedWisdom = [];
    this.resetSession();
  }
}

export default Mentor;
