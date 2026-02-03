/**
 * Dog Learning Service - L3 Learning Loop
 *
 * Wires L2 judgments back to L1 Dog heuristics.
 * When LLM judgments reveal patterns, feeds them back to Dog patterns.json.
 *
 * "Le chien apprend en écoutant la meute" - κυνικός
 *
 * Flow:
 *   L1: Dog patterns.json (instant check)
 *   L2: LLM judgment (expensive, thorough)
 *   L3: Learning loop (L2 → L1 feedback)
 *
 * @module @cynic/node/dogs/learning-service
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';
import { addLearnedPattern, DOGS, loadDogPatterns } from './index.js';

const log = createLogger('DogLearning');

/**
 * φ-aligned learning constants
 */
const LEARNING_CONFIG = Object.freeze({
  // Minimum observations before learning
  MIN_OBSERVATIONS: 3,

  // Minimum Q-Score delta to trigger learning
  MIN_SCORE_DELTA: PHI_INV_2 * 100, // 38.2% significant

  // Maximum confidence for learned patterns
  MAX_LEARNED_CONFIDENCE: PHI_INV, // 61.8%

  // Decay rate for pattern relevance
  DECAY_RATE: PHI_INV_2, // 38.2%

  // Maximum learned patterns per Dog
  MAX_LEARNED_PER_DOG: 21, // Fib(8)

  // Time window for pattern correlation (ms)
  CORRELATION_WINDOW_MS: 89 * 1000, // Fib(11) seconds
});

/**
 * Pattern observation for learning correlation
 */
class PatternObservation {
  constructor(dogId, pattern, verdict, qScore, context) {
    this.dogId = dogId;
    this.pattern = pattern;
    this.verdict = verdict;
    this.qScore = qScore;
    this.context = context;
    this.timestamp = Date.now();
    this.count = 1;
  }

  /**
   * Check if observation is recent
   */
  isRecent(windowMs = LEARNING_CONFIG.CORRELATION_WINDOW_MS) {
    return Date.now() - this.timestamp < windowMs;
  }

  /**
   * Increment observation count
   */
  increment() {
    this.count++;
    this.timestamp = Date.now();
  }
}

/**
 * Dog Learning Service
 *
 * Observes judgment outcomes and learns patterns for Dogs.
 */
export class DogLearningService extends EventEmitter {
  /**
   * @param {Object} options
   * @param {EventEmitter} options.eventBus - Event bus to subscribe to
   * @param {boolean} [options.enabled=true] - Enable learning
   * @param {boolean} [options.persistImmediately=true] - Persist learned patterns immediately
   */
  constructor(options = {}) {
    super();

    this.eventBus = options.eventBus || null;
    this.enabled = options.enabled !== false;
    this.persistImmediately = options.persistImmediately !== false;

    // Pattern observation buffer: key → PatternObservation
    this._observations = new Map();

    // Learned patterns waiting for confirmation
    this._pendingLearning = new Map();

    // Stats
    this.stats = {
      judgmentsObserved: 0,
      patternsLearned: 0,
      patternsFalsified: 0,
      lastLearning: null,
      byDog: {},
    };

    // Initialize stats per Dog
    for (const dog of DOGS) {
      this.stats.byDog[dog.id] = { learned: 0, falsified: 0 };
    }

    // Bind handlers
    this._onJudgmentCreated = this._onJudgmentCreated.bind(this);
    this._onCollectiveVote = this._onCollectiveVote.bind(this);
  }

  /**
   * Start the learning service
   */
  start() {
    if (!this.eventBus) {
      log.warn('DogLearningService: No event bus provided');
      return;
    }

    // Subscribe to judgment events
    this.eventBus.on('judgment:created', this._onJudgmentCreated);
    this.eventBus.on('collective:vote:completed', this._onCollectiveVote);

    log.info('DogLearningService started', { enabled: this.enabled });
    this.emit('started');
  }

  /**
   * Stop the learning service
   */
  stop() {
    if (this.eventBus) {
      this.eventBus.off('judgment:created', this._onJudgmentCreated);
      this.eventBus.off('collective:vote:completed', this._onCollectiveVote);
    }

    log.info('DogLearningService stopped', { stats: this.stats });
    this.emit('stopped');
  }

  /**
   * Handle judgment created event
   * @private
   */
  _onJudgmentCreated(event) {
    if (!this.enabled) return;

    this.stats.judgmentsObserved++;

    const { judgmentId, verdict, qScore, item, confidence } = event;

    // Extract learnable patterns based on verdict and item
    this._extractPatterns(judgmentId, verdict, qScore, item, confidence);
  }

  /**
   * Handle collective vote completion
   * @private
   */
  _onCollectiveVote(event) {
    if (!this.enabled) return;

    const { judgmentId, votes, consensus, dogVotes } = event;

    // Learn from Dog voting patterns
    if (dogVotes && Array.isArray(dogVotes)) {
      for (const vote of dogVotes) {
        this._learnFromDogVote(vote, consensus);
      }
    }
  }

