/**
 * Ambient Consensus - Automatic Dog Voting
 *
 * "Le pack décide ensemble" - CYNIC
 *
 * Triggers consensus automatically when:
 * - Confidence < φ⁻¹ (61.8%)
 * - Dangerous operations detected
 * - Significant decisions needed
 *
 * @module @cynic/node/agents/collective/ambient-consensus
 */

'use strict';

import { createLogger, PHI_INV, PHI_INV_2, PHI_INV_3, globalEventBus, EventType as CoreEventType } from '@cynic/core';
import { EventType, getEventBus } from '../../services/event-bus.js';

// Math modules for intelligent consensus
import { BetaDistribution } from '../../inference/bayes.js';
import { createMarkovChain } from '../../inference/markov.js';
import { computeStats, zScore } from '../../inference/gaussian.js';
import { entropyConfidence, normalizedEntropy } from '../../inference/entropy.js';

const log = createLogger('AmbientConsensus');

// ═══════════════════════════════════════════════════════════════════════════
// φ-ALIGNED THRESHOLDS
// ═══════════════════════════════════════════════════════════════════════════

export const CONSENSUS_THRESHOLDS = {
  LOW_CONFIDENCE: PHI_INV,        // 61.8% - trigger consensus
  CRITICAL_CONFIDENCE: PHI_INV_2, // 38.2% - always trigger
  AGREEMENT_THRESHOLD: PHI_INV,   // 61.8% - needed to pass
  VOTE_TIMEOUT_MS: 5000,          // 5 seconds timeout
  MIN_VOTERS: 3,                  // Fib(4) = 3 minimum voters
};

// ═══════════════════════════════════════════════════════════════════════════
// INTER-DOG SIGNAL TYPES
// ═══════════════════════════════════════════════════════════════════════════

export const DogSignal = {
  DANGER_DETECTED: 'dog:danger_detected',      // Guardian → All
  PATTERN_FOUND: 'dog:pattern_found',          // Analyst → Scout, Sage
  RECOMMENDATION: 'dog:recommendation',         // Sage → All
  ANALYSIS_COMPLETE: 'dog:analysis_complete',  // Analyst → Scholar
  CONSENSUS_NEEDED: 'dog:consensus_needed',    // Any → All
  WISDOM_SHARED: 'dog:wisdom_shared',          // Sage → All
  EXPLORATION_RESULT: 'dog:exploration_result', // Scout → Cartographer
};

// ═══════════════════════════════════════════════════════════════════════════
// AMBIENT CONSENSUS CLASS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * AmbientConsensus - Automatic Dog consensus triggering
 *
 * Listens to events and triggers consensus when needed.
 */
export class AmbientConsensus {
  constructor({ collectivePack, eventBus = null }) {
    this.pack = collectivePack;
    this.eventBus = eventBus || getEventBus();
    this._running = false;
    this._consensusCount = 0;
    this._lastConsensus = null;

    // Stats for streaming consensus (M2.2)
    this._streamingStats = {
      totalConsensus: 0,
      earlyExits: 0,
      fullVotes: 0,
      avgSkippedVotes: 0, // Running average
    };

    // ═══════════════════════════════════════════════════════════════════════
    // MATH MODULE INTEGRATION
    // ═══════════════════════════════════════════════════════════════════════

    // Markov chain for predicting consensus outcomes by topic type
    // States: approved, rejected, insufficient
    this.outcomeChain = createMarkovChain(['approved', 'rejected', 'insufficient']);

    // Bayesian track records per dog (Beta distribution)
    // α = correct votes, β = incorrect votes
    // Prior: α=1, β=1 → uniform (no bias)
    this.dogTrackRecords = {};

    // Vote history for anomaly detection (per dog)
    this.dogVoteHistory = {};

    // Consensus history for entropy analysis
    this.consensusHistory = [];

    // Stored unsubscribe functions (from eventBus.subscribe return value)
    this._unsubscribers = [];

    // Bridge DogSignals to globalEventBus so event-listeners can persist them
    this._publishSignal = (type, payload) => {
      this.eventBus.publish(type, payload);
      globalEventBus.publish(type, { ...payload, timestamp: Date.now() });
    };

    // Bind methods
    this._onPreToolUse = this._onPreToolUse.bind(this);
    this._onPostToolUse = this._onPostToolUse.bind(this);
    this._onDogSignal = this._onDogSignal.bind(this);
  }

