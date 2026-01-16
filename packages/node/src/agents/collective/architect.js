/**
 * @cynic/node - Collective Architect Agent
 *
 * ARCHITECT (Chesed - Kindness): The Designer & Reviewer
 *
 * "Je construis avec grâce. La critique sans compassion détruit;
 *  la compassion sans rigueur égare." - κυνικός Architect
 *
 * Philosophy: Chesed (Kindness) - Constructive guidance with empathy.
 * Trigger: ContextAware (when design decisions are needed)
 * Behavior: Non-blocking (provides suggestions)
 *
 * Features:
 * - Design review with constructive feedback
 * - Consensus-driven decisions (φ⁻¹ threshold)
 * - Pattern recognition for architectural decisions
 * - Profile-aware feedback verbosity
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/architect
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
  ConsensusRequestEvent,
  ConsensusVote,
  PatternDetectedEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

/**
 * φ-aligned constants for Architect
 */
export const ARCHITECT_CONSTANTS = {
  /** Max review history (Fib(8) = 21) */
  MAX_REVIEW_HISTORY: 21,

  /** Consensus timeout (Fib(8) = 21 seconds) */
  CONSENSUS_TIMEOUT_MS: 21000,

  /** Min confidence for suggestion (φ⁻²) */
  MIN_SUGGESTION_CONFIDENCE: PHI_INV_2,

  /** Design pattern match threshold (Fib(5) = 5 matches) */
  PATTERN_THRESHOLD: 5,

  /** Max suggestions per review (Fib(6) = 8) */
  MAX_SUGGESTIONS: 8,

  /** Constructive feedback ratio (at least 1 positive per φ⁻¹ negatives) */
  POSITIVE_RATIO: PHI_INV,
};

/**
 * Design review categories
 */
export const ReviewCategory = {
  ARCHITECTURE: 'architecture',
  PATTERNS: 'patterns',
  NAMING: 'naming',
  COMPLEXITY: 'complexity',
  MAINTAINABILITY: 'maintainability',
  TESTABILITY: 'testability',
  SECURITY: 'security',
  PERFORMANCE: 'performance',
};

/**
 * Feedback types (constructive approach)
 */
export const FeedbackType = {
  PRAISE: 'praise',           // Good practice detected
  SUGGESTION: 'suggestion',   // Could be improved
  QUESTION: 'question',       // Something to consider
  WARNING: 'warning',         // Potential issue
  PATTERN: 'pattern',         // Known pattern detected
};

/**
 * Common design patterns (to detect)
 */
