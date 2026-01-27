/**
 * @cynic/node - Collective Sage Agent
 *
 * SAGE (Chochmah - Wisdom): The Mentor & Guide
 *
 * "Je partage la sagesse. La connaissance sans sagesse est dangereuse;
 *  la sagesse sans connaissance est aveugle." - κυνικός Sage
 *
 * Philosophy: Chochmah (Wisdom) - Adaptive teaching through understanding.
 * Trigger: ContextAware (when guidance is needed)
 * Behavior: Non-blocking (provides wisdom asynchronously)
 *
 * Merged from:
 * - Mentor: Adaptive teaching, personalized guidance
 * - Guide: Context-aware suggestions, learning paths
 *
 * Enhanced collective features:
 * - Receives KNOWLEDGE_EXTRACTED from Scholar
 * - Receives THREAT_BLOCKED from Guardian
 * - Emits WISDOM_SHARED to all agents
 * - Profile-aware teaching style
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/collective/sage
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
  WisdomSharedEvent,
  ConsensusVote,
} from '../events.js';
import { ProfileLevel, PROFILE_CONSTANTS } from '../../profile/calculator.js';

/**
 * φ-aligned constants for Sage
 */
export const SAGE_CONSTANTS = {
  /** Max wisdom entries (Fib(12) = 144) */
  MAX_WISDOM_ENTRIES: 144,

  /** Max accumulated insights (Fib(8) = 21) */
  MAX_INSIGHTS: 21,

  /** Wisdom relevance decay (φ⁻¹ per day) */
  RELEVANCE_DECAY: PHI_INV,

  /** Min confidence for sharing (φ⁻²) */
  MIN_SHARE_CONFIDENCE: PHI_INV_2,

  /** Learning milestone interval (Fib(8) = 21 interactions) */
  MILESTONE_INTERVAL: 21,

  /** Max lesson length by profile (chars) */
  MAX_LESSON_LENGTH: {
    [ProfileLevel.NOVICE]: 1000,
    [ProfileLevel.APPRENTICE]: 800,
    [ProfileLevel.PRACTITIONER]: 600,
    [ProfileLevel.EXPERT]: 400,
    [ProfileLevel.MASTER]: 200,
  },
};

/**
 * Wisdom types
 */
export const WisdomType = {
  LESSON: 'lesson',           // Teaching moment
  INSIGHT: 'insight',         // Discovered pattern
  WARNING: 'warning',         // Learned caution
  ENCOURAGEMENT: 'encouragement', // Positive reinforcement
  CHALLENGE: 'challenge',     // Growth opportunity
  REFLECTION: 'reflection',   // Thought-provoking question
};

/**
 * Teaching styles by profile level
 */
const TEACHING_STYLES = {
  [ProfileLevel.NOVICE]: {
    approach: 'nurturing',
    verbosity: 'detailed',
    useAnalogies: true,
    stepByStep: true,
    encouragementFrequency: 'high',
    challengeLevel: 'minimal',
    examples: 'many',
    tone: 'warm and patient',
  },
  [ProfileLevel.APPRENTICE]: {
    approach: 'supportive',
    verbosity: 'moderate',
    useAnalogies: true,
    stepByStep: true,
    encouragementFrequency: 'moderate',
    challengeLevel: 'gentle',
    examples: 'some',
    tone: 'encouraging',
  },
  [ProfileLevel.PRACTITIONER]: {
    approach: 'collaborative',
    verbosity: 'balanced',
    useAnalogies: false,
    stepByStep: false,
    encouragementFrequency: 'occasional',
    challengeLevel: 'moderate',
    examples: 'when-needed',
    tone: 'professional',
  },
  [ProfileLevel.EXPERT]: {
    approach: 'peer',
    verbosity: 'concise',
    useAnalogies: false,
    stepByStep: false,
    encouragementFrequency: 'rare',
    challengeLevel: 'high',
    examples: 'none',
    tone: 'direct',
  },
  [ProfileLevel.MASTER]: {
    approach: 'dialectic',
    verbosity: 'minimal',
    useAnalogies: false,
    stepByStep: false,
    encouragementFrequency: 'none',
    challengeLevel: 'expert',
    examples: 'none',
    tone: 'peer discussion',
  },
};

