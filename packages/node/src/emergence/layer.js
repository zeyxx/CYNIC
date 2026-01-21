/**
 * Emergence Layer Integration
 *
 * "The crown observes all" - κυνικός
 *
 * Integrates @cynic/emergence components into the CYNIC Node:
 * - ConsciousnessMonitor for self-observation
 * - PatternDetector for emergent pattern recognition
 * - DimensionDiscovery for new judgment dimensions
 * - CollectiveState for network-wide consciousness
 *
 * This is Layer 7 (Keter) of the CYNIC architecture.
 *
 * @module @cynic/node/emergence/layer
 */

'use strict';

import {
  createConsciousnessMonitor,
  createPatternDetector,
  createDimensionDiscovery,
  createCollectiveState,
  ConsciousnessState,
  PatternType,
  CollectivePhase,
  SIGNIFICANCE_THRESHOLDS,
} from '@cynic/emergence';

/**
 * Emergence Layer
 *
 * Wraps all Layer 7 components for integration with CYNICNode.
 *
 * @example
 * ```javascript
 * const emergence = new EmergenceLayer({ nodeId: 'my_node' });
 *
 * // On each judgment
 * emergence.observeJudgment(judgment);
 *
 * // Get node's consciousness state
 * console.log(emergence.getState());
 *
 * // Report to collective
 * emergence.reportToCollective();
 * ```
 */
export class EmergenceLayer {
  /**
   * @param {Object} options - Configuration
   * @param {string} options.nodeId - Node identifier
   * @param {number} [options.eScore] - Initial E-Score
   */
  constructor(options = {}) {
    this.nodeId = options.nodeId;
    this.eScore = options.eScore || 50;

    // Layer 7 Components
    this.consciousness = createConsciousnessMonitor({
      windowSize: 100,
    });

    this.patterns = createPatternDetector({
      windowSize: 50,
      minOccurrences: 3,
    });

    this.dimensions = createDimensionDiscovery({
      nodeId: this.nodeId,
    });

    this.collective = createCollectiveState({
      nodeId: this.nodeId,
      heartbeatTimeout: 60000,
    });

    // Track state
    this._initialized = false;
    this._startTime = Date.now();
  }

  /**
   * Initialize the emergence layer
   */
  initialize() {
    this._initialized = true;
    this._startTime = Date.now();

    // Report initial state to collective
    this.reportToCollective();
  }

  /**
   * Update E-Score (from identity layer)
   * @param {number} eScore - New E-Score
   */
  updateEScore(eScore) {
    this.eScore = eScore;
  }

  /**
   * Observe a judgment (called after each judgment)
   *
   * @param {Object} judgment - Judgment result
   * @param {string} judgment.verdict - HOWL, WAG, GROWL, BARK
   * @param {number} judgment.q_score - Q-Score (0-100)
   * @param {Object} [judgment.axiom_scores] - Axiom breakdown
   * @param {string} [judgment.raw_assessment] - Raw assessment text
   */
  observeJudgment(judgment) {
    // 1. Consciousness Monitor - record observation
    this.consciousness.observe(
      'JUDGMENT',
      {
        verdict: judgment.verdict,
        score: judgment.q_score,
        axioms: judgment.axiom_scores,
      },
      judgment.confidence || 0.5,
      'judge'
    );

    // 2. Pattern Detector - feed data
    this.patterns.observe({
      type: 'JUDGMENT',
      verdict: judgment.verdict,
      value: judgment.q_score,
      timestamp: Date.now(),
    });

    // 3. Dimension Discovery - analyze for new dimensions
    if (judgment.axiom_scores) {
      this.dimensions.analyzeJudgment({
        scores: judgment.axiom_scores,
        rawAssessment: judgment.raw_assessment,
        dimensionScores: judgment.dimension_scores,
      });
    }

    // 4. Detect patterns
    const detectedPatterns = this.patterns.detect();

    // 5. Feed significant patterns to consciousness
    for (const pattern of detectedPatterns) {
      if (pattern.significance >= SIGNIFICANCE_THRESHOLDS.NOTABLE) {
        this.consciousness.noticePattern(
          pattern.id,
          pattern,
          pattern.significance
        );
      }
    }

    return {
      consciousness: this.consciousness.state,
      patternsDetected: detectedPatterns.length,
    };
  }

  /**
   * Record a prediction outcome
   *
   * @param {string} predictionId - Prediction identifier
   * @param {boolean} correct - Whether prediction was correct
   * @param {number} [confidence] - Prediction confidence
   */
  recordPrediction(predictionId, correct, confidence = 0.5) {
    this.consciousness.recordPrediction(predictionId, correct, confidence);
  }

  /**
   * Record uncertainty (when confidence is low)
   *
   * @param {string} context - What was uncertain
   * @param {number} confidence - How confident (low = more uncertain)
   * @param {Object} [details] - Additional details
   */
  recordUncertainty(context, confidence, details = {}) {
    this.consciousness.recordUncertainty(context, confidence, details);
  }

