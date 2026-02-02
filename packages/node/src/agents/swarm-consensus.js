/**
 * Swarm Consensus - Collective Intelligence for Dog Pack
 *
 * Implements advanced consensus algorithms:
 * - Anti-drift mechanisms
 * - Conflict resolution
 * - Emergent decision making
 * - φ-aligned quorum thresholds
 *
 * P2.4: Beyond simple voting to true collective intelligence.
 *
 * "Many dogs, one truth" - κυνικός
 *
 * @module @cynic/node/agents/swarm-consensus
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { DOG_CONFIG, DogModel } from './orchestrator.js';
import { AgentId } from './events.js';
import { getEventBus } from '../services/event-bus.js';

/**
 * Consensus strategies
 */
export const ConsensusStrategy = {
  SIMPLE_MAJORITY: 'simple_majority',      // > 50%
  PHI_SUPERMAJORITY: 'phi_supermajority',  // > 61.8% (φ⁻¹)
  UNANIMOUS: 'unanimous',                   // 100%
  WEIGHTED_MEDIAN: 'weighted_median',       // Median with weights
  EMERGENT: 'emergent',                     // Pattern-based emergence
};

/**
 * Drift types
 */
export const DriftType = {
  SCORE_DRIFT: 'score_drift',           // Dog scoring differently than historical
  POSITION_DRIFT: 'position_drift',     // Dog changing allow/block tendency
  DIMENSION_DRIFT: 'dimension_drift',   // Dog weighting dimensions differently
  ALIGNMENT_DRIFT: 'alignment_drift',   // Dog diverging from pack
};

/**
 * Conflict resolution strategies
 */
export const ConflictResolution = {
  DOMAIN_EXPERT: 'domain_expert',       // Expert dog in that domain wins
  BLOCKING_PRIORITY: 'blocking_priority', // Blocking dogs have veto
  CONFIDENCE_WEIGHTED: 'confidence_weighted', // Higher confidence wins
  HISTORICAL_ACCURACY: 'historical_accuracy', // More accurate dog wins
  MEDIATED: 'mediated',                 // Seek middle ground
};

/**
 * φ-aligned swarm configuration
 */
export const SWARM_CONFIG = {
  // Drift detection
  driftWindow: 20,                    // Votes to track for drift
  driftThreshold: PHI_INV_2,          // 38.2% deviation triggers alert
  driftDecay: 0.95,                   // Historical weight decay

  // Conflict resolution
  conflictThreshold: 3,               // # of opposing votes before conflict
  mediationWeight: PHI_INV,           // 61.8% weight to expert

  // Emergent patterns
  emergenceMinVotes: 5,               // Minimum votes for pattern
  emergenceConfidence: PHI_INV,       // Required confidence for emergence
  patternSimilarity: 0.8,             // Similarity threshold for pattern match

  // Quorum requirements
  quorum: {
    security: 0.9,                    // Security decisions: 90%
    deployment: 0.8,                  // Deploy decisions: 80%
    standard: PHI_INV,                // Standard: 61.8%
    exploratory: 0.5,                 // Exploration: 50%
  },

  // Dog domain expertise
  domainExperts: {
    security: [AgentId.GUARDIAN, AgentId.ANALYST],
    architecture: [AgentId.ARCHITECT, AgentId.SAGE],
    deployment: [AgentId.DEPLOYER, AgentId.GUARDIAN],
    code_quality: [AgentId.JANITOR, AgentId.ANALYST],
    exploration: [AgentId.SCOUT, AgentId.CARTOGRAPHER],
    synthesis: [AgentId.ORACLE, AgentId.CYNIC],
    knowledge: [AgentId.SCHOLAR, AgentId.SAGE],
  },
};

/**
 * Dog position (allow/block tendency)
 */
export class DogPosition {
  /**
   * @param {string} dogId
   * @param {Object} [options]
   */
  constructor(dogId, options = {}) {
    this.dogId = dogId;
    this.scoreHistory = [];        // Recent scores
    this.positionHistory = [];     // 'allow' or 'block' history
    this.dimensionWeights = {};    // How dog weights each dimension
    this.accuracy = 0.5;           // Accuracy vs ground truth
    this.voteCount = 0;
    this.maxHistory = options.maxHistory || SWARM_CONFIG.driftWindow;
  }