  /**
   * Extract learnable patterns from a judgment
   * @private
   */
  _extractPatterns(judgmentId, verdict, qScore, item, confidence) {
    // Determine which Dog(s) should learn from this judgment
    const relevantDogs = this._identifyRelevantDogs(item, verdict);

    for (const dogId of relevantDogs) {
      const pattern = this._extractPatternForDog(dogId, item, verdict, qScore);

      if (pattern) {
        const key = `${dogId}:${pattern.signature}`;

        // Check if we've seen this pattern before
        const existing = this._observations.get(key);

        if (existing && existing.isRecent()) {
          existing.increment();

          // Check if we have enough observations to learn
          if (existing.count >= LEARNING_CONFIG.MIN_OBSERVATIONS) {
            this._proposePatternLearning(dogId, pattern, existing);
          }
        } else {
          // New observation
          this._observations.set(key, new PatternObservation(
            dogId,
            pattern,
            verdict,
            qScore,
            { judgmentId, item }
          ));
        }
      }
    }

    // Cleanup old observations
    this._cleanupObservations();
  }

  /**
   * Identify which Dogs should learn from this judgment
   * @private
   */
  _identifyRelevantDogs(item, verdict) {
    const dogs = [];

    if (!item) return dogs;

    const itemType = item.type?.toLowerCase() || '';

    // Guardian learns from security-related judgments
    if (
      itemType.includes('security') ||
      itemType.includes('bash') ||
      itemType.includes('command') ||
      verdict === 'BARK'
    ) {
      dogs.push('guardian');
    }

    // Analyst learns from quality/metrics judgments
    if (
      itemType.includes('quality') ||
      itemType.includes('code') ||
      itemType.includes('metric')
    ) {
      dogs.push('analyst');
    }

    // Janitor learns from complexity judgments
    if (
      itemType.includes('complexity') ||
      itemType.includes('refactor') ||
      itemType.includes('dead_code')
    ) {
      dogs.push('janitor');
    }

    // Architect learns from structural decisions
    if (
      itemType.includes('architecture') ||
      itemType.includes('design') ||
      itemType.includes('pattern')
    ) {
      dogs.push('architect');
    }

    // Scholar learns from factual/verification judgments
    if (
      itemType.includes('fact') ||
      itemType.includes('claim') ||
      itemType.includes('verification')
    ) {
      dogs.push('scholar');
    }

    // Scout learns from exploration/search patterns
    if (
      itemType.includes('search') ||
      itemType.includes('explore') ||
      itemType.includes('navigation')
    ) {
      dogs.push('scout');
    }

    return dogs;
  }

  /**
   * Extract a pattern specific to a Dog's domain
   * @private
   */
  _extractPatternForDog(dogId, item, verdict, qScore) {
    if (!item || !item.identifier) return null;

    const basePattern = {
      signature: `${item.type}:${item.identifier}`.toLowerCase(),
      type: item.type,
      verdict,
      qScore,
      confidence: Math.min(LEARNING_CONFIG.MAX_LEARNED_CONFIDENCE, qScore / 100),
    };

    switch (dogId) {
      case 'guardian':
        return this._extractGuardianPattern(basePattern, item);
      case 'analyst':
        return this._extractAnalystPattern(basePattern, item);
      case 'janitor':
        return this._extractJanitorPattern(basePattern, item);
      case 'architect':
        return this._extractArchitectPattern(basePattern, item);
      case 'scholar':
        return this._extractScholarPattern(basePattern, item);
      case 'scout':
        return this._extractScoutPattern(basePattern, item);
      default:
        return basePattern;
    }
  }

  /**
   * Extract Guardian-specific pattern (security)
   * @private
   */
  _extractGuardianPattern(base, item) {
    return {
      ...base,
      category: 'security',
      dangerLevel: base.verdict === 'BARK' ? 'critical'
        : base.verdict === 'GROWL' ? 'high'
        : base.verdict === 'WAG' ? 'medium'
        : 'low',
    };
  }

  /**
   * Extract Analyst-specific pattern (quality)
   * @private
   */
  _extractAnalystPattern(base, item) {
    return {
      ...base,
      category: 'quality',
      metricType: item.identifier?.includes('coverage') ? 'coverage'
        : item.identifier?.includes('complexity') ? 'complexity'
        : 'general',
    };
  }

  /**
   * Extract Janitor-specific pattern (complexity)
   * @private
   */
  _extractJanitorPattern(base, item) {
    return {
      ...base,
      category: 'complexity',
      smellType: item.identifier?.includes('dead') ? 'dead_code'
        : item.identifier?.includes('duplicate') ? 'duplication'
        : 'complexity',
    };
  }