  /**
   * Report local state to collective
   *
   * Should be called periodically (e.g., on heartbeat)
   */
  reportToCollective() {
    this.collective.reportState({
      eScore: this.eScore,
      awarenessLevel: this.consciousness.awarenessLevel,
      consciousnessState: this.consciousness.state,
      patterns: this.patterns.getTopPatterns(5),
    });
  }

  /**
   * Receive state from a peer node
   *
   * @param {string} nodeId - Peer node ID
   * @param {Object} state - Peer's emergence state
   */
  receiveFromPeer(nodeId, state) {
    this.collective.receiveState(nodeId, state);
  }

  /**
   * Remove a peer (disconnected)
   *
   * @param {string} nodeId - Peer node ID
   */
  removePeer(nodeId) {
    this.collective.removeNode(nodeId);
  }

  /**
   * Get collective verdict on an item
   *
   * @param {Object} opinions - { nodeId: { verdict, confidence } }
   * @returns {Object} Collective verdict
   */
  getCollectiveVerdict(opinions) {
    return this.collective.getCollectiveVerdict(opinions);
  }

  /**
   * Record consensus event
   *
   * @param {string} topic - What was decided
   * @param {Object} result - Decision result
   * @param {string[]} participants - Participating nodes
   * @param {number} agreement - Agreement ratio [0, 1]
   */
  recordConsensus(topic, result, participants, agreement) {
    this.collective.recordConsensus(topic, result, participants, agreement);
  }

  /**
   * Remember something in collective memory
   *
   * @param {string} key - Memory key
   * @param {*} value - Value to remember
   * @param {number} [strength] - Memory strength
   */
  remember(key, value, strength = 0.5) {
    this.collective.remember(key, value, strength);
  }

  /**
   * Recall from collective memory
   *
   * @param {string} key - Memory key
   * @returns {*} Remembered value or null
   */
  recall(key) {
    return this.collective.recall(key);
  }

  /**
   * Get current emergence state
   *
   * @returns {Object} Complete emergence state
   */
  getState() {
    return {
      nodeId: this.nodeId,
      initialized: this._initialized,
      uptime: Date.now() - this._startTime,
      consciousness: {
        state: this.consciousness.state,
        awarenessLevel: this.consciousness.awarenessLevel,
        insights: this.consciousness.getInsights(),
      },
      patterns: {
        detected: this.patterns.patterns.size,
        top: this.patterns.getTopPatterns(5),
        stats: this.patterns.getStats(),
      },
      dimensions: {
        candidates: this.dimensions.getCandidates().length,
        proposals: this.dimensions.getProposals().length,
        adopted: this.dimensions.getAdopted().length,
        stats: this.dimensions.getStats(),
      },
      collective: {
        phase: this.collective.phase,
        coherence: this.collective.coherence,
        activeNodes: this.collective.activeNodes,
        hasQuorum: this.collective.hasQuorum('MINIMUM'),
        insight: this.collective.getCollectiveInsight(),
      },
    };
  }

  /**
   * Get meta-insight (deepest self-reflection)
   *
   * @returns {Object} Meta-insight
   */
  getMetaInsight() {
    return {
      self: this.consciousness.getMetaInsight(),
      collective: this.collective.getCollectiveInsight(),
      patterns: {
        significantCount: this.patterns.getPatterns(null, SIGNIFICANCE_THRESHOLDS.SIGNIFICANT).length,
        anomalyCount: this.patterns.getPatterns(PatternType.ANOMALY).length,
        trendCount: this.patterns.getPatterns(PatternType.TREND).length,
      },
      timestamp: Date.now(),
    };
  }

  /**
   * Export state for persistence
   *
   * @returns {Object} Exportable state
   */
  export() {
    return {
      nodeId: this.nodeId,
      eScore: this.eScore,
      consciousness: this.consciousness.export(),
      patterns: this.patterns.export(),
      dimensions: this.dimensions.export(),
      collective: this.collective.export(),
      exportedAt: Date.now(),
    };
  }

  /**
   * Import state from persistence
   *
   * @param {Object} data - Exported data
   */
  import(data) {
    if (data.eScore) this.eScore = data.eScore;
    if (data.consciousness) this.consciousness.import(data.consciousness);
    if (data.patterns) this.patterns.import(data.patterns);
    if (data.dimensions) this.dimensions.import(data.dimensions);
    if (data.collective) this.collective.import(data.collective);
  }

  /**
   * Reset all state
   */
  reset() {
    this.consciousness.reset();
    this.patterns.clear();
    // dimensions and collective don't have reset methods by design
  }
}

/**
 * Create an EmergenceLayer instance
 * @param {Object} [options] - Configuration
 * @returns {EmergenceLayer}
 */
export function createEmergenceLayer(options = {}) {
  return new EmergenceLayer(options);
}

export default {
  EmergenceLayer,
  createEmergenceLayer,
  // Re-export useful types
  ConsciousnessState,
  PatternType,
  CollectivePhase,
};