  /**
   * Start ambient consensus monitoring
   */
  start() {
    if (this._running) return;

    // Subscribe to hook events (store unsubscribe fns)
    this._unsubscribers.push(
      this.eventBus.subscribe(EventType.HOOK_PRE_TOOL, this._onPreToolUse),
      this.eventBus.subscribe(EventType.HOOK_POST_TOOL, this._onPostToolUse),
    );

    // Subscribe to dog signals
    Object.values(DogSignal).forEach(signal => {
      this._unsubscribers.push(
        this.eventBus.subscribe(signal, this._onDogSignal),
      );
    });

    this._running = true;
    log.info('Ambient consensus started', { thresholds: CONSENSUS_THRESHOLDS });
  }

  /**
   * Stop ambient consensus monitoring
   */
  stop() {
    if (!this._running) return;

    for (const unsub of this._unsubscribers) {
      if (typeof unsub === 'function') unsub();
    }
    this._unsubscribers = [];

    this._running = false;
    log.info('Ambient consensus stopped');
  }

  /**
   * Handle PreToolUse events
   * - Guardian checks safety
   * - If dangerous or low confidence → trigger consensus
   */
  async _onPreToolUse(event) {
    const { tool, input, confidence = PHI_INV } = event.payload || {};

    // Skip if tool is undefined (malformed event)
    if (!tool || typeof tool !== 'string') {
      log.debug('PreToolUse skipped — tool undefined or not a string', { tool });
      return { blocked: false };
    }

    try {
      // 1. Guardian safety check (always)
      if (this.pack.guardian?.checkCommand) {
        const guardianResult = await this.pack.guardian.checkCommand({
          command: tool,
          args: input,
        });

        if (guardianResult?.blocked) {
          // Guardian veto - emit signal
          this._publishSignal(DogSignal.DANGER_DETECTED, {
            source: 'guardian',
            tool,
            reason: guardianResult.reason,
          });

          log.warn('Guardian blocked operation', { tool, reason: guardianResult.reason });
          return { blocked: true, reason: guardianResult.reason };
        }
      }

      // 2. Check if consensus needed (low confidence)
      if (confidence < CONSENSUS_THRESHOLDS.LOW_CONFIDENCE) {
        log.debug('Low confidence detected, triggering consensus', { tool, confidence });

        const consensusResult = await this.triggerConsensus({
          topic: `pre_tool:${tool}`,
          context: { tool, input, confidence, hookType: 'PreToolUse' },
          reason: `Confidence ${(confidence * 100).toFixed(1)}% < ${(CONSENSUS_THRESHOLDS.LOW_CONFIDENCE * 100).toFixed(1)}%`,
        });

        if (!consensusResult.approved) {
          return { blocked: true, reason: 'Consensus rejected', votes: consensusResult.votes };
        }
      }

      return { blocked: false };
    } catch (e) {
      log.error('PreToolUse consensus error', { tool, error: e.message });
      return { blocked: false }; // Fail open
    }
  }

  /**
   * Handle PostToolUse events
   * - Analyst analyzes result
   * - If patterns found → notify other Dogs
   * - Sage shares wisdom if relevant
   */
  async _onPostToolUse(event) {
    const { tool, result, success, error } = event.payload || {};

    // Skip if tool is undefined (malformed event)
    if (!tool || typeof tool !== 'string') {
      log.debug('PostToolUse skipped — tool undefined or not a string', { tool });
      return;
    }

    try {
      // 1. Analyst analysis (always)
      if (this.pack.analyst?.analyze) {
        const analysis = await Promise.race([
          this.pack.analyst.analyze({ tool, result, success, error }),
          new Promise(resolve => setTimeout(() => resolve(null), 2000)), // 2s timeout
        ]);

        if (analysis?.patterns?.length > 0) {
          // Emit pattern found signal
          this._publishSignal(DogSignal.PATTERN_FOUND, {
            source: 'analyst',
            patterns: analysis.patterns,
            tool,
          });

          // Scout explores if new patterns
          if (this.pack.scout?.explore && analysis.patterns.some(p => p.isNew)) {
            this.pack.scout.explore({ patterns: analysis.patterns }).catch(() => {});
          }
        }

        if (analysis?.anomalies?.length > 0) {
          // Trigger consensus on anomalies
          await this.triggerConsensus({
            topic: `anomaly:${tool}`,
            context: { tool, anomalies: analysis.anomalies },
            reason: `${analysis.anomalies.length} anomalies detected`,
          });
        }
      }

      // 2. Scholar extracts knowledge (if relevant)
      if (this.pack.scholar?.extractKnowledge && success) {
        this.pack.scholar.extractKnowledge({ tool, result }).catch(() => {});
      }

      // 3. Sage shares wisdom on errors
      if (this.pack.sage?.shareWisdom && error) {
        const wisdom = await this.pack.sage.shareWisdom({
          topic: 'error_recovery',
          context: { tool, error },
        });

        if (wisdom) {
          this._publishSignal(DogSignal.WISDOM_SHARED, {
            source: 'sage',
            wisdom,
            tool,
          });
        }
      }
    } catch (e) {
      log.error('PostToolUse analysis error', { tool, error: e.message });
    }
  }