  /**
   * Extract Architect-specific pattern (design)
   * @private
   */
  _extractArchitectPattern(base, item) {
    return {
      ...base,
      category: 'design',
      patternType: item.identifier?.includes('anti') ? 'antipattern'
        : 'pattern',
    };
  }

  /**
   * Extract Scholar-specific pattern (verification)
   * @private
   */
  _extractScholarPattern(base, item) {
    return {
      ...base,
      category: 'verification',
      claimType: item.identifier?.includes('fact') ? 'factual'
        : item.identifier?.includes('opinion') ? 'opinion'
        : 'claim',
    };
  }

  /**
   * Extract Scout-specific pattern (exploration)
   * @private
   */
  _extractScoutPattern(base, item) {
    return {
      ...base,
      category: 'exploration',
      searchType: item.identifier?.includes('definition') ? 'definition'
        : item.identifier?.includes('usage') ? 'usage'
        : 'general',
    };
  }

  /**
   * Propose learning a pattern (with verification)
   * @private
   */
  _proposePatternLearning(dogId, pattern, observation) {
    const key = `${dogId}:${pattern.signature}`;

    // Check if Dog has L1 capability
    const dog = DOGS.find(d => d.id === dogId);
    if (!dog || !dog.hasL1) {
      log.trace(`Dog ${dogId} has no L1 - skipping learning`);
      return;
    }

    // Check existing patterns to avoid duplicates
    const existingPatterns = loadDogPatterns(dogId);
    if (existingPatterns?.learnedPatterns) {
      const isDuplicate = existingPatterns.learnedPatterns.some(
        p => p.signature === pattern.signature
      );
      if (isDuplicate) {
        log.trace(`Pattern already learned for ${dogId}`, { signature: pattern.signature });
        return;
      }

      // Check max limit
      if (existingPatterns.learnedPatterns.length >= LEARNING_CONFIG.MAX_LEARNED_PER_DOG) {
        log.warn(`Max learned patterns reached for ${dogId}`);
        return;
      }
    }

    // Learn the pattern
    const success = addLearnedPattern(dogId, {
      ...pattern,
      observationCount: observation.count,
      source: 'L3_learning_loop',
    });

    if (success) {
      this.stats.patternsLearned++;
      this.stats.byDog[dogId].learned++;
      this.stats.lastLearning = new Date();

      log.info(`Pattern learned for ${dogId}`, {
        signature: pattern.signature,
        verdict: pattern.verdict,
        observations: observation.count,
      });

      this.emit('pattern:learned', {
        dogId,
        pattern,
        observations: observation.count,
      });

      // Clear observation after learning
      this._observations.delete(key);
    }
  }

  /**
   * Learn from a Dog's vote in collective consensus
   * @private
   */
  _learnFromDogVote(vote, consensus) {
    const { dogId, decision, confidence, reasoning } = vote;

    // If Dog disagrees with consensus, consider falsification
    const dogAgreed = (decision === 'approve') === (consensus.verdict !== 'BARK');

    if (!dogAgreed && confidence > LEARNING_CONFIG.MIN_SCORE_DELTA / 100) {
      // Dog was wrong with high confidence - potential pattern falsification
      this.stats.patternsFalsified++;
      this.stats.byDog[dogId].falsified++;

      log.debug(`Dog ${dogId} disagreed with consensus`, {
        dogDecision: decision,
        consensusVerdict: consensus.verdict,
        confidence,
      });

      this.emit('pattern:falsified', {
        dogId,
        consensus,
        vote,
      });
    }
  }

  /**
   * Cleanup old observations
   * @private
   */
  _cleanupObservations() {
    const cutoff = Date.now() - LEARNING_CONFIG.CORRELATION_WINDOW_MS;

    for (const [key, obs] of this._observations.entries()) {
      if (obs.timestamp < cutoff) {
        this._observations.delete(key);
      }
    }
  }

  /**
   * Get learning statistics
   */
  getStats() {
    return {
      ...this.stats,
      observationBufferSize: this._observations.size,
      config: LEARNING_CONFIG,
    };
  }

  /**
   * Get learned patterns for a Dog
   */
  getLearnedPatterns(dogId) {
    const patterns = loadDogPatterns(dogId);
    return patterns?.learnedPatterns || [];
  }

  /**
   * Manually trigger learning for testing
   */
  async triggerLearning(dogId, pattern) {
    const observation = new PatternObservation(
      dogId,
      pattern,
      pattern.verdict,
      pattern.qScore * 100,
      { manual: true }
    );
    observation.count = LEARNING_CONFIG.MIN_OBSERVATIONS;

    this._proposePatternLearning(dogId, pattern, observation);
  }
}

/**
 * Create DogLearningService instance
 */
export function createDogLearningService(options = {}) {
  return new DogLearningService(options);
}

export default DogLearningService;
