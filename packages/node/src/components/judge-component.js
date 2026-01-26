/**
 * JudgeComponent - Judgment Domain
 *
 * Encapsulates CYNICJudge, ResidualDetector, and LearningService.
 * Part of CYNICNode decomposition (Phase 2, #28).
 *
 * "Judge with wisdom, learn from error" - κυνικός
 *
 * @module @cynic/node/components/judge-component
 */

'use strict';

import { EventEmitter } from 'events';
import { CYNICJudge } from '../judge/judge.js';
import { ResidualDetector } from '../judge/residual.js';
import { LearningService } from '../judge/learning-service.js';

/**
 * Judge Component - manages judgment creation and learning
 *
 * Single Responsibility: Judgment lifecycle, anomaly detection, RLHF learning
 */
export class JudgeComponent extends EventEmitter {
  /**
   * Create judge component
   *
   * @param {Object} options - Component options
   * @param {Object} [options.persistence] - Persistence config
   * @param {number} [options.learningRate=0.236] - RLHF learning rate (φ⁻³)
   * @param {Object} [options.sharedMemory] - SharedMemory for pattern storage
   */
  constructor(options = {}) {
    super();

    // Initialize judge
    this._judge = new CYNICJudge();

    // Initialize residual detector
    this._residualDetector = new ResidualDetector();

    // Initialize learning service
    this._learningService = new LearningService({
      persistence: options.persistence,
      learningRate: options.learningRate || 0.236, // φ⁻³
    });

    // Wire learning service to judge
    this._judge.setLearningService(this._learningService);

    // SharedMemory for anomaly patterns
    this._sharedMemory = options.sharedMemory || null;

    // Stats
    this._stats = {
      judgmentsMade: 0,
      anomaliesDetected: 0,
      learningCycles: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize the learning service
   * @returns {Promise<void>}
   */
  async initialize() {
    await this._learningService.init();
    console.log('[JudgeComponent] Learning service initialized');
  }

  /**
   * Set shared memory for anomaly patterns
   * @param {Object} sharedMemory - SharedMemory instance
   */
  setSharedMemory(sharedMemory) {
    this._sharedMemory = sharedMemory;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Judgment Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a judgment for an item
   *
   * @param {Object} item - Item to judge
   * @param {Object} [context={}] - Judgment context
   * @returns {Object} Judgment result with anomaly analysis
   */
  judge(item, context = {}) {
    // Create judgment
    const judgment = this._judge.judge(item, context);

    // Analyze for residuals (anomalies)
    const residualAnalysis = this._residualDetector.analyze(judgment, context);

    // Track anomaly
    if (residualAnalysis.isAnomaly) {
      this._stats.anomaliesDetected++;
      this._processAnomaly(judgment, residualAnalysis);
    }

    this._stats.judgmentsMade++;

    // Emit event
    this.emit('judgment:created', {
      judgment,
      residualAnalysis,
      timestamp: Date.now(),
    });

    return {
      judgment,
      residualAnalysis,
    };
  }

  /**
   * Process an anomaly signal
   *
   * @param {Object} judgment - The judgment
   * @param {Object} residualAnalysis - Residual analysis result
   * @private
   */
  _processAnomaly(judgment, residualAnalysis) {
    // Feed to learning service
    this._learningService.processAnomalySignal?.({
      judgmentId: judgment.id,
      residual: residualAnalysis.residual,
      threshold: this._residualDetector.threshold,
      dimensions: judgment.dimensions,
      verdict: judgment.verdict,
      qScore: judgment.q_score || judgment.global_score,
      source: 'residual_detector',
      timestamp: Date.now(),
    });

    // Add to shared memory as pattern
    if (this._sharedMemory) {
      this._sharedMemory.addPattern?.({
        type: 'anomaly_detected',
        judgmentId: judgment.id,
        residual: residualAnalysis.residual,
        timestamp: Date.now(),
      });
    }

    // Emit anomaly event
    this.emit('anomaly:detected', {
      judgmentId: judgment.id,
      residual: residualAnalysis.residual,
      threshold: this._residualDetector.threshold,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Learning Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process feedback for a judgment
   *
   * @param {Object} feedback - Feedback data
   * @param {string} feedback.judgmentId - Judgment ID
   * @param {string} feedback.feedbackType - 'positive' or 'negative'
   * @param {string} [feedback.source] - Feedback source
   * @param {number} [feedback.confidence] - Feedback confidence
   * @param {Object} [feedback.details] - Additional details
   */
  processFeedback(feedback) {
    this._learningService.processFeedback(feedback);
  }

  /**
   * Wire learning events to external handlers
   *
   * @param {Object} handlers - Event handlers
   * @param {Function} [handlers.onFeedbackProcessed] - (result) => void
   * @param {Function} [handlers.onLearningComplete] - (result) => void
   * @param {Function} [handlers.onLearningAdded] - (entry) => void
   */
  wireLearningEvents(handlers = {}) {
    this._learningService.on('feedback-processed', (result) => {
      this.emit('learning:feedback-processed', result);
      handlers.onFeedbackProcessed?.(result);
    });

    this._learningService.on('learning-complete', (result) => {
      this._stats.learningCycles++;
      this.emit('learning:complete', result);
      handlers.onLearningComplete?.(result);
    });

    this._learningService.on('learning-added', (entry) => {
      this.emit('learning:added', entry);
      handlers.onLearningAdded?.(entry);
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Collective Feedback Integration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process collective consensus as feedback
   *
   * @param {Object} consensus - Collective consensus data
   * @param {string} consensus.vote - Vote (APPROVE, WAG, GROWL, etc.)
   * @param {number} consensus.confidence - Consensus confidence
   * @param {Array} consensus.voters - Voters (dogs)
   * @param {Object} consensus.context - Context with judgmentId
   */
  processCollectiveConsensus(consensus) {
    const { vote, confidence, voters, context } = consensus;

    // Only process judgment-related consensus
    if (context?.type !== 'judgment') return;

    const isCorrect = vote === 'APPROVE' || vote === 'WAG';
    const feedbackType = isCorrect ? 'positive' : 'negative';

    this.processFeedback({
      judgmentId: context.judgmentId,
      feedbackType,
      source: 'collective_consensus',
      confidence,
      details: {
        vote,
        voters,
        dogCount: voters?.length || 0,
      },
    });

    console.log(`[JudgeComponent] Collective feedback: ${vote} (${Math.round(confidence * 100)}%)`);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats & Info
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get judge stats
   * @returns {Object} Stats
   */
  getJudgeStats() {
    return this._judge.getStats();
  }

  /**
   * Get residual detector stats
   * @returns {Object} Stats
   */
  getResidualStats() {
    return this._residualDetector.getStats();
  }

  /**
   * Get component stats
   * @returns {Object} Combined stats
   */
  getStats() {
    return {
      ...this._stats,
      judge: this.getJudgeStats(),
      residual: this.getResidualStats(),
    };
  }

  /**
   * Get component info
   * @returns {Object} Component info
   */
  getInfo() {
    return {
      stats: this.getStats(),
      residualThreshold: this._residualDetector.threshold,
      learningRate: this._learningService.learningRate,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Backward Compatibility
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get raw judge
   * @returns {CYNICJudge} Judge
   * @deprecated Use component methods instead
   */
  get judge() {
    return this._judge;
  }

  /**
   * Get raw residual detector
   * @returns {ResidualDetector} Residual detector
   * @deprecated Use component methods instead
   */
  get residualDetector() {
    return this._residualDetector;
  }

  /**
   * Get raw learning service
   * @returns {LearningService} Learning service
   * @deprecated Use component methods instead
   */
  get learningService() {
    return this._learningService;
  }
}

export default JudgeComponent;