  /**
   * Record a vote
   */
  recordVote(vote) {
    this.voteCount++;

    // Score history
    if (typeof vote.score === 'number') {
      this.scoreHistory.push(vote.score);
      if (this.scoreHistory.length > this.maxHistory) {
        this.scoreHistory.shift();
      }
    }

    // Position history
    const position = vote.response === 'block' ? 1 : 0;
    this.positionHistory.push(position);
    if (this.positionHistory.length > this.maxHistory) {
      this.positionHistory.shift();
    }

    // Dimension weights
    if (vote.dimensions) {
      for (const [dim, score] of Object.entries(vote.dimensions)) {
        if (!this.dimensionWeights[dim]) {
          this.dimensionWeights[dim] = [];
        }
        this.dimensionWeights[dim].push(score);
        if (this.dimensionWeights[dim].length > this.maxHistory) {
          this.dimensionWeights[dim].shift();
        }
      }
    }
  }

  /**
   * Update accuracy based on feedback
   */
  updateAccuracy(wasCorrect) {
    // EMA update
    const alpha = 0.1;
    this.accuracy = this.accuracy * (1 - alpha) + (wasCorrect ? 1 : 0) * alpha;
  }

  /**
   * Get mean score
   */
  getMeanScore() {
    if (this.scoreHistory.length === 0) return 50;
    return this.scoreHistory.reduce((a, b) => a + b, 0) / this.scoreHistory.length;
  }

  /**
   * Get score standard deviation
   */
  getScoreStdDev() {
    if (this.scoreHistory.length < 2) return 0;
    const mean = this.getMeanScore();
    const variance = this.scoreHistory.reduce((sum, s) => sum + (s - mean) ** 2, 0) / this.scoreHistory.length;
    return Math.sqrt(variance);
  }

  /**
   * Get block tendency (0-1)
   */
  getBlockTendency() {
    if (this.positionHistory.length === 0) return 0;
    return this.positionHistory.reduce((a, b) => a + b, 0) / this.positionHistory.length;
  }
}

/**
 * Conflict record between two dogs
 */
export class ConflictRecord {
  constructor(dogA, dogB) {
    this.dogA = dogA;
    this.dogB = dogB;
    this.conflicts = 0;
    this.agreements = 0;
    this.resolutions = [];
    this.lastConflict = null;
  }

  recordConflict(resolved = false, resolution = null) {
    this.conflicts++;
    this.lastConflict = Date.now();
    if (resolved && resolution) {
      this.resolutions.push({
        timestamp: Date.now(),
        resolution,
      });
    }
  }

  recordAgreement() {
    this.agreements++;
  }

  /**
   * Get disagreement ratio
   */
  getDisagreementRatio() {
    const total = this.conflicts + this.agreements;
    return total > 0 ? this.conflicts / total : 0;
  }
}

/**
 * Emergent pattern from collective voting
 */
export class EmergentPattern {
  constructor(options = {}) {
    this.id = options.id || `epattern_${Date.now().toString(36)}`;
    this.signature = options.signature || null;    // What votes look like
    this.frequency = options.frequency || 1;
    this.confidence = options.confidence || 0;
    this.outcomes = [];                            // What outcomes followed
    this.discovered = Date.now();
  }

  /**
   * Check if a vote pattern matches this pattern
   */
  matches(votes, threshold = SWARM_CONFIG.patternSimilarity) {
    if (!this.signature) return false;
    const similarity = this._calculateSimilarity(votes);
    return similarity >= threshold;
  }

  _calculateSimilarity(votes) {
    // Simple cosine similarity between vote vectors
    const voteVector = this._votesToVector(votes);
    const sigVector = this.signature;

    if (voteVector.length !== sigVector.length) return 0;

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < voteVector.length; i++) {
      dotProduct += voteVector[i] * sigVector[i];
      normA += voteVector[i] ** 2;
      normB += sigVector[i] ** 2;
    }