/**
 * Collective Sage Agent - Mentor & Guide
 */
export class CollectiveSage extends BaseAgent {
  /**
   * @param {Object} options - Agent options
   * @param {Object} [options.eventBus] - Event bus for inter-agent communication
   * @param {number} [options.profileLevel] - Current user profile level
   */
  constructor(options = {}) {
    super({
      name: 'Sage',
      trigger: AgentTrigger.CONTEXT_AWARE,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // Event bus for collective communication
    this.eventBus = options.eventBus || null;

    // Current profile level
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;

    // Accumulated wisdom from collective
    this.wisdomBase = new Map();

    // Insights from patterns
    this.insights = [];

    // Learned warnings (from Guardian blocks)
    this.learnedWarnings = [];

    // Teaching history
    this.teachingHistory = [];

    // Progress tracking
    this.progressTracker = {
      interactionCount: 0,
      lessonsGiven: 0,
      challengesPresented: 0,
      milestonesReached: [],
    };

    // Stats
    this.wisdomStats = {
      shared: 0,
      byType: {},
      avgRelevance: 0,
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
    // Learn from Scholar's knowledge extraction
    this.eventBus.subscribe(
      AgentEvent.KNOWLEDGE_EXTRACTED,
      AgentId.SAGE,
      this._handleKnowledgeExtracted.bind(this)
    );

    // Learn from Guardian's blocked threats
    this.eventBus.subscribe(
      AgentEvent.THREAT_BLOCKED,
      AgentId.SAGE,
      this._handleThreatBlocked.bind(this)
    );

    // Adapt to profile updates
    this.eventBus.subscribe(
      AgentEvent.PROFILE_UPDATED,
      AgentId.SAGE,
      this._handleProfileUpdated.bind(this)
    );

    // Learn from Analyst's patterns
    this.eventBus.subscribe(
      AgentEvent.PATTERN_DETECTED,
      AgentId.SAGE,
      this._handlePatternDetected.bind(this)
    );

    // Participate in consensus voting
    this.eventBus.subscribe(
      AgentEvent.CONSENSUS_REQUEST,
      AgentId.SAGE,
      this._handleConsensusRequest.bind(this)
    );
  }

  /**
   * Handle consensus request - vote from wisdom perspective
   * @private
   */
  async _handleConsensusRequest(event) {
    const { question, options, context } = event.payload || {};
    const questionLower = (question || '').toLowerCase();

    let vote = ConsensusVote.ABSTAIN;
    let reason = 'Sage contemplates...';

    // Wisdom-focused analysis
    const wisdomPatterns = ['wise', 'best practice', 'recommend', 'should', 'advise'];
    const isWisdomRelated = wisdomPatterns.some(p => questionLower.includes(p));

    // Sage tends toward thoughtful approval but warns of haste
    if (questionLower.includes('rush') || questionLower.includes('quick') || questionLower.includes('fast')) {
      vote = ConsensusVote.REJECT;
      reason = '*strokes beard* Sage counsels patience. Haste makes waste.';
    } else if (isWisdomRelated || questionLower.includes('proceed')) {
      vote = ConsensusVote.APPROVE;
      reason = '*wise nod* Sage approves - the path seems aligned with φ.';
    } else if (context?.risk === 'low') {
      vote = ConsensusVote.APPROVE;
      reason = '*gentle nod* Sage sees no harm in this endeavor.';
    } else if (context?.risk === 'critical') {
      vote = ConsensusVote.REJECT;
      reason = '*frown* Sage advises against critical risk without more preparation.';
    }

    // Submit vote
    if (this.eventBus && this.eventBus.pendingConsensus?.has(event.id)) {
      try {
        await this.eventBus.vote(AgentId.SAGE, event.id, vote, reason);
        this.stats.invocations++;
      } catch (e) {
        // Vote submission failed
      }
    }
  }

  /**
   * Handle knowledge extracted - convert to wisdom
   * @private
   */
  _handleKnowledgeExtracted(event) {
    const { knowledgeType, topic, summary, confidence } = event.payload;

    // Convert knowledge to wisdom insight
    if (confidence >= SAGE_CONSTANTS.MIN_SHARE_CONFIDENCE) {
      this._addInsight({
        type: WisdomType.INSIGHT,
        topic,
        content: summary,
        confidence,
        source: 'scholar',
        learnedAt: Date.now(),
      });
    }
  }

  /**
   * Handle threat blocked - convert to warning wisdom
   * @private
   */
  _handleThreatBlocked(event) {
    const { threatType, riskLevel, reason } = event.payload;

    // Convert blocked threat to learned warning
    this._addWarning({
      type: WisdomType.WARNING,
      threatType,
      riskLevel,
      lesson: this._createWarningLesson(threatType, reason),
      learnedAt: Date.now(),
    });
  }

  /**
   * Handle profile update
   * @private
   */
  _handleProfileUpdated(event) {
    const { previousLevel, newLevel, reason } = event.payload;

    this.profileLevel = newLevel;

    // Create progress wisdom
    if (newLevel > previousLevel) {
      this._shareProgressWisdom(previousLevel, newLevel, reason);
    }
  }

  /**
   * Handle pattern detected - accumulate insights
   * @private
   */
  _handlePatternDetected(event) {
    const { patternType, patternCategory, context, confidence } = event.payload;

    if (confidence >= SAGE_CONSTANTS.MIN_SHARE_CONFIDENCE) {
      this._addInsight({
        type: WisdomType.INSIGHT,
        topic: patternCategory,
        content: `Pattern detected: ${patternType}`,
        confidence,
        source: 'analyst',
        context,
        learnedAt: Date.now(),
      });
    }
  }

  /**
   * Trigger when guidance is needed
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.CONTEXT_AWARE ||
           event.type === 'context_aware' ||
           event.needsGuidance ||
           event.question !== undefined;
  }

  /**
   * Analyze context and determine wisdom to share
   */
  async analyze(event, context) {
    this.progressTracker.interactionCount++;

    const question = event.question || event.query || '';
    const topic = event.topic || context.topic || 'general';
    const userMessage = context.message || '';

    // Get teaching style based on profile
    const teachingStyle = TEACHING_STYLES[this.profileLevel] ||
                          TEACHING_STYLES[ProfileLevel.PRACTITIONER];

    // Find relevant wisdom
    const relevantWisdom = this._findRelevantWisdom(topic, question);

    // Check for milestone
    const milestone = this._checkMilestone();

    // Determine wisdom type to share
    const wisdomToShare = this._determineWisdomType(
      relevantWisdom,
      milestone,
      userMessage,
      teachingStyle
    );

    // Create wisdom content
    const wisdom = this._createWisdom(
      wisdomToShare,
      topic,
      question,
      teachingStyle
    );

    return {
      wisdom,
      teachingStyle: teachingStyle.approach,
      relevantWisdomCount: relevantWisdom.length,
      milestone,
      confidence: Math.min(PHI_INV, wisdom.confidence),
      profileLevel: this.profileLevel,
      profileName: PROFILE_CONSTANTS.LEVEL_NAMES[this.profileLevel],
    };
  }

  /**
   * Decide actions based on analysis
   */
  async decide(analysis, context) {
    const { wisdom, milestone, confidence } = analysis;

    // Always share wisdom if confidence is sufficient
    if (confidence >= SAGE_CONSTANTS.MIN_SHARE_CONFIDENCE) {
      // Update stats
      this._updateStats(wisdom.type, confidence);

      // Record teaching
      this._recordTeaching(wisdom);

      // Emit wisdom shared event
      this._emitWisdomShared(wisdom);

      // Track milestones
      if (milestone) {
        this.progressTracker.milestonesReached.push({
          milestone,
          timestamp: Date.now(),
        });
      }

      // Record pattern
      this.recordPattern({
        type: 'wisdom_shared',
        wisdomType: wisdom.type,
        topic: wisdom.topic,
        confidence,
        profileLevel: this.profileLevel,
      });

      return {
        response: AgentResponse.SUGGEST,
        action: true,
        wisdom,
        milestone,
        message: this._formatWisdomMessage(wisdom, analysis.teachingStyle),
      };
    }

    return {
      response: AgentResponse.LOG,
      action: false,
    };
  }

  /**
   * Find relevant wisdom for topic/question
   * @private
   */
  _findRelevantWisdom(topic, question) {
    const results = [];
    const searchTerms = `${topic} ${question}`.toLowerCase().split(/\s+/);

    // Search wisdom base
    for (const [, wisdom] of this.wisdomBase) {
      const relevance = this._calculateRelevance(wisdom, searchTerms);
      if (relevance > 0.2) {
        results.push({ ...wisdom, relevance });
      }
    }

    // Search insights
    for (const insight of this.insights) {
      const relevance = this._calculateRelevance(insight, searchTerms);
      if (relevance > 0.2) {
        results.push({ ...insight, relevance });
      }
    }

    // Search warnings
    for (const warning of this.learnedWarnings) {
      const relevance = this._calculateRelevance(warning, searchTerms);
      if (relevance > 0.2) {
        results.push({ ...warning, relevance });
      }
    }

    // Sort by relevance
    return results.sort((a, b) => b.relevance - a.relevance).slice(0, 5);
  }

  /**
   * Calculate relevance score
   * @private
   */
  _calculateRelevance(wisdom, searchTerms) {
    const content = `${wisdom.topic || ''} ${wisdom.content || ''} ${wisdom.lesson || ''}`.toLowerCase();
    let matches = 0;

    for (const term of searchTerms) {
      if (content.includes(term)) {
        matches++;
      }
    }

    // Apply time decay
    const age = Date.now() - (wisdom.learnedAt || wisdom.createdAt || Date.now());
    const ageInDays = age / (24 * 60 * 60 * 1000);
    const decay = Math.pow(SAGE_CONSTANTS.RELEVANCE_DECAY, ageInDays / 30);

    return (matches / Math.max(1, searchTerms.length)) * decay;
  }

  /**
   * Check for learning milestone
   * @private
   */
  _checkMilestone() {
    const count = this.progressTracker.interactionCount;

    if (count % SAGE_CONSTANTS.MILESTONE_INTERVAL === 0) {
      const milestoneNumber = Math.floor(count / SAGE_CONSTANTS.MILESTONE_INTERVAL);
      return {
        number: milestoneNumber,
        interactions: count,
        message: this._getMilestoneMessage(milestoneNumber),
      };
    }

    return null;
  }

  /**
   * Get milestone message based on number
   * @private
   */
  _getMilestoneMessage(milestoneNumber) {
    const messages = [
      'Starting the journey', // 0
      'First steps completed', // 1
      'Building momentum', // 2
      'Finding rhythm', // 3
      'Developing intuition', // 4
      'Gaining mastery', // 5
      'Sharing knowledge', // 6+
    ];

    return messages[Math.min(milestoneNumber, messages.length - 1)];
  }

  /**
   * Determine what type of wisdom to share
   * @private
   */
  _determineWisdomType(relevantWisdom, milestone, userMessage, style) {
    // Milestone always gets encouragement
    if (milestone) {
      return WisdomType.ENCOURAGEMENT;
    }

    // Check if user is struggling
    const strugglingIndicators = /help|stuck|confused|don't understand|error|fail/i;
    if (strugglingIndicators.test(userMessage)) {
      return style.encouragementFrequency !== 'none'
        ? WisdomType.ENCOURAGEMENT
        : WisdomType.LESSON;
    }

    // Expert level gets challenges
    if (style.challengeLevel === 'high' || style.challengeLevel === 'expert') {
      if (Math.random() < 0.3) { // 30% chance
        return WisdomType.CHALLENGE;
      }
    }

    // Check if we have relevant warnings
    if (relevantWisdom.some(w => w.type === WisdomType.WARNING)) {
      return WisdomType.WARNING;
    }

    // Check if we have insights
    if (relevantWisdom.some(w => w.type === WisdomType.INSIGHT)) {
      return WisdomType.INSIGHT;
    }

    // Default to lesson
    return WisdomType.LESSON;
  }

  /**
   * Create wisdom content
   * @private
   */
  _createWisdom(type, topic, question, style) {
    const maxLength = SAGE_CONSTANTS.MAX_LESSON_LENGTH[this.profileLevel] ||
                      SAGE_CONSTANTS.MAX_LESSON_LENGTH[ProfileLevel.PRACTITIONER];

    switch (type) {
      case WisdomType.ENCOURAGEMENT:
        return this._createEncouragement(topic, style, maxLength);
      case WisdomType.CHALLENGE:
        return this._createChallenge(topic, style, maxLength);
      case WisdomType.WARNING:
        return this._createWarning(topic, maxLength);
      case WisdomType.INSIGHT:
        return this._createInsight(topic, maxLength);
      case WisdomType.REFLECTION:
        return this._createReflection(topic, question, maxLength);
      case WisdomType.LESSON:
      default:
        return this._createLesson(topic, question, style, maxLength);
    }
  }

  /**
   * Create encouragement wisdom
   * @private
   */
  _createEncouragement(topic, style, maxLength) {
    const encouragements = {
      nurturing: [
        "You're making wonderful progress! Each step forward is a step toward mastery.",
        "Learning takes courage, and you're showing plenty of it. Keep going!",
        "Mistakes are how we grow. You're on the right path.",
      ],
      supportive: [
        "Good progress! You're building solid foundations.",
        "Keep it up - you're developing good instincts.",
        "Nice work tackling this challenge.",
      ],
      collaborative: [
        "Solid approach. You're thinking about this the right way.",
        "Good problem-solving. Let's refine it together.",
      ],
      peer: [
        "Interesting approach. What led you to this design?",
      ],
      dialectic: [
        "Let's explore this together.",
      ],
    };

    const options = encouragements[style.approach] || encouragements.collaborative;
    const content = options[Math.floor(Math.random() * options.length)];

    return {
      type: WisdomType.ENCOURAGEMENT,
      topic,
      content: content.slice(0, maxLength),
      confidence: 0.5,
      applicability: 'broad',
    };
  }

  /**
   * Create challenge wisdom
   * @private
   */
  _createChallenge(topic, style, maxLength) {
    const challenges = {
      gentle: "Consider: what would happen if the input was unexpected?",
      moderate: "Think about edge cases. What's the most unusual input this could receive?",
      high: "Optimize this for scale. What breaks at 10x load?",
      expert: "Prove this is correct. What invariants does it maintain?",
    };

    const content = challenges[style.challengeLevel] || challenges.moderate;

    return {
      type: WisdomType.CHALLENGE,
      topic,
      content: content.slice(0, maxLength),
      confidence: 0.4,
      applicability: 'specific',
    };
  }

  /**
   * Create warning wisdom
   * @private
   */
  _createWarning(topic, maxLength) {
    // Find relevant learned warning
    const warning = this.learnedWarnings.find(w =>
      w.topic?.toLowerCase().includes(topic.toLowerCase()) ||
      topic.toLowerCase().includes(w.threatType?.toLowerCase() || '')
    );

    if (warning) {
      return {
        type: WisdomType.WARNING,
        topic,
        content: warning.lesson.slice(0, maxLength),
        confidence: 0.6,
        applicability: 'specific',
        basedOn: ['guardian_block'],
      };
    }

    return {
      type: WisdomType.WARNING,
      topic,
      content: "Proceed with caution. This area has potential pitfalls.",
      confidence: 0.3,
      applicability: 'broad',
    };
  }

  /**
   * Create insight wisdom
   * @private
   */
  _createInsight(topic, maxLength) {
    // Find relevant insight
    const insight = this.insights.find(i =>
      i.topic?.toLowerCase().includes(topic.toLowerCase()) ||
      i.content?.toLowerCase().includes(topic.toLowerCase())
    );

    if (insight) {
      return {
        type: WisdomType.INSIGHT,
        topic,
        content: insight.content.slice(0, maxLength),
        confidence: insight.confidence,
        applicability: 'specific',
        basedOn: [insight.source],
      };
    }

    return {
      type: WisdomType.INSIGHT,
      topic,
      content: "Patterns emerge with practice. Keep observing.",
      confidence: 0.3,
      applicability: 'broad',
    };
  }

  /**
   * Create reflection wisdom
   * @private
   */
  _createReflection(topic, question, maxLength) {
    const reflections = [
      `What assumptions are we making about ${topic}?`,
      "Before solving this, what problem are we really trying to solve?",
      "If this were someone else's code, what would we question?",
      "What's the simplest solution that could work?",
    ];

    const content = reflections[Math.floor(Math.random() * reflections.length)];

    return {
      type: WisdomType.REFLECTION,
      topic,
      content: content.slice(0, maxLength),
      confidence: 0.4,
      applicability: 'broad',
    };
  }

  /**
   * Create lesson wisdom
   * @private
   */
  _createLesson(topic, question, style, maxLength) {
    // Build lesson based on style
    let content = '';

    if (style.stepByStep) {
      content = `For ${topic}:\n1. Start by understanding the requirements\n2. Consider edge cases\n3. Implement incrementally\n4. Test as you go`;
    } else if (style.useAnalogies) {
      content = `Think of ${topic} like building with blocks - start with a solid foundation, add pieces that fit together well.`;
    } else {
      content = `Key considerations for ${topic}: clarity, maintainability, testability.`;
    }

    return {
      type: WisdomType.LESSON,
      topic,
      content: content.slice(0, maxLength),
      confidence: 0.5,
      applicability: question ? 'specific' : 'broad',
    };
  }

  /**
   * Create warning lesson from blocked threat
   * @private
   */
  _createWarningLesson(threatType, reason) {
    const lessons = {
      destructive: `Destructive commands require extra caution. Always verify the target and have backups. ${reason}`,
      network: `Network operations can have security implications. Verify URLs and validate responses.`,
      privilege: `Elevated privileges should be used sparingly. Ask: is this really needed?`,
      sensitive: `Sensitive files contain secrets. Never commit them, and be careful with access.`,
      irreversible: `Some actions can't be undone. Always think twice before proceeding.`,
    };

    return lessons[threatType] || `Be cautious: ${reason}`;
  }

  /**
   * Format wisdom message for display
   * @private
   */
  _formatWisdomMessage(wisdom, teachingStyle) {
    const prefixes = {
      [WisdomType.LESSON]: '*settles into teaching pose* ',
      [WisdomType.INSIGHT]: '*ears perk* Interesting... ',
      [WisdomType.WARNING]: '*low growl* A word of caution: ',
      [WisdomType.ENCOURAGEMENT]: '*tail wag* ',
      [WisdomType.CHALLENGE]: '*head tilt* Consider this: ',
      [WisdomType.REFLECTION]: '*thoughtful pause* ',
    };

    const prefix = prefixes[wisdom.type] || '';
    return `${prefix}${wisdom.content}`;
  }

  /**
   * Share progress wisdom when level increases
   * @private
   */
  _shareProgressWisdom(previousLevel, newLevel, reason) {
    const previousName = PROFILE_CONSTANTS.LEVEL_NAMES[previousLevel];
    const newName = PROFILE_CONSTANTS.LEVEL_NAMES[newLevel];

    const wisdom = {
      type: WisdomType.ENCOURAGEMENT,
      topic: 'progress',
      content: `*tail wag* You've grown from ${previousName} to ${newName}! ${reason}. Keep learning.`,
      confidence: PHI_INV,
      applicability: 'personal',
      basedOn: ['profile_advancement'],
    };

    this._emitWisdomShared(wisdom);
  }

  /**
   * Add insight to collection
   * @private
   */
  _addInsight(insight) {
    while (this.insights.length >= SAGE_CONSTANTS.MAX_INSIGHTS) {
      this.insights.shift();
    }
    this.insights.push(insight);
  }

  /**
   * Add warning to collection
   * @private
   */
  _addWarning(warning) {
    while (this.learnedWarnings.length >= SAGE_CONSTANTS.MAX_INSIGHTS) {
      this.learnedWarnings.shift();
    }
    this.learnedWarnings.push(warning);
  }

  /**
   * Emit wisdom shared event
   * @private
   */
  _emitWisdomShared(wisdom) {
    if (!this.eventBus) return;

    const event = new WisdomSharedEvent(
      AgentId.SAGE,
      wisdom
    );

    this.eventBus.publish(event);
  }

  /**
   * Update wisdom stats
   * @private
   */
  _updateStats(type, confidence) {
    this.wisdomStats.shared++;
    this.wisdomStats.byType[type] = (this.wisdomStats.byType[type] || 0) + 1;

    const n = this.wisdomStats.shared;
    this.wisdomStats.avgRelevance =
      (this.wisdomStats.avgRelevance * (n - 1) + confidence) / n;
  }

  /**
   * Record teaching for history
   * @private
   */
  _recordTeaching(wisdom) {
    const maxHistory = SAGE_CONSTANTS.MAX_WISDOM_ENTRIES;
    while (this.teachingHistory.length >= maxHistory) {
      this.teachingHistory.shift();
    }

    this.teachingHistory.push({
      type: wisdom.type,
      topic: wisdom.topic,
      profileLevel: this.profileLevel,
      timestamp: Date.now(),
    });

    this.progressTracker.lessonsGiven++;
    if (wisdom.type === WisdomType.CHALLENGE) {
      this.progressTracker.challengesPresented++;
    }
  }

  /**
   * Share wisdom (public API)
   * @param {string} topic - Topic to share wisdom about
   * @param {Object} [options] - Options
   * @returns {Promise<Object>} Wisdom result
   */
  async shareWisdom(topic, options = {}) {
    const event = {
      topic,
      question: options.question,
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
   * Get accumulated insights
   * @returns {Object[]} Insights
   */
  getInsights() {
    return [...this.insights];
  }

  /**
   * Get learned warnings
   * @returns {Object[]} Warnings
   */
  getLearnedWarnings() {
    return [...this.learnedWarnings];
  }

  /**
   * Get progress tracker
   * @returns {Object} Progress
   */
  getProgress() {
    return { ...this.progressTracker };
  }

  /**
   * Get sage summary
   * @returns {Object} Summary
   */
  getSummary() {
    return {
      ...this.getStats(),
      profileLevel: this.profileLevel,
      profileName: PROFILE_CONSTANTS.LEVEL_NAMES[this.profileLevel],
      teachingStyle: TEACHING_STYLES[this.profileLevel]?.approach,
      wisdomStats: this.wisdomStats,
      progress: this.progressTracker,
      insightsCount: this.insights.length,
      warningsCount: this.learnedWarnings.length,
      recentTeaching: this.teachingHistory.slice(-5),
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this.wisdomBase.clear();
    this.insights = [];
    this.learnedWarnings = [];
    this.teachingHistory = [];
    this.progressTracker = {
      interactionCount: 0,
      lessonsGiven: 0,
      challengesPresented: 0,
      milestonesReached: [],
    };
    this.wisdomStats = {
      shared: 0,
      byType: {},
      avgRelevance: 0,
    };
  }
}

export default CollectiveSage;