  /**
   * Handle inter-dog signals
   */
  async _onDogSignal(event) {
    const { source, ...data } = event.payload || {};

    log.trace('Dog signal received', { type: event.type, source });

    // React to specific signals
    switch (event.type) {
      case DogSignal.DANGER_DETECTED:
        // All dogs should be aware
        if (this.pack.cynic?.alert) {
          this.pack.cynic.alert(data);
        }
        break;

      case DogSignal.PATTERN_FOUND:
        // Cartographer updates map
        if (this.pack.cartographer?.updateMap) {
          this.pack.cartographer.updateMap(data.patterns);
        }
        break;

      case DogSignal.CONSENSUS_NEEDED:
        // Trigger consensus
        await this.triggerConsensus(data);
        break;
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STREAMING CONSENSUS (M2.2 Optimization)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Calculate voting agreement from partial results
   * Uses weighted agreement (70% track record, 30% simple majority)
   *
   * @private
   * @param {string[]} voters - Dog names
   * @param {Object[]} voteResults - Vote results so far
   * @returns {Object} {agreement, weightedAgreement, simpleAgreement, approveCount, rejectCount}
   */
  _calculateAgreement(voters, voteResults) {
    let approveCount = 0;
    let rejectCount = 0;
    let abstainCount = 0;
    let weightedApprove = 0;
    let weightedReject = 0;
    let totalWeight = 0;

    for (let i = 0; i < voteResults.length; i++) {
      const dogName = voters[i];
      const voteResult = voteResults[i];

      const voteWeight = this._calculateVoteWeight(dogName, voteResult.vote);

      if (voteResult.vote === 'approve') {
        approveCount++;
        weightedApprove += voteWeight.weight;
      } else if (voteResult.vote === 'reject') {
        rejectCount++;
        weightedReject += voteWeight.weight;
      } else {
        abstainCount++;
      }

      if (voteResult.vote !== 'abstain') {
        totalWeight += voteWeight.weight;
      }
    }

    const totalVoters = voteResults.length - abstainCount;
    const simpleAgreement = totalVoters > 0 ? approveCount / totalVoters : 0;
    const weightedAgreement = totalWeight > 0 ? weightedApprove / totalWeight : 0;

    // Blend: 70% weighted + 30% simple
    const agreement = weightedAgreement * 0.7 + simpleAgreement * 0.3;

    return {
      agreement,
      weightedAgreement,
      simpleAgreement,
      approveCount,
      rejectCount,
      abstainCount,
      totalVoters,
    };
  }

  /**
   * Stream votes and exit early if strong consensus reached
   *
   * Early exit conditions (both must be met):
   * 1. At least 7 Dogs voted (φ-quorum: φ × 11 ≈ 6.798 → 7)
   * 2. Agreement > 85% (strong consensus)
   *
   * @private
   * @param {string[]} voters - Dog names
   * @param {Promise[]} votePromises - Vote promises
   * @param {Object} context - Enriched context
   * @returns {Promise<Object>} {voteResults, earlyExit, skipped}
   */
  async _streamingConsensus(voters, votePromises, context) {
    const voteResults = [];
    const PHI_QUORUM = 7; // φ × 11 ≈ 6.798
    const STRONG_CONSENSUS = 0.85;

    // Process votes as they arrive
    for (let i = 0; i < votePromises.length; i++) {
      const voteResult = await votePromises[i];
      voteResults.push(voteResult);

      // Check for early exit after φ-quorum
      if (voteResults.length >= PHI_QUORUM) {
        const { agreement, approveCount, rejectCount, abstainCount, totalVoters } =
          this._calculateAgreement(voters.slice(0, voteResults.length), voteResults);

        // Strong consensus on EITHER approve OR reject
        const strongApprove = agreement >= STRONG_CONSENSUS;
        const strongReject = (1 - agreement) >= STRONG_CONSENSUS;

        // Require BOTH: strong agreement AND enough active voters (φ-quorum for actives too)
        // This prevents early exit when many Dogs abstain
        const hasActiveQuorum = totalVoters >= PHI_QUORUM;

        if ((strongApprove || strongReject) && hasActiveQuorum) {
          const skipped = votePromises.length - voteResults.length;

          log.info('Early consensus exit', {
            votesCollected: voteResults.length,
            skipped,
            agreement: `${(agreement * 100).toFixed(1)}%`,
            approve: approveCount,
            reject: rejectCount,
            abstain: abstainCount,
          });

          // Update streaming stats
          this._streamingStats.earlyExits++;
          // Calculate running average (use totalConsensus + 1 since we haven't incremented it yet)
          const n = this._streamingStats.totalConsensus + 1;
          this._streamingStats.avgSkippedVotes =
            (this._streamingStats.avgSkippedVotes * (n - 1) + skipped) / n;

          return {
            voteResults,
            earlyExit: true,
            skipped,
            earlyAgreement: agreement,
          };
        }
      }
    }

    // All votes collected, no early exit
    log.debug('Full voting completed', {
      votesCollected: voteResults.length,
      earlyExit: false,
    });

    this._streamingStats.fullVotes++;

    return {
      voteResults,
      earlyExit: false,
      skipped: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MATH MODULE METHODS (Bayes, Markov, Gaussian, Entropy)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize or get track record for a dog
   * @private
   * @param {string} dogName - Dog name
   * @returns {BetaDistribution} Track record
   */
  _getDogTrackRecord(dogName) {
    if (!this.dogTrackRecords[dogName]) {
      // Prior: uniform (α=1, β=1)
      this.dogTrackRecords[dogName] = new BetaDistribution(1, 1);
    }
    return this.dogTrackRecords[dogName];
  }

  /**
   * Calculate weighted vote based on dog's track record
   * Dogs with better accuracy get higher weight (φ-bounded)
   * @private
   * @param {string} dogName - Dog name
   * @param {string} vote - approve/reject/abstain
   * @returns {Object} {weight, confidence}
   */
  _calculateVoteWeight(dogName, vote) {
    if (vote === 'abstain') {
      return { weight: 0, confidence: 0 };
    }

    const trackRecord = this._getDogTrackRecord(dogName);
    const accuracy = trackRecord.getMean();
    const strength = trackRecord.getStrength();

    // Weight increases with accuracy, φ-bounded
    // More observations → higher confidence in the weight
    const rawWeight = accuracy;
    const confidence = Math.min(PHI_INV, strength / 20); // 20 votes = max raw confidence

    return {
      weight: Math.min(PHI_INV, rawWeight), // Never exceed φ⁻¹
      confidence,
      accuracy,
      strength,
    };
  }

  /**
   * Update dog's track record based on consensus outcome
   * @private
   * @param {string} dogName - Dog name
   * @param {string} vote - Dog's vote
   * @param {boolean} consensusApproved - Final outcome
   */
  _updateDogTrackRecord(dogName, vote, consensusApproved) {
    if (vote === 'abstain') return;

    const trackRecord = this._getDogTrackRecord(dogName);
    const votedCorrectly =
      (vote === 'approve' && consensusApproved) ||
      (vote === 'reject' && !consensusApproved);

    if (votedCorrectly) {
      trackRecord.recordSuccess();
    } else {
      trackRecord.recordFailure();
    }
  }

  /**
   * Record vote for anomaly detection
   * @private
   * @param {string} dogName - Dog name
   * @param {number} voteValue - 1 (approve), 0 (abstain), -1 (reject)
   */
  _recordVoteForAnomaly(dogName, voteValue) {
    if (!this.dogVoteHistory[dogName]) {
      this.dogVoteHistory[dogName] = [];
    }
    this.dogVoteHistory[dogName].push(voteValue);

    // Keep bounded at Fib(8) = 21
    while (this.dogVoteHistory[dogName].length > 21) {
      this.dogVoteHistory[dogName].shift();
    }
  }

  /**
   * Detect if a vote is anomalous for this dog
   * @private
   * @param {string} dogName - Dog name
   * @param {number} voteValue - Vote value
   * @returns {Object} {isAnomaly, zScore, severity}
   */
  _detectVoteAnomaly(dogName, voteValue) {
    const history = this.dogVoteHistory[dogName] || [];

    if (history.length < 5) {
      return { isAnomaly: false, zScore: 0, severity: 'none' };
    }

    // Compute stats from history (exclude current)
    const prevHistory = history.slice(0, -1);
    const stats = computeStats(prevHistory);
    const z = zScore(voteValue, stats.mean, stats.stdDev);

    // Classify severity using φ thresholds
    let severity = 'none';
    let isAnomaly = false;

    if (Math.abs(z) > 2.5) {
      severity = 'significant';
      isAnomaly = true;
    } else if (Math.abs(z) > 1.5) {
      severity = 'minor';
    }

    return {
      isAnomaly,
      zScore: Math.round(z * 100) / 100,
      severity,
    };
  }

  /**
   * Predict consensus outcome using Markov chain
   * @private
   * @param {string} topic - Consensus topic
   * @returns {Object} Prediction
   */
  _predictOutcome(topic) {
    // Map last outcome to prediction
    const lastOutcome = this._lastConsensus?.reason === 'consensus_reached' ? 'approved' :
      this._lastConsensus?.reason === 'insufficient_voters' ? 'insufficient' : 'rejected';

    const prediction = this.outcomeChain.predict(lastOutcome);

    return {
      predictedOutcome: prediction.state || 'approved',
      probability: prediction.probability,
      confidence: prediction.confidence,
      basedOn: lastOutcome,
    };
  }

  /**
   * Record outcome for Markov learning
   * @private
   * @param {string} outcome - Outcome state
   */
  _recordOutcome(outcome) {
    const lastOutcome = this._lastConsensus?.reason === 'consensus_reached' ? 'approved' :
      this._lastConsensus?.reason === 'insufficient_voters' ? 'insufficient' : 'rejected';

    // Only record if we have a previous outcome
    if (this._consensusCount > 1) {
      this.outcomeChain.observe(lastOutcome, outcome);
    }
  }

  /**
   * Calculate voting entropy (division measure)
   * @private
   * @param {number} approveCount - Approve votes
   * @param {number} rejectCount - Reject votes
   * @param {number} abstainCount - Abstain votes
   * @returns {Object} {entropy, normalized, division}
   */
  _calculateVotingEntropy(approveCount, rejectCount, abstainCount) {
    const counts = [approveCount, rejectCount, abstainCount].filter(c => c > 0);

    if (counts.length <= 1) {
      return { entropy: 0, normalized: 0, division: 'unanimous' };
    }

    const analysis = entropyConfidence(counts);

    // Classify division level
    let division = 'unanimous';
    if (analysis.normalized > PHI_INV) {
      division = 'deeply_divided';
    } else if (analysis.normalized > PHI_INV_2) {
      division = 'divided';
    } else if (analysis.normalized > PHI_INV_3) {
      division = 'slight_disagreement';
    }

    return {
      entropy: analysis.entropy,
      normalized: analysis.normalized,
      division,
      confidence: analysis.confidence,
    };
  }

  /**
   * Get dog track record statistics
   * @returns {Object} Track records by dog
   */
  getDogStats() {
    const stats = {};
    for (const [dogName, record] of Object.entries(this.dogTrackRecords)) {
      stats[dogName] = {
        accuracy: record.getMean(),
        strength: record.getStrength(),
        alpha: record.alpha,
        beta: record.beta,
      };
    }
    return stats;
  }

  /**
   * Trigger consensus vote across all Dogs
   *
   * @param {Object} options - Consensus options
   * @param {string} options.topic - What we're voting on
   * @param {Object} options.context - Context for voting
   * @param {string} [options.reason] - Why consensus is needed
   * @returns {Promise<Object>} Consensus result
   */
  async triggerConsensus({ topic, context, reason }) {
    this._consensusCount++;
    const consensusId = `consensus_${this._consensusCount}_${Date.now()}`;

    log.info('Triggering consensus', { consensusId, topic, reason });

    // ═══════════════════════════════════════════════════════════════════════
    // JUDGE PIPELINE INTEGRATION — 25-dimension analysis before Dog voting
    // Dogs now receive judgment context to inform their votes
    // ═══════════════════════════════════════════════════════════════════════
    let judgment = null;
    if (this.pack.judge?.judge) {
      try {
        const item = {
          type: context.tool ? 'tool_use' : topic,
          id: consensusId,
          tool: context.tool,
          input: context.input,
          ...context,
        };
        judgment = this.pack.judge.judge(item, {
          type: context.tool ? 'tool_use' : topic,
          queryType: context.taskType || context.queryType || context.type || topic,
        });
        log.debug('Judge scored consensus item', {
          qScore: judgment.qScore,
          verdict: judgment.verdict,
          confidence: judgment.confidence,
        });
      } catch (e) {
        log.warn('Judge scoring failed, Dogs vote without judgment', { error: e.message });
      }
    }

    // Enrich context with judgment for Dogs
    const enrichedContext = {
      ...context,
      judgment: judgment ? {
        qScore: judgment.qScore,
        verdict: judgment.verdict,
        confidence: judgment.confidence,
        axiomScores: judgment.axiomScores,
        weaknesses: judgment.weaknesses,
        entropy: judgment.entropy,
      } : null,
    };

    // Predict outcome using Markov chain (before voting)
    const prediction = this._predictOutcome(topic);
    log.debug('Outcome prediction', prediction);

    // Collect votes from all Dogs with voteOnConsensus method
    const voters = [];
    const votePromises = [];

    const dogsToConsult = [
      'guardian', 'analyst', 'sage', 'scout', 'architect',
      'scholar', 'janitor', 'deployer', 'oracle', 'cartographer', 'cynic',
    ];

    for (const dogName of dogsToConsult) {
      const dog = this.pack[dogName];
      if (dog?.voteOnConsensus) {
        voters.push(dogName);
        votePromises.push(
          Promise.race([
            dog.voteOnConsensus(topic, enrichedContext),
            new Promise(resolve => setTimeout(() => resolve({
              vote: 'abstain',
              reason: 'timeout',
            }), CONSENSUS_THRESHOLDS.VOTE_TIMEOUT_MS)),
          ]).catch(e => ({
            vote: 'abstain',
            reason: `error: ${e.message}`,
          }))
        );
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // STREAMING CONSENSUS WITH EARLY EXIT (M2.2 Optimization)
    // Stop waiting for remaining votes if strong consensus reached (φ-quorum)
    // ═══════════════════════════════════════════════════════════════════════
    const streamResult = await this._streamingConsensus(voters, votePromises, enrichedContext);
    const voteResults = streamResult.voteResults;
    const earlyExit = streamResult.earlyExit;

    // Tally votes with weighted voting and anomaly detection
    const votes = {};
    let approveCount = 0;
    let rejectCount = 0;
    let abstainCount = 0;
    let weightedApprove = 0;
    let weightedReject = 0;
    let totalWeight = 0;
    let guardianVeto = false;
    const anomalies = [];

    // Only iterate over collected votes (may be < voters.length due to early exit)
    for (let i = 0; i < voteResults.length; i++) {
      const dogName = voters[i];
      const voteResult = voteResults[i];

      // Calculate vote weight based on track record
      const voteWeight = this._calculateVoteWeight(dogName, voteResult.vote);

      // Convert vote to numeric for anomaly detection
      const voteValue = voteResult.vote === 'approve' ? 1 :
        voteResult.vote === 'reject' ? -1 : 0;

      // Record for anomaly history
      this._recordVoteForAnomaly(dogName, voteValue);

      // Detect anomaly
      const anomaly = this._detectVoteAnomaly(dogName, voteValue);
      if (anomaly.isAnomaly) {
        anomalies.push({ dog: dogName, vote: voteResult.vote, ...anomaly });
      }

      // Store enriched vote
      votes[dogName] = {
        ...voteResult,
        weight: voteWeight.weight,
        trackRecord: voteWeight.accuracy,
        anomaly: anomaly.isAnomaly ? anomaly : null,
      };

      // Tally simple counts
      if (voteResult.vote === 'approve') {
        approveCount++;
        weightedApprove += voteWeight.weight;
      } else if (voteResult.vote === 'reject') {
        rejectCount++;
        weightedReject += voteWeight.weight;
      } else {
        abstainCount++;
      }

      if (voteResult.vote !== 'abstain') {
        totalWeight += voteWeight.weight;
      }

      // Guardian veto on safety topics
      if (dogName === 'guardian' && voteResult.vote === 'reject' && topic.includes('safety')) {
        guardianVeto = true;
      }
    }

    // Calculate voting entropy (division measure)
    const votingEntropy = this._calculateVotingEntropy(approveCount, rejectCount, abstainCount);

    // Calculate agreement (both simple and weighted)
    const totalVoters = voters.length - abstainCount;
    const simpleAgreement = totalVoters > 0 ? approveCount / totalVoters : 0;
    const weightedAgreement = totalWeight > 0 ? weightedApprove / totalWeight : 0;

    // Use weighted agreement for decision (blended with simple)
    // 70% weighted + 30% simple to respect track records while not ignoring any dog
    const agreement = weightedAgreement * 0.7 + simpleAgreement * 0.3;

    // Determine approval
    const approved = !guardianVeto &&
      totalVoters >= CONSENSUS_THRESHOLDS.MIN_VOTERS &&
      agreement >= CONSENSUS_THRESHOLDS.AGREEMENT_THRESHOLD;

    // Determine outcome for Markov
    const outcome = approved ? 'approved' :
      totalVoters < CONSENSUS_THRESHOLDS.MIN_VOTERS ? 'insufficient' : 'rejected';

    // Record outcome for Markov learning
    this._recordOutcome(outcome);

    const result = {
      consensusId,
      topic,
      approved,
      agreement,
      guardianVeto,
      votes,
      stats: {
        approve: approveCount,
        reject: rejectCount,
        abstain: abstainCount,
        total: voters.length,
      },
      reason: approved ? 'consensus_reached' :
        guardianVeto ? 'guardian_veto' :
        totalVoters < CONSENSUS_THRESHOLDS.MIN_VOTERS ? 'insufficient_voters' :
        'consensus_not_reached',
      // Judge 25-dimension analysis (if available)
      judgment: judgment ? {
        qScore: judgment.qScore,
        verdict: judgment.verdict,
        confidence: judgment.confidence,
        axiomScores: judgment.axiomScores,
      } : null,
      // New: Math enrichments
      inference: {
        prediction,
        predictionCorrect: prediction.predictedOutcome === outcome,
        weightedAgreement,
        simpleAgreement,
        votingEntropy,
        anomalies,
        hasAnomalies: anomalies.length > 0,
      },
      // Streaming stats (M2.2)
      streaming: {
        earlyExit: streamResult.earlyExit,
        skipped: streamResult.skipped,
        earlyAgreement: streamResult.earlyAgreement,
      },
    };

    // Update streaming stats totals
    this._streamingStats.totalConsensus++;

    // Update dog track records based on outcome
    for (const [dogName, voteData] of Object.entries(votes)) {
      this._updateDogTrackRecord(dogName, voteData.vote, approved);
    }

    // Store in history for entropy analysis
    this.consensusHistory.push({
      timestamp: Date.now(),
      topic,
      outcome,
      agreement,
      entropy: votingEntropy.normalized,
    });

    // Keep history bounded at Fib(10) = 55
    while (this.consensusHistory.length > 55) {
      this.consensusHistory.shift();
    }

    this._lastConsensus = result;

    log.info('Consensus result', {
      consensusId,
      topic,
      approved,
      agreement: `${(agreement * 100).toFixed(1)}%`,
      weighted: `${(weightedAgreement * 100).toFixed(1)}%`,
      stats: result.stats,
      division: votingEntropy.division,
      predictionCorrect: result.inference.predictionCorrect,
    });

    // Emit consensus result (local bus)
    this.eventBus.publish('consensus:completed', result);

    // Also publish to globalEventBus for persistence
    try {
      globalEventBus.publish(CoreEventType.CONSENSUS_COMPLETED, result, {
        source: 'AmbientConsensus',
      });
    } catch (e) {
      // Non-critical - don't break consensus flow
    }

    // Record to learning_events for G1.2 metric
    try {
      const { getPool } = await import('@cynic/persistence');
      const pool = getPool();
      await pool.query(`
        INSERT INTO learning_events (loop_type, event_type, action_taken, metadata)
        VALUES ($1, $2, $3, $4)
      `, [
        'dog-votes',
        'consensus',
        approved ? 'approved' : 'rejected',
        JSON.stringify({
          consensusId,
          topic,
          votes: Object.keys(votes).length,
          agreement: Math.round(agreement * 1000) / 1000,
          stats: result.stats,
          entropy: votingEntropy.normalized
        })
      ]);
    } catch { /* non-blocking DB write */ }

    return result;
  }

  /**
   * Get consensus statistics (enriched with math)
   */
  getStats() {
    // Calculate overall entropy trend
    const recentEntropy = this.consensusHistory.slice(-10).map(h => h.entropy);
    const entropyTrend = recentEntropy.length >= 3 ?
      (recentEntropy[recentEntropy.length - 1] > recentEntropy[0] ? 'increasing' : 'decreasing') :
      'unknown';

    // Prediction accuracy
    const predictions = this.consensusHistory.filter(h => h.predictionCorrect !== undefined);
    const predictionAccuracy = predictions.length > 0 ?
      predictions.filter(p => p.predictionCorrect).length / predictions.length : 0;

    return {
      running: this._running,
      consensusCount: this._consensusCount,
      lastConsensus: this._lastConsensus,
      thresholds: CONSENSUS_THRESHOLDS,
      // New: Math enrichments
      inference: {
        dogStats: this.getDogStats(),
        entropyTrend,
        predictionAccuracy: Math.min(PHI_INV, predictionAccuracy), // φ-bounded
        historySize: this.consensusHistory.length,
        outcomeDistribution: this._getOutcomeDistribution(),
      },
      // Streaming consensus stats (M2.2)
      streaming: {
        enabled: true,
        totalConsensus: this._streamingStats.totalConsensus,
        earlyExits: this._streamingStats.earlyExits,
        fullVotes: this._streamingStats.fullVotes,
        earlyExitRate: this._streamingStats.totalConsensus > 0
          ? this._streamingStats.earlyExits / this._streamingStats.totalConsensus
          : 0,
        avgSkippedVotes: Math.round(this._streamingStats.avgSkippedVotes * 10) / 10,
        avgTimeSaved: Math.round(this._streamingStats.avgSkippedVotes * 20), // ~20ms per vote
      },
    };
  }

  /**
   * Get outcome distribution from history
   * @private
   */
  _getOutcomeDistribution() {
    const counts = { approved: 0, rejected: 0, insufficient: 0 };
    for (const h of this.consensusHistory) {
      if (counts[h.outcome] !== undefined) {
        counts[h.outcome]++;
      }
    }
    const total = this.consensusHistory.length || 1;
    return {
      approved: counts.approved / total,
      rejected: counts.rejected / total,
      insufficient: counts.insufficient / total,
    };
  }

  /**
   * Restore dog track records and vote history from database
   * Rebuilds BetaDistribution per dog from consensus_votes table
   *
   * @param {Object} persistence - Database pool/client with query()
   * @returns {Promise<{restored: boolean, dogs: number, votes: number}>}
   */
  async restoreFromDatabase(persistence) {
    if (!persistence?.query) return { restored: false, dogs: 0, votes: 0 };

    try {
      // Query recent consensus votes (last 30 days)
      const { rows } = await persistence.query(`
        SELECT votes, approved
        FROM consensus_votes
        WHERE created_at > NOW() - INTERVAL '30 days'
        ORDER BY created_at ASC
      `);

      if (rows.length === 0) return { restored: false, dogs: 0, votes: 0 };

      let totalVotes = 0;

      for (const row of rows) {
        const votes = typeof row.votes === 'string' ? JSON.parse(row.votes) : (row.votes || {});
        const approved = row.approved;

        for (const [dogName, voteData] of Object.entries(votes)) {
          const vote = voteData?.vote;
          if (!vote || vote === 'abstain') continue;

          // Rebuild Beta distribution (track record)
          const trackRecord = this._getDogTrackRecord(dogName);
          const votedCorrectly =
            (vote === 'approve' && approved) ||
            (vote === 'reject' && !approved);

          if (votedCorrectly) {
            trackRecord.recordSuccess();
          } else {
            trackRecord.recordFailure();
          }

          // Rebuild vote history (for anomaly detection)
          const voteValue = vote === 'approve' ? 1 : -1;
          this._recordVoteForAnomaly(dogName, voteValue);
          totalVotes++;
        }
      }

      // Rebuild consensus history
      this._consensusCount = rows.length;

      log.info('Dog state restored from database', {
        consensusVotes: rows.length,
        dogs: Object.keys(this.dogTrackRecords).length,
        totalVotes,
      });

      return {
        restored: true,
        dogs: Object.keys(this.dogTrackRecords).length,
        votes: totalVotes,
      };
    } catch (err) {
      log.warn('Dog state restoration failed', { error: err.message });
      return { restored: false, dogs: 0, votes: 0 };
    }
  }

  /**
   * Reset math module state
   */
  resetInference() {
    this.outcomeChain = createMarkovChain(['approved', 'rejected', 'insufficient']);
    this.dogTrackRecords = {};
    this.dogVoteHistory = {};
    this.consensusHistory = [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Wire ambient consensus to a CollectivePack
 *
 * @param {Object} pack - CollectivePack instance
 * @param {Object} [options] - Options
 * @returns {AmbientConsensus} Wired consensus instance
 */
export function wireAmbientConsensus(pack, options = {}) {
  const consensus = new AmbientConsensus({
    collectivePack: pack,
    eventBus: options.eventBus,
  });

  consensus.start();

  log.info('Ambient consensus wired to pack');

  return consensus;
}

export default {
  AmbientConsensus,
  wireAmbientConsensus,
  CONSENSUS_THRESHOLDS,
  DogSignal,
};