    if (normA === 0 || normB === 0) return 0;
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  _votesToVector(votes) {
    // Normalize votes to fixed-length vector
    const dogs = Object.keys(DOG_CONFIG);
    return dogs.map(dog => {
      const vote = votes.find(v => v.dog === dog);
      if (!vote || !vote.success) return 0;
      return (vote.score || 50) / 100;
    });
  }

  recordOutcome(outcome, wasPositive) {
    this.outcomes.push({
      timestamp: Date.now(),
      outcome,
      positive: wasPositive,
    });

    // Update confidence based on outcomes
    const positiveRatio = this.outcomes.filter(o => o.positive).length / this.outcomes.length;
    this.confidence = positiveRatio * PHI_INV; // Max confidence φ⁻¹
  }
}

/**
 * Swarm Consensus Engine
 *
 * Coordinates collective decision-making beyond simple voting.
 */
export class SwarmConsensus {
  /**
   * @param {Object} options
   * @param {Object} [options.config] - Override config
   * @param {Object} [options.orchestrator] - DogOrchestrator instance
   */
  constructor(options = {}) {
    this.config = { ...SWARM_CONFIG, ...options.config };
    this.orchestrator = options.orchestrator;

    // Event bus for visibility
    this.eventBus = options.eventBus || getEventBus();

    // Dog positions
    this.positions = new Map();
    for (const dogId of Object.keys(DOG_CONFIG)) {
      this.positions.set(dogId, new DogPosition(dogId));
    }

    // Conflict matrix
    this.conflicts = new Map();

    // Emergent patterns
    this.patterns = [];

    // Statistics
    this.stats = {
      totalDecisions: 0,
      driftAlerts: 0,
      conflictsDetected: 0,
      conflictsResolved: 0,
      emergenceEvents: 0,
      strategyUsage: {},
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Main Consensus Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Calculate consensus using appropriate strategy
   *
   * @param {Array} votes - Dog votes
   * @param {Object} [context] - Decision context
   * @returns {Object} Consensus result
   */
  calculateConsensus(votes, context = {}) {
    this.stats.totalDecisions++;

    // Record votes in positions
    for (const vote of votes) {
      const position = this.positions.get(vote.dog);
      if (position) {
        position.recordVote(vote);
      }
    }

    // Check for drift
    const driftAlerts = this._detectDrift(votes);

    // Check for conflicts
    const conflicts = this._detectConflicts(votes);

    // Determine strategy based on context
    const strategy = this._selectStrategy(context, conflicts);
    this.stats.strategyUsage[strategy] = (this.stats.strategyUsage[strategy] || 0) + 1;

    // Calculate consensus using selected strategy
    let result;
    switch (strategy) {
      case ConsensusStrategy.UNANIMOUS:
        result = this._unanimousConsensus(votes);
        break;
      case ConsensusStrategy.WEIGHTED_MEDIAN:
        result = this._weightedMedianConsensus(votes);
        break;
      case ConsensusStrategy.EMERGENT:
        result = this._emergentConsensus(votes, context);
        break;
      case ConsensusStrategy.PHI_SUPERMAJORITY:
      default:
        result = this._phiConsensus(votes);
        break;
    }

    // Resolve conflicts if any
    if (conflicts.length > 0) {
      result = this._resolveConflicts(result, conflicts, votes, context);
    }

    // Check for emergent patterns
    const emergence = this._checkEmergence(votes, result);

    return {
      ...result,
      strategy,
      driftAlerts,
      conflicts: conflicts.length,
      emergence,
      stats: {
        dogsVoted: votes.filter(v => v.success).length,
        totalDogs: Object.keys(DOG_CONFIG).length,
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Consensus Strategies
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * φ-supermajority consensus (default)
   */
  _phiConsensus(votes) {
    const successfulVotes = votes.filter(v => v.success);
    const totalWeight = successfulVotes.reduce((sum, v) => sum + (v.weight || 1), 0);

    // Check for blocking votes
    const blockingVote = successfulVotes.find(v =>
      v.response === 'block' && DOG_CONFIG[v.dog]?.blocking
    );

    if (blockingVote) {
      return {
        blocked: true,
        blockedBy: blockingVote.dog,
        reason: blockingVote.reason,
        ratio: 0,
        reached: false,
      };
    }

    // Calculate weighted approval
    const approvalWeight = successfulVotes
      .filter(v => v.response === 'allow' || v.response === 'approve')
      .reduce((sum, v) => sum + (v.weight || 1), 0);

    const ratio = totalWeight > 0 ? approvalWeight / totalWeight : 0;

    return {
      blocked: false,
      ratio,
      reached: ratio >= PHI_INV,
      threshold: PHI_INV,
      score: this._calculateWeightedScore(successfulVotes),
    };
  }

  /**
   * Unanimous consensus
   */
  _unanimousConsensus(votes) {
    const successfulVotes = votes.filter(v => v.success);

    const allApprove = successfulVotes.every(v =>
      v.response === 'allow' || v.response === 'approve'
    );

    return {
      blocked: !allApprove,
      blockedBy: allApprove ? null : 'dissent',
      ratio: allApprove ? 1.0 : 0,
      reached: allApprove,
      threshold: 1.0,
      score: this._calculateWeightedScore(successfulVotes),
    };
  }

  /**
   * Weighted median consensus
   */
  _weightedMedianConsensus(votes) {
    const successfulVotes = votes.filter(v => v.success && typeof v.score === 'number');

    if (successfulVotes.length === 0) {
      return {
        blocked: false,
        ratio: 0,
        reached: false,
        score: 50,
      };
    }

    // Sort by score with weights
    const sortedVotes = [...successfulVotes].sort((a, b) => a.score - b.score);
    const totalWeight = sortedVotes.reduce((sum, v) => sum + (v.weight || 1), 0);
    const midpoint = totalWeight / 2;

    let cumWeight = 0;
    let medianScore = sortedVotes[0].score;

    for (const vote of sortedVotes) {
      cumWeight += vote.weight || 1;
      if (cumWeight >= midpoint) {
        medianScore = vote.score;
        break;
      }
    }

    // Approve if median score > 50
    const ratio = medianScore / 100;

    return {
      blocked: medianScore < 38.2,   // Below φ⁻² threshold
      ratio,
      reached: medianScore >= 61.8,  // Above φ⁻¹ threshold
      score: medianScore,
      threshold: 0.5,
    };
  }

  /**
   * Emergent consensus - pattern-based collective intelligence
   */
  _emergentConsensus(votes, context) {
    // First, try to find a matching pattern
    for (const pattern of this.patterns) {
      if (pattern.matches(votes)) {
        this.stats.emergenceEvents++;

        // Use pattern's historical insight
        const suggestedScore = pattern.confidence > 0.5 ? 75 : 40;

        return {
          blocked: false,
          ratio: pattern.confidence,
          reached: pattern.confidence >= PHI_INV,
          score: suggestedScore,
          emergent: true,
          patternId: pattern.id,
          patternConfidence: pattern.confidence,
        };
      }
    }

    // No pattern found, fall back to φ-consensus
    return this._phiConsensus(votes);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Drift Detection
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Detect drift in dog voting patterns
   */
  _detectDrift(votes) {
    const alerts = [];

    for (const vote of votes) {
      if (!vote.success) continue;

      const position = this.positions.get(vote.dog);
      if (!position || position.voteCount < this.config.driftWindow / 2) continue;

      // Score drift
      const meanScore = position.getMeanScore();
      const stdDev = position.getScoreStdDev();
      const scoreDrift = Math.abs(vote.score - meanScore) / (stdDev || 1);

      if (scoreDrift > 2) { // 2 standard deviations
        alerts.push({
          type: DriftType.SCORE_DRIFT,
          dog: vote.dog,
          current: vote.score,
          expected: meanScore,
          deviation: scoreDrift,
          severity: scoreDrift > 3 ? 'high' : 'medium',
        });
        this.stats.driftAlerts++;
      }

      // Position drift (allow/block tendency change)
      const expectedBlock = position.getBlockTendency();
      const actualBlock = vote.response === 'block' ? 1 : 0;
      const positionDrift = Math.abs(actualBlock - expectedBlock);

      if (positionDrift > this.config.driftThreshold) {
        alerts.push({
          type: DriftType.POSITION_DRIFT,
          dog: vote.dog,
          current: vote.response,
          expectedBlockRate: expectedBlock,
          deviation: positionDrift,
          severity: positionDrift > 0.5 ? 'high' : 'medium',
        });
        this.stats.driftAlerts++;
      }
    }

    return alerts;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Conflict Detection & Resolution
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Detect conflicts between dogs
   */
  _detectConflicts(votes) {
    const conflicts = [];
    const successfulVotes = votes.filter(v => v.success);

    // Compare each pair of dogs
    for (let i = 0; i < successfulVotes.length; i++) {
      for (let j = i + 1; j < successfulVotes.length; j++) {
        const voteA = successfulVotes[i];
        const voteB = successfulVotes[j];

        // Check for position conflict (allow vs block)
        const positionConflict =
          (voteA.response === 'allow' && voteB.response === 'block') ||
          (voteA.response === 'block' && voteB.response === 'allow');

        // Check for score conflict (divergent scores)
        const scoreConflict =
          typeof voteA.score === 'number' &&
          typeof voteB.score === 'number' &&
          Math.abs(voteA.score - voteB.score) > 30;

        if (positionConflict || scoreConflict) {
          // Get or create conflict record
          const key = [voteA.dog, voteB.dog].sort().join(':');
          if (!this.conflicts.has(key)) {
            this.conflicts.set(key, new ConflictRecord(voteA.dog, voteB.dog));
          }

          const record = this.conflicts.get(key);
          record.recordConflict();

          if (record.conflicts >= this.config.conflictThreshold) {
            conflicts.push({
              dogs: [voteA.dog, voteB.dog],
              type: positionConflict ? 'position' : 'score',
              voteA,
              voteB,
              history: record,
            });
            this.stats.conflictsDetected++;

            // Emit conflict detected event for visibility
            this.eventBus?.publish('conflict:detected', {
              dogA: voteA.dog?.toLowerCase(),
              dogB: voteB.dog?.toLowerCase(),
              type: positionConflict ? 'position' : 'score',
              scoreA: voteA.score,
              scoreB: voteB.score,
            }, { source: 'SwarmConsensus' });
          }
        } else {
          // Record agreement
          const key = [voteA.dog, voteB.dog].sort().join(':');
          if (this.conflicts.has(key)) {
            this.conflicts.get(key).recordAgreement();
          }
        }
      }
    }

    return conflicts;
  }

  /**
   * Resolve conflicts using appropriate strategy
   */
  _resolveConflicts(result, conflicts, votes, context) {
    if (conflicts.length === 0) return result;

    // Select resolution strategy
    const strategy = this._selectResolutionStrategy(context);

    for (const conflict of conflicts) {
      const resolved = this._resolveConflict(conflict, strategy, context);
      if (resolved) {
        this.stats.conflictsResolved++;

        // Emit conflict resolved event for visibility
        this.eventBus?.publish('conflict:resolved', {
          dogA: conflict.dogs[0]?.toLowerCase(),
          dogB: conflict.dogs[1]?.toLowerCase(),
          winner: resolved.winner?.toLowerCase(),
          resolution: strategy,
        }, { source: 'SwarmConsensus' });

        // Update result based on resolution
        if (resolved.adjustedScore !== undefined) {
          result.score = (result.score + resolved.adjustedScore) / 2;
        }
        if (resolved.decision === 'block') {
          result.blocked = true;
          result.blockedBy = resolved.winner;
        }
      }
    }

    return result;
  }

  /**
   * Resolve a single conflict
   */
  _resolveConflict(conflict, strategy, context) {
    const { voteA, voteB, dogs, type } = conflict;

    switch (strategy) {
      case ConflictResolution.DOMAIN_EXPERT: {
        // Find domain experts for this context
        const domain = context.domain || 'standard';
        const experts = this.config.domainExperts[domain] || [];

        // Winner is the expert
        if (experts.includes(dogs[0])) {
          return { winner: dogs[0], adjustedScore: voteA.score, decision: voteA.response };
        }
        if (experts.includes(dogs[1])) {
          return { winner: dogs[1], adjustedScore: voteB.score, decision: voteB.response };
        }
        break;
      }

      case ConflictResolution.BLOCKING_PRIORITY: {
        // Blocking dogs win
        if (DOG_CONFIG[dogs[0]]?.blocking) {
          return { winner: dogs[0], adjustedScore: voteA.score, decision: voteA.response };
        }
        if (DOG_CONFIG[dogs[1]]?.blocking) {
          return { winner: dogs[1], adjustedScore: voteB.score, decision: voteB.response };
        }
        break;
      }

      case ConflictResolution.CONFIDENCE_WEIGHTED: {
        // Higher confidence wins
        const confA = voteA.confidence || 0.5;
        const confB = voteB.confidence || 0.5;

        if (confA > confB) {
          return { winner: dogs[0], adjustedScore: voteA.score, decision: voteA.response };
        }
        return { winner: dogs[1], adjustedScore: voteB.score, decision: voteB.response };
      }

      case ConflictResolution.HISTORICAL_ACCURACY: {
        // More accurate dog wins
        const posA = this.positions.get(dogs[0]);
        const posB = this.positions.get(dogs[1]);
        const accA = posA?.accuracy || 0.5;
        const accB = posB?.accuracy || 0.5;

        if (accA > accB) {
          return { winner: dogs[0], adjustedScore: voteA.score, decision: voteA.response };
        }
        return { winner: dogs[1], adjustedScore: voteB.score, decision: voteB.response };
      }

      case ConflictResolution.MEDIATED:
      default: {
        // Find middle ground
        const midScore = (voteA.score + voteB.score) / 2;
        const midDecision = midScore >= 50 ? 'allow' : 'block';
        return { winner: 'mediated', adjustedScore: midScore, decision: midDecision };
      }
    }

    // No resolution found
    return null;
  }

  /**
   * Select resolution strategy based on context
   */
  _selectResolutionStrategy(context) {
    const domain = context.domain || 'standard';

    // Security: blocking priority
    if (domain === 'security') {
      return ConflictResolution.BLOCKING_PRIORITY;
    }

    // Architecture: domain expert
    if (domain === 'architecture' || domain === 'deployment') {
      return ConflictResolution.DOMAIN_EXPERT;
    }

    // Default: mediated
    return ConflictResolution.MEDIATED;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Emergent Patterns
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check for emergence of new patterns
   */
  _checkEmergence(votes, result) {
    // Need enough votes
    if (votes.filter(v => v.success).length < this.config.emergenceMinVotes) {
      return null;
    }

    // Create vote signature
    const dogs = Object.keys(DOG_CONFIG);
    const signature = dogs.map(dog => {
      const vote = votes.find(v => v.dog === dog);
      if (!vote || !vote.success) return 0;
      return (vote.score || 50) / 100;
    });

    // Check if this is a new pattern
    for (const pattern of this.patterns) {
      if (pattern.matches(votes)) {
        pattern.frequency++;
        return {
          type: 'pattern_match',
          patternId: pattern.id,
          frequency: pattern.frequency,
        };
      }
    }

    // Create new pattern if result is clear
    if (result.ratio > this.config.emergenceConfidence || result.ratio < (1 - this.config.emergenceConfidence)) {
      const newPattern = new EmergentPattern({
        signature,
        confidence: Math.abs(result.ratio - 0.5) * 2 * PHI_INV,
      });
      this.patterns.push(newPattern);

      // Limit pattern storage
      if (this.patterns.length > 100) {
        // Remove least frequent patterns
        this.patterns.sort((a, b) => b.frequency - a.frequency);
        this.patterns = this.patterns.slice(0, 50);
      }

      return {
        type: 'pattern_discovered',
        patternId: newPattern.id,
        confidence: newPattern.confidence,
      };
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Strategy Selection
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Select consensus strategy based on context
   */
  _selectStrategy(context, conflicts) {
    const domain = context.domain || 'standard';

    // Security decisions: require supermajority
    if (domain === 'security' && context.severity === 'high') {
      return ConsensusStrategy.UNANIMOUS;
    }

    // Many conflicts: use emergent to find pattern
    if (conflicts.length >= 3) {
      return ConsensusStrategy.EMERGENT;
    }

    // Exploration: use median for balanced view
    if (domain === 'exploration' || context.exploratory) {
      return ConsensusStrategy.WEIGHTED_MEDIAN;
    }

    // Default: φ-supermajority
    return ConsensusStrategy.PHI_SUPERMAJORITY;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Utility Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Calculate weighted average score
   */
  _calculateWeightedScore(votes) {
    let totalWeight = 0;
    let weightedSum = 0;

    for (const vote of votes) {
      if (typeof vote.score !== 'number') continue;
      const weight = vote.weight || 1;
      weightedSum += vote.score * weight;
      totalWeight += weight;
    }

    return totalWeight > 0 ? weightedSum / totalWeight : 50;
  }

  /**
   * Record feedback for learning
   */
  recordFeedback(decisionId, wasCorrect, details = {}) {
    // Update dog accuracy based on feedback
    if (details.votes) {
      for (const vote of details.votes) {
        const position = this.positions.get(vote.dog);
        if (position) {
          // Dog was correct if their vote aligned with the outcome
          const voteAligned = (wasCorrect && vote.response === 'allow') ||
                             (!wasCorrect && vote.response === 'block');
          position.updateAccuracy(voteAligned);
        }
      }
    }

    // Update pattern outcomes
    if (details.patternId) {
      const pattern = this.patterns.find(p => p.id === details.patternId);
      if (pattern) {
        pattern.recordOutcome(details.outcome, wasCorrect);
      }
    }
  }

  /**
   * Get quorum requirement for a domain
   */
  getQuorum(domain) {
    return this.config.quorum[domain] || this.config.quorum.standard;
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      positions: Array.from(this.positions.entries()).map(([id, pos]) => ({
        dog: id,
        voteCount: pos.voteCount,
        accuracy: pos.accuracy,
        meanScore: pos.getMeanScore(),
        blockTendency: pos.getBlockTendency(),
      })),
      patterns: this.patterns.length,
      activeConflicts: this.conflicts.size,
    };
  }

  /**
   * Export state for persistence
   */
  exportState() {
    return {
      positions: Array.from(this.positions.entries()).map(([id, pos]) => ({
        dogId: id,
        scoreHistory: pos.scoreHistory,
        positionHistory: pos.positionHistory,
        accuracy: pos.accuracy,
        voteCount: pos.voteCount,
      })),
      patterns: this.patterns.map(p => ({
        id: p.id,
        signature: p.signature,
        frequency: p.frequency,
        confidence: p.confidence,
        outcomes: p.outcomes,
      })),
      stats: this.stats,
    };
  }

  /**
   * Import state from persistence
   */
  importState(state) {
    if (state.positions) {
      for (const posData of state.positions) {
        const position = this.positions.get(posData.dogId);
        if (position) {
          position.scoreHistory = posData.scoreHistory || [];
          position.positionHistory = posData.positionHistory || [];
          position.accuracy = posData.accuracy || 0.5;
          position.voteCount = posData.voteCount || 0;
        }
      }
    }

    if (state.patterns) {
      this.patterns = state.patterns.map(p => {
        const pattern = new EmergentPattern(p);
        pattern.frequency = p.frequency || 1;
        pattern.confidence = p.confidence || 0;
        pattern.outcomes = p.outcomes || [];
        return pattern;
      });
    }

    if (state.stats) {
      this.stats = { ...this.stats, ...state.stats };
    }
  }
}

/**
 * Create swarm consensus instance
 */
export function createSwarmConsensus(options = {}) {
  return new SwarmConsensus(options);
}

// Singleton for global access
let _swarmInstance = null;

/**
 * Get or create global swarm consensus
 */
export function getSwarmConsensus(options = {}) {
  if (!_swarmInstance) {
    _swarmInstance = createSwarmConsensus(options);
  }
  return _swarmInstance;
}

export default SwarmConsensus;