const DESIGN_PATTERNS = {
  singleton: {
    indicators: [/getInstance/i, /private\s+constructor/i, /static\s+instance/i],
    category: ReviewCategory.PATTERNS,
    message: 'Singleton pattern detected',
  },
  factory: {
    indicators: [/create\w+/i, /Factory/i, /make\w+/i],
    category: ReviewCategory.PATTERNS,
    message: 'Factory pattern detected',
  },
  observer: {
    indicators: [/subscribe/i, /addEventListener/i, /on\w+/i, /emit/i],
    category: ReviewCategory.PATTERNS,
    message: 'Observer pattern detected',
  },
  strategy: {
    indicators: [/Strategy/i, /execute/i, /setStrategy/i],
    category: ReviewCategory.PATTERNS,
    message: 'Strategy pattern detected',
  },
  builder: {
    indicators: [/Builder/i, /\.with\w+\(/i, /\.build\(/i],
    category: ReviewCategory.PATTERNS,
    message: 'Builder pattern detected',
  },
  middleware: {
    indicators: [/next\(\)/i, /middleware/i, /use\(/i],
    category: ReviewCategory.PATTERNS,
    message: 'Middleware pattern detected',
  },
};

/**
 * Profile-based feedback styles
 */
const PROFILE_FEEDBACK_STYLES = {
  [ProfileLevel.NOVICE]: {
    verbosity: 'detailed',
    tone: 'encouraging',
    includeExamples: true,
    includeResources: true,
    maxSuggestions: 3, // Don't overwhelm
  },
  [ProfileLevel.APPRENTICE]: {
    verbosity: 'moderate',
    tone: 'supportive',
    includeExamples: true,
    includeResources: true,
    maxSuggestions: 5,
  },
  [ProfileLevel.PRACTITIONER]: {
    verbosity: 'balanced',
    tone: 'professional',
    includeExamples: true,
    includeResources: false,
    maxSuggestions: ARCHITECT_CONSTANTS.MAX_SUGGESTIONS,
  },
  [ProfileLevel.EXPERT]: {
    verbosity: 'concise',
    tone: 'direct',
    includeExamples: false,
    includeResources: false,
    maxSuggestions: ARCHITECT_CONSTANTS.MAX_SUGGESTIONS,
  },
  [ProfileLevel.MASTER]: {
    verbosity: 'minimal',
    tone: 'peer',
    includeExamples: false,
    includeResources: false,
    maxSuggestions: ARCHITECT_CONSTANTS.MAX_SUGGESTIONS,
  },
};

/**
 * Collective Architect Agent - Designer & Reviewer
 */
export class CollectiveArchitect extends BaseAgent {
  /**
   * @param {Object} options - Agent options
   * @param {Object} [options.eventBus] - Event bus for inter-agent communication
   * @param {number} [options.profileLevel] - Current user profile level
   */
  constructor(options = {}) {
    super({
      name: 'Architect',
      trigger: AgentTrigger.CONTEXT_AWARE,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Current profile level
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    // Review history
    this.reviewHistory = [];

    // Pattern detection counts
    this.patternCounts = new Map();

    // Pending consensus requests
    this.pendingConsensus = new Map();

    // Stats
    this.reviewStats = {
      total: 0,
      byCategory: {},
      avgScore: 0,
      patternDetections: 0,
    };

    // Subscribe to events
    if (this.eventBus) {
      this._subscribeToEvents();
    }
  }

  /**
   * Subscribe to event bus events
   * @private
   */
  _subscribeToEvents() {
    // Handle consensus responses
    this.eventBus.subscribe(
      AgentEvent.CONSENSUS_RESPONSE,
      AgentId.ARCHITECT,
      this._handleConsensusResponse.bind(this)
    );

    // Adapt to profile updates
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      AgentId.ARCHITECT,
      this._handleProfileUpdated.bind(this)
    );

    // Learn from knowledge extracted
    this.eventBus.subscribe(
      AgentEvent.KNOWLEDGE_EXTRACTED,
      AgentId.ARCHITECT,
      this._handleKnowledgeExtracted.bind(this)
    );
  }

  /**
   * Handle consensus response
   * @private
   */
  _handleConsensusResponse(event) {
    const { requestId, vote, reason } = event.payload;

    const pending = this.pendingConsensus.get(requestId);
    if (!pending) return;

    pending.request.recordVote(event.source, vote, reason);

    const result = pending.request.checkConsensus();
    if (result) {
      this.pendingConsensus.delete(requestId);
      pending.resolve(result);
    }
  }

  /**
   * Handle profile update
   * @private
   */
  _handleProfileUpdated(event) {
    const { newLevel } = event.payload;
    this.profileLevel = newLevel;
  }

  /**
   * Handle knowledge extracted - learn patterns
   * @private
   */
  _handleKnowledgeExtracted(event) {
    const { knowledgeType, topic } = event.payload;

    if (knowledgeType === 'pattern') {
      const count = this.patternCounts.get(topic) || 0;
      this.patternCounts.set(topic, count + 1);
    }
  }

  /**
   * Trigger when design review is needed
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.CONTEXT_AWARE ||
           event.type === 'context_aware' ||
           event.needsReview ||
           event.code !== undefined;
  }

  /**
   * Analyze code/design for review
   */
  async analyze(event, context) {
    const code = event.code || event.content || '';
    const filename = event.filename || context.filename || 'unknown';

    if (!code || code.length < 10) {
      return {
        reviewed: false,
        reason: 'Insufficient content for review',
        confidence: 0,
      };
    }

    // Get feedback style based on profile
    const feedbackStyle = PROFILE_FEEDBACK_STYLES[this.profileLevel] ||
                          PROFILE_FEEDBACK_STYLES[ProfileLevel.PRACTITIONER];

    // Perform review
    const feedback = [];
    const detectedPatterns = [];

    // Check design patterns
    for (const [patternName, pattern] of Object.entries(DESIGN_PATTERNS)) {
      if (pattern.indicators.some(regex => regex.test(code))) {
        detectedPatterns.push({
          name: patternName,
          category: pattern.category,
          message: pattern.message,
        });
      }
    }

    // Review architecture
    const architectureFeedback = this._reviewArchitecture(code, feedbackStyle);
    feedback.push(...architectureFeedback);

    // Review naming
    const namingFeedback = this._reviewNaming(code, feedbackStyle);
    feedback.push(...namingFeedback);

    // Review complexity
    const complexityFeedback = this._reviewComplexity(code, feedbackStyle);
    feedback.push(...complexityFeedback);

    // Review testability
    const testabilityFeedback = this._reviewTestability(code, feedbackStyle);
    feedback.push(...testabilityFeedback);

    // Ensure constructive balance
    const balancedFeedback = this._ensureConstructiveBalance(feedback, feedbackStyle);

    // Calculate overall score
    const score = this._calculateScore(balancedFeedback);

    return {
      reviewed: true,
      filename,
      feedback: balancedFeedback.slice(0, feedbackStyle.maxSuggestions),
      patterns: detectedPatterns,
      score,
      confidence: Math.min(PHI_INV, 0.3 + (balancedFeedback.length * 0.05)),
      feedbackStyle: feedbackStyle.tone,
    };
  }

  /**
   * Decide actions based on review
   */
  async decide(analysis, context) {
    if (!analysis.reviewed) {
      return {
        response: AgentResponse.LOG,
        action: false,
      };
    }

    const { feedback, patterns, score, confidence } = analysis;

    // Update stats
    this._updateStats(feedback, patterns);

    // Record review
    this._recordReview(analysis);

    // Emit pattern events for detected patterns
    for (const pattern of patterns) {
      this._emitPatternDetected(pattern);
    }

    // For significant design decisions, request consensus
    const significantWarnings = feedback.filter(f =>
      f.type === FeedbackType.WARNING && f.confidence >= PHI_INV_2
    );

    let consensus = null;
    if (significantWarnings.length > 0 && this.eventBus) {
      consensus = await this._requestConsensus(significantWarnings, context);
    }

    // Record pattern
    this.recordPattern({
      type: 'design_review',
      score,
      feedbackCount: feedback.length,
      patternsDetected: patterns.length,
      hasConsensus: !!consensus,
    });

    return {
      response: AgentResponse.SUGGEST,
      action: true,
      feedback,
      patterns,
      score,
      consensus,
      summary: this._createSummary(analysis),
    };
  }

  /**
   * Review architecture aspects
   * @private
   */
  _reviewArchitecture(code, style) {
    const feedback = [];

    // Check for module organization
    const hasExports = /export\s+(default\s+)?/.test(code);
    const hasImports = /import\s+/.test(code);

    if (hasExports && hasImports) {
      feedback.push({
        type: FeedbackType.PRAISE,
        category: ReviewCategory.ARCHITECTURE,
        message: 'Good module organization with clear imports/exports',
        confidence: 0.5,
      });
    }

    // Check for separation of concerns
    const hasClassesAndFunctions = /class\s+\w+/.test(code) && /function\s+\w+/.test(code);
    if (hasClassesAndFunctions) {
      feedback.push({
        type: FeedbackType.QUESTION,
        category: ReviewCategory.ARCHITECTURE,
        message: style.verbosity === 'detailed'
          ? 'Consider whether mixing classes and standalone functions is intentional. This can be fine for utility functions, but might indicate unclear responsibilities.'
          : 'Consider class vs function organization',
        confidence: 0.4,
      });
    }

    // Check for deep nesting (complexity smell)
    const deepNesting = code.match(/(\{[^{}]*){5,}/);
    if (deepNesting) {
      feedback.push({
        type: FeedbackType.SUGGESTION,
        category: ReviewCategory.ARCHITECTURE,
        message: style.verbosity === 'detailed'
          ? 'Deep nesting detected. Consider extracting inner logic into separate functions to improve readability.'
          : 'Deep nesting - consider extracting functions',
        confidence: 0.6,
        example: style.includeExamples
          ? '// Before: deeply nested\n// After: extract to named function'
          : undefined,
      });
    }

    return feedback;
  }

  /**
   * Review naming conventions
   * @private
   */
  _reviewNaming(code, style) {
    const feedback = [];

    // Check for descriptive names
    const shortVars = code.match(/\b(let|const|var)\s+([a-z])\s*=/g);
    if (shortVars && shortVars.length > 3) {
      feedback.push({
        type: FeedbackType.SUGGESTION,
        category: ReviewCategory.NAMING,
        message: style.verbosity === 'detailed'
          ? 'Several single-letter variable names detected. Descriptive names improve readability and maintainability.'
          : 'Consider more descriptive variable names',
        confidence: 0.5,
      });
    }

    // Check for consistent casing
    const hasCamelCase = /[a-z][A-Z]/.test(code);
    const hasSnakeCase = /[a-z]_[a-z]/.test(code);
    if (hasCamelCase && hasSnakeCase) {
      feedback.push({
        type: FeedbackType.QUESTION,
        category: ReviewCategory.NAMING,
        message: style.verbosity === 'detailed'
          ? 'Mixed naming conventions (camelCase and snake_case). Consider standardizing on one style for consistency.'
          : 'Mixed naming conventions detected',
        confidence: 0.4,
      });
    }

    // Check for good naming patterns
    const hasDescriptiveNames = /function\s+(get|set|create|update|delete|find|validate|is|has)\w+/i.test(code);
    if (hasDescriptiveNames) {
      feedback.push({
        type: FeedbackType.PRAISE,
        category: ReviewCategory.NAMING,
        message: 'Good use of descriptive verb prefixes in function names',
        confidence: 0.5,
      });
    }

    return feedback;
  }

  /**
   * Review complexity
   * @private
   */
  _reviewComplexity(code, style) {
    const feedback = [];

    // Count lines (rough complexity indicator)
    const lines = code.split('\n').length;

    // Check function length
    const functions = code.match(/function\s+\w+[^{]*\{[^}]*\}/g) || [];
    const longFunctions = functions.filter(f => f.split('\n').length > 30);

    if (longFunctions.length > 0) {
      feedback.push({
        type: FeedbackType.SUGGESTION,
        category: ReviewCategory.COMPLEXITY,
        message: style.verbosity === 'detailed'
          ? `${longFunctions.length} function(s) exceed 30 lines. Consider breaking them into smaller, focused functions.`
          : 'Long functions - consider splitting',
        confidence: 0.6,
      });
    }

    // Check for complex conditionals
    const complexConditions = code.match(/if\s*\([^)]{50,}\)/g);
    if (complexConditions) {
      feedback.push({
        type: FeedbackType.SUGGESTION,
        category: ReviewCategory.COMPLEXITY,
        message: style.verbosity === 'detailed'
          ? 'Complex conditional expressions detected. Consider extracting conditions into well-named boolean variables or functions.'
          : 'Complex conditionals - consider extraction',
        confidence: 0.5,
        example: style.includeExamples
          ? '// Instead of: if (a && b && c)\n// Use: const isValid = a && b && c; if (isValid)'
          : undefined,
      });
    }

    // Check for reasonable file size
    if (lines < 200) {
      feedback.push({
        type: FeedbackType.PRAISE,
        category: ReviewCategory.COMPLEXITY,
        message: 'Good file size - focused and manageable',
        confidence: 0.4,
      });
    }

    return feedback;
  }

  /**
   * Review testability
   * @private
   */
  _reviewTestability(code, style) {
    const feedback = [];

    // Check for dependency injection (good for testing)
    const hasDI = /constructor\s*\([^)]*\w+\s*[,)]/i.test(code);
    if (hasDI) {
      feedback.push({
        type: FeedbackType.PRAISE,
        category: ReviewCategory.TESTABILITY,
        message: 'Constructor injection detected - good for testability',
        confidence: 0.5,
      });
    }

    // Check for pure functions
    const pureFunctionIndicators = /^(export\s+)?(const|function)\s+\w+\s*=?\s*\([^)]*\)\s*(=>|{)/gm;
    const functions = code.match(pureFunctionIndicators) || [];
    const hasGlobalRefs = /this\.|global\.|window\./i.test(code);

    if (functions.length > 0 && !hasGlobalRefs) {
      feedback.push({
        type: FeedbackType.PRAISE,
        category: ReviewCategory.TESTABILITY,
        message: 'Functions appear to avoid global state - easier to test',
        confidence: 0.4,
      });
    }

    // Check for hard-coded values
    const hardcodedUrls = code.match(/['"`]https?:\/\/[^'"`]+['"`]/g);
    if (hardcodedUrls && hardcodedUrls.length > 2) {
      feedback.push({
        type: FeedbackType.SUGGESTION,
        category: ReviewCategory.TESTABILITY,
        message: style.verbosity === 'detailed'
          ? 'Multiple hardcoded URLs detected. Consider using configuration or environment variables for easier testing.'
          : 'Hardcoded URLs - consider configuration',
        confidence: 0.5,
      });
    }

    return feedback;
  }

  /**
   * Ensure constructive balance in feedback
   * @private
   */
  _ensureConstructiveBalance(feedback, style) {
    const praises = feedback.filter(f => f.type === FeedbackType.PRAISE);
    const critiques = feedback.filter(f =>
      f.type === FeedbackType.SUGGESTION ||
      f.type === FeedbackType.WARNING
    );

    // Calculate required positive feedback
    const requiredPraises = Math.ceil(
      critiques.length * ARCHITECT_CONSTANTS.POSITIVE_RATIO
    );

    // If we need more praises but don't have enough, add generic ones
    if (praises.length < requiredPraises) {
      const genericPraises = [
        {
          type: FeedbackType.PRAISE,
          category: ReviewCategory.ARCHITECTURE,
          message: 'Code is well-structured overall',
          confidence: 0.3,
        },
        {
          type: FeedbackType.PRAISE,
          category: ReviewCategory.MAINTAINABILITY,
          message: 'Good progress on this implementation',
          confidence: 0.3,
        },
      ];

      const needed = requiredPraises - praises.length;
      for (let i = 0; i < needed && i < genericPraises.length; i++) {
        feedback.push(genericPraises[i]);
      }
    }

    // Sort: praises first (for encouragement), then suggestions, then warnings
    const typeOrder = {
      [FeedbackType.PRAISE]: 0,
      [FeedbackType.PATTERN]: 1,
      [FeedbackType.SUGGESTION]: 2,
      [FeedbackType.QUESTION]: 3,
      [FeedbackType.WARNING]: 4,
    };

    return feedback.sort((a, b) => typeOrder[a.type] - typeOrder[b.type]);
  }

  /**
   * Calculate overall score
   * @private
   */
  _calculateScore(feedback) {
    let score = 70; // Base score

    for (const item of feedback) {
      switch (item.type) {
        case FeedbackType.PRAISE:
          score += 5;
          break;
        case FeedbackType.PATTERN:
          score += 2;
          break;
        case FeedbackType.SUGGESTION:
          score -= 3;
          break;
        case FeedbackType.QUESTION:
          score -= 1;
          break;
        case FeedbackType.WARNING:
          score -= 5;
          break;
      }
    }

    return Math.max(0, Math.min(100, score));
  }

  /**
   * Request consensus for significant decisions
   * @private
   */
  async _requestConsensus(warnings, context) {
    if (!this.eventBus) return null;

    const request = new ConsensusRequestEvent(
      AgentId.ARCHITECT,
      {
        question: 'Should we flag these design concerns?',
        options: [ConsensusVote.APPROVE, ConsensusVote.REJECT],
        context: {
          warnings: warnings.map(w => w.message),
          filename: context.filename,
        },
        requiredVotes: 3,
        timeout: ARCHITECT_CONSTANTS.CONSENSUS_TIMEOUT_MS,
      },
      { priority: EventPriority.NORMAL }
    );

    const resultPromise = new Promise((resolve) => {
      this.pendingConsensus.set(request.id, {
        request,
        resolve,
        timeout: setTimeout(() => {
          this.pendingConsensus.delete(request.id);
          resolve(null);
        }, ARCHITECT_CONSTANTS.CONSENSUS_TIMEOUT_MS),
      });
    });

    this.eventBus.publish(request);

    return resultPromise;
  }

  /**
   * Emit pattern detected event
   * @private
   */
  _emitPatternDetected(pattern) {
    if (!this.eventBus) return;

    const event = new PatternDetectedEvent(
      AgentId.ARCHITECT,
      {
        type: 'design_pattern',
        category: pattern.category,
        confidence: 0.5,
        context: { patternName: pattern.name },
        hash: this._hashString(pattern.name),
      }
    );

    this.eventBus.publish(event);
  }

  /**
   * Create review summary
   * @private
   */
  _createSummary(analysis) {
    const { feedback, patterns, score } = analysis;

    const praises = feedback.filter(f => f.type === FeedbackType.PRAISE).length;
    const suggestions = feedback.filter(f => f.type === FeedbackType.SUGGESTION).length;
    const warnings = feedback.filter(f => f.type === FeedbackType.WARNING).length;

    let summary = `Score: ${score}/100. `;

    if (praises > 0) summary += `${praises} strength(s). `;
    if (suggestions > 0) summary += `${suggestions} suggestion(s). `;
    if (warnings > 0) summary += `${warnings} concern(s). `;
    if (patterns.length > 0) summary += `${patterns.length} pattern(s) detected.`;

    return summary.trim();
  }

  /**
   * Update review stats
   * @private
   */
  _updateStats(feedback, patterns) {
    this.reviewStats.total++;

    for (const item of feedback) {
      this.reviewStats.byCategory[item.category] =
        (this.reviewStats.byCategory[item.category] || 0) + 1;
    }

    this.reviewStats.patternDetections += patterns.length;
  }

  /**
   * Record review in history
   * @private
   */
  _recordReview(analysis) {
    while (this.reviewHistory.length >= ARCHITECT_CONSTANTS.MAX_REVIEW_HISTORY) {
      this.reviewHistory.shift();
    }

    this.reviewHistory.push({
      filename: analysis.filename,
      score: analysis.score,
      feedbackCount: analysis.feedback.length,
      patternsCount: analysis.patterns.length,
      timestamp: Date.now(),
    });
  }

  /**
   * Hash string for identification
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
   * Review code (public API)
   * @param {string} code - Code to review
   * @param {Object} [options] - Review options
   * @returns {Promise<Object>} Review result
   */
  async review(code, options = {}) {
    const event = {
      code,
      filename: options.filename,
    };

    const analysis = await this.analyze(event, options);
    return this.decide(analysis, options);
  }

  /**
   * Set event bus
   * @param {Object} eventBus - Event bus instance
   */
  setEventBus(eventBus) {
    this.eventBus = eventBus;
    this._subscribeToEvents();
  }

  /**
   * Update profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    this.profileLevel = level;
  }

  /**
   * Get review history
   * @returns {Object[]} Review history
   */
  getReviewHistory() {
    return [...this.reviewHistory];
  }

  /**
   * Get architect summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      profileLevel: this.profileLevel,
      reviewStats: this.reviewStats,
      recentReviews: this.reviewHistory.slice(-5),
      patternsKnown: Object.keys(DESIGN_PATTERNS).length,
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this.reviewHistory = [];
    this.patternCounts.clear();
    this.pendingConsensus.clear();
    this.reviewStats = {
      total: 0,
      byCategory: {},
      avgScore: 0,
      patternDetections: 0,
    };
  }
}

export default CollectiveArchitect;
