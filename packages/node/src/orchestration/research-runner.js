/**
 * ResearchRunner — Execute Structured Research Protocols
 *
 * Scientific methodology for CYNIC's investigations:
 *
 * QUESTION → DESIGN → EXECUTE → ANALYZE → CONCLUDE
 *    ↓         ↓         ↓          ↓         ↓
 *  Goal    Protocol   Steps    Evidence   Report
 *
 * Each protocol is a structured investigation following φ-bounded science:
 * - Hypotheses tested with max 61.8% confidence claims
 * - Evidence collected systematically
 * - Conclusions drawn from reproducible observations
 * - Bias detection and mitigation
 *
 * "Question everything. Trust only what you can verify." — κυνικός
 *
 * @module @cynic/node/orchestration/research-runner
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2, globalEventBus } from '@cynic/core';

const log = createLogger('ResearchRunner');

// ═══════════════════════════════════════════════════════════════════════════
// PROTOCOL TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Research protocol types
 */
export const ProtocolType = {
  EXPLORATORY: 'exploratory',     // Open-ended investigation
  COMPARATIVE: 'comparative',     // Compare alternatives
  VALIDATION: 'validation',       // Test hypothesis
  REPLICATION: 'replication',     // Reproduce prior findings
  META: 'meta',                   // Analyze prior research
};

/**
 * Research step types
 */
export const StepType = {
  OBSERVE: 'observe',             // Collect raw observations
  MEASURE: 'measure',             // Quantitative measurement
  ANALYZE: 'analyze',             // Process data
  COMPARE: 'compare',             // Cross-reference
  SYNTHESIZE: 'synthesize',       // Combine findings
  VALIDATE: 'validate',           // Check reproducibility
};

/**
 * Evidence quality levels (φ-bounded)
 */
export const EvidenceQuality = {
  STRONG: 'strong',               // ≥ φ⁻¹ (61.8%)
  MODERATE: 'moderate',           // φ⁻² to φ⁻¹ (38.2% - 61.8%)
  WEAK: 'weak',                   // < φ⁻² (38.2%)
  INCONCLUSIVE: 'inconclusive',   // Insufficient data
};

// ═══════════════════════════════════════════════════════════════════════════
// RESEARCH ENTITIES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * ResearchStep — one step in a research protocol
 */
export class ResearchStep {
  constructor(data = {}) {
    this.id = data.id || `step-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.type = data.type || StepType.OBSERVE;
    this.description = data.description || '';
    this.method = data.method || null; // Function to execute
    this.expectedDuration = data.expectedDuration || 1000; // ms
    this.completed = false;
    this.startTime = null;
    this.endTime = null;
    this.observations = [];
    this.errors = [];
  }

  get duration() {
    if (!this.startTime) return 0;
    return (this.endTime || Date.now()) - this.startTime;
  }

  toJSON() {
    return {
      id: this.id,
      type: this.type,
      description: this.description,
      completed: this.completed,
      duration: this.duration,
      observationCount: this.observations.length,
      errorCount: this.errors.length,
    };
  }
}

/**
 * Evidence — a piece of supporting data
 */
export class Evidence {
  constructor(data = {}) {
    this.id = data.id || `evidence-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.timestamp = data.timestamp || Date.now();
    this.source = data.source || 'unknown';
    this.content = data.content || '';
    this.quality = data.quality || EvidenceQuality.WEAK;
    this.confidence = Math.min(data.confidence || 0, PHI_INV); // φ-bounded
    this.metadata = data.metadata || {};
  }

  /**
   * Classify quality based on confidence
   */
  classifyQuality() {
    if (this.confidence >= PHI_INV) {
      this.quality = EvidenceQuality.STRONG;
    } else if (this.confidence >= PHI_INV_2) {
      this.quality = EvidenceQuality.MODERATE;
    } else if (this.confidence > 0) {
      this.quality = EvidenceQuality.WEAK;
    } else {
      this.quality = EvidenceQuality.INCONCLUSIVE;
    }
    return this.quality;
  }

  toJSON() {
    return {
      id: this.id,
      source: this.source,
      quality: this.quality,
      confidence: Math.round(this.confidence * 1000) / 1000,
      timestamp: this.timestamp,
    };
  }
}

/**
 * ResearchProtocol — structured investigation plan
 */
export class ResearchProtocol {
  constructor(data = {}) {
    this.id = data.id || `protocol-${Date.now()}`;
    this.type = data.type || ProtocolType.EXPLORATORY;
    this.question = data.question || '';
    this.hypothesis = data.hypothesis || null;
    this.steps = data.steps || [];
    this.evidence = [];
    this.conclusions = [];
    this.metadata = data.metadata || {};
    this.startTime = null;
    this.endTime = null;
    this.status = 'pending'; // pending, running, completed, failed
  }

  get duration() {
    if (!this.startTime) return 0;
    return (this.endTime || Date.now()) - this.startTime;
  }

  get completedSteps() {
    return this.steps.filter(s => s.completed).length;
  }

  get progress() {
    if (this.steps.length === 0) return 0;
    return this.completedSteps / this.steps.length;
  }

  toJSON() {
    return {
      id: this.id,
      type: this.type,
      question: this.question,
      hypothesis: this.hypothesis,
      status: this.status,
      progress: Math.round(this.progress * 100),
      duration: this.duration,
      stepCount: this.steps.length,
      completedSteps: this.completedSteps,
      evidenceCount: this.evidence.length,
      conclusionCount: this.conclusions.length,
    };
  }
}

/**
 * ResearchReport — synthesized findings
 */
export class ResearchReport {
  constructor(protocol) {
    this.id = `report-${protocol.id}`;
    this.protocolId = protocol.id;
    this.timestamp = Date.now();
    this.question = protocol.question;
    this.hypothesis = protocol.hypothesis;
    this.type = protocol.type;
    this.duration = protocol.duration;

    // Evidence summary
    this.evidenceSummary = {
      total: protocol.evidence.length,
      strong: protocol.evidence.filter(e => e.quality === EvidenceQuality.STRONG).length,
      moderate: protocol.evidence.filter(e => e.quality === EvidenceQuality.MODERATE).length,
      weak: protocol.evidence.filter(e => e.quality === EvidenceQuality.WEAK).length,
      inconclusive: protocol.evidence.filter(e => e.quality === EvidenceQuality.INCONCLUSIVE).length,
    };

    // Overall confidence (φ-bounded)
    this.confidence = this._calculateConfidence(protocol.evidence);

    // Conclusions
    this.conclusions = protocol.conclusions;

    // Reproducibility score (φ-bounded)
    this.reproducibility = this._calculateReproducibility(protocol);
  }

  /**
   * Calculate overall confidence from evidence
   * φ-bounded: max 61.8%
   */
  _calculateConfidence(evidence) {
    if (evidence.length === 0) return 0;

    // Weight by quality
    const weights = {
      [EvidenceQuality.STRONG]: 1.0,
      [EvidenceQuality.MODERATE]: PHI_INV,
      [EvidenceQuality.WEAK]: PHI_INV_2,
      [EvidenceQuality.INCONCLUSIVE]: 0,
    };

    let totalWeight = 0;
    let weightedSum = 0;

    for (const e of evidence) {
      const weight = weights[e.quality] || 0;
      weightedSum += e.confidence * weight;
      totalWeight += weight;
    }

    const confidence = totalWeight > 0 ? weightedSum / totalWeight : 0;
    return Math.min(confidence, PHI_INV); // φ-bounded
  }

  /**
   * Calculate reproducibility score
   * φ-bounded: max 61.8%
   */
  _calculateReproducibility(protocol) {
    // Factors: step completion, evidence quality, error rate
    const completionRate = protocol.progress;
    const errorRate = protocol.steps.reduce((sum, s) => sum + s.errors.length, 0) / protocol.steps.length;

    const strongEvidence = this.evidenceSummary.strong / (this.evidenceSummary.total || 1);

    // Weighted average (φ-bounded components)
    const score = (
      completionRate * PHI_INV +
      (1 - errorRate) * PHI_INV_2 +
      strongEvidence * (1 - PHI_INV)
    ) / 3;

    return Math.min(score, PHI_INV); // φ-bounded
  }

  toJSON() {
    return {
      id: this.id,
      question: this.question,
      type: this.type,
      confidence: Math.round(this.confidence * 1000) / 1000,
      reproducibility: Math.round(this.reproducibility * 1000) / 1000,
      evidenceSummary: this.evidenceSummary,
      conclusions: this.conclusions,
      duration: this.duration,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESEARCH RUNNER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * ResearchRunner — executes research protocols
 *
 * Orchestrates:
 * - Protocol design
 * - Step-by-step execution
 * - Evidence collection
 * - Report generation
 */
export class ResearchRunner extends EventEmitter {
  constructor(options = {}) {
    super();

    // Configuration
    this.config = {
      maxConcurrentProtocols: options.maxConcurrentProtocols || 3,
      defaultTimeout: options.defaultTimeout || 30000, // 30s per step
      enableValidation: options.enableValidation !== false,
    };

    // State
    this.protocols = new Map(); // protocolId → ResearchProtocol
    this.reports = new Map();   // reportId → ResearchReport
    this.stats = {
      totalProtocols: 0,
      completedProtocols: 0,
      failedProtocols: 0,
      totalEvidence: 0,
      avgConfidence: 0,
    };

    log.info('ResearchRunner created', this.config);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // CORE API
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Create a new research protocol
   *
   * @param {Object} options - Protocol options
   * @param {string} options.type - Protocol type
   * @param {string} options.question - Research question
   * @param {string} [options.hypothesis] - Hypothesis to test
   * @param {ResearchStep[]} options.steps - Research steps
   * @returns {ResearchProtocol}
   */
  createProtocol(options) {
    const protocol = new ResearchProtocol({
      type: options.type,
      question: options.question,
      hypothesis: options.hypothesis,
      steps: options.steps || [],
      metadata: options.metadata || {},
    });

    this.protocols.set(protocol.id, protocol);
    this.stats.totalProtocols++;

    this.emit('protocol:created', protocol.toJSON());
    log.info('Protocol created', { id: protocol.id, type: protocol.type });

    return protocol;
  }

  /**
   * Execute a research protocol
   *
   * @param {string|ResearchProtocol} protocolOrId - Protocol or ID
   * @returns {Promise<ResearchReport>}
   */
  async execute(protocolOrId) {
    const protocol = typeof protocolOrId === 'string'
      ? this.protocols.get(protocolOrId)
      : protocolOrId;

    if (!protocol) {
      throw new Error('Protocol not found');
    }

    if (protocol.status === 'running') {
      throw new Error('Protocol already running');
    }

    protocol.status = 'running';
    protocol.startTime = Date.now();

    this.emit('protocol:started', protocol.toJSON());
    log.info('Protocol execution started', { id: protocol.id });

    try {
      // Execute steps sequentially
      for (const step of protocol.steps) {
        await this._executeStep(step, protocol);
      }

      // Analyze evidence
      await this._analyzeEvidence(protocol);

      // Generate conclusions
      await this._generateConclusions(protocol);

      // Complete protocol
      protocol.status = 'completed';
      protocol.endTime = Date.now();
      this.stats.completedProtocols++;

      // Generate report
      const report = new ResearchReport(protocol);
      this.reports.set(report.id, report);

      // Update stats
      this._updateStats(protocol);

      this.emit('protocol:completed', { protocol: protocol.toJSON(), report: report.toJSON() });
      log.info('Protocol execution completed', {
        id: protocol.id,
        duration: protocol.duration,
        confidence: report.confidence,
      });

      return report;
    } catch (err) {
      protocol.status = 'failed';
      protocol.endTime = Date.now();
      this.stats.failedProtocols++;

      this.emit('protocol:failed', { protocol: protocol.toJSON(), error: err.message });
      log.error('Protocol execution failed', { id: protocol.id, error: err.message });

      throw err;
    }
  }

  /**
   * Get protocol by ID
   */
  getProtocol(protocolId) {
    return this.protocols.get(protocolId);
  }

  /**
   * Get report by ID
   */
  getReport(reportId) {
    return this.reports.get(reportId);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // EXECUTION STAGES
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Execute a single research step
   */
  async _executeStep(step, protocol) {
    this.emit('step:started', { protocolId: protocol.id, step: step.toJSON() });

    step.startTime = Date.now();

    try {
      // Execute step method if provided
      if (typeof step.method === 'function') {
        const result = await Promise.race([
          step.method({ protocol, step }),
          this._timeout(this.config.defaultTimeout),
        ]);

        // Store observations
        if (result) {
          if (Array.isArray(result)) {
            step.observations.push(...result);
          } else {
            step.observations.push(result);
          }
        }
      }

      // Convert observations to evidence
      for (const obs of step.observations) {
        const evidence = new Evidence({
          source: step.id,
          content: obs,
          confidence: obs.confidence || PHI_INV_2, // Default moderate
        });
        evidence.classifyQuality();
        protocol.evidence.push(evidence);
      }

      step.completed = true;
      step.endTime = Date.now();

      this.emit('step:completed', {
        protocolId: protocol.id,
        step: step.toJSON(),
        evidenceCount: step.observations.length,
      });
    } catch (err) {
      step.errors.push(err.message);
      step.endTime = Date.now();

      this.emit('step:failed', {
        protocolId: protocol.id,
        step: step.toJSON(),
        error: err.message,
      });

      // Don't throw — continue with other steps
      log.warn('Step failed', { stepId: step.id, error: err.message });
    }
  }

  /**
   * Analyze collected evidence
   */
  async _analyzeEvidence(protocol) {
    this.emit('analysis:started', { protocolId: protocol.id });

    // Group evidence by quality
    const byQuality = {
      [EvidenceQuality.STRONG]: [],
      [EvidenceQuality.MODERATE]: [],
      [EvidenceQuality.WEAK]: [],
      [EvidenceQuality.INCONCLUSIVE]: [],
    };

    for (const evidence of protocol.evidence) {
      byQuality[evidence.quality].push(evidence);
    }

    // Detect contradictions
    const contradictions = this._detectContradictions(protocol.evidence);
    if (contradictions.length > 0) {
      log.warn('Contradictions detected', {
        protocolId: protocol.id,
        count: contradictions.length,
      });
      protocol.metadata.contradictions = contradictions;
    }

    this.emit('analysis:completed', {
      protocolId: protocol.id,
      evidenceByQuality: Object.fromEntries(
        Object.entries(byQuality).map(([k, v]) => [k, v.length])
      ),
    });
  }

  /**
   * Generate conclusions from evidence
   */
  async _generateConclusions(protocol) {
    this.emit('conclusions:started', { protocolId: protocol.id });

    const conclusions = [];

    // Hypothesis validation (if applicable)
    if (protocol.hypothesis && protocol.type === ProtocolType.VALIDATION) {
      const strongEvidence = protocol.evidence.filter(e => e.quality === EvidenceQuality.STRONG);
      const supported = strongEvidence.length >= protocol.evidence.length * PHI_INV_2;

      conclusions.push({
        type: 'hypothesis-validation',
        hypothesis: protocol.hypothesis,
        supported,
        confidence: Math.min(strongEvidence.length / (protocol.evidence.length || 1), PHI_INV),
        evidence: strongEvidence.map(e => e.id),
      });
    }

    // General findings
    const avgConfidence = protocol.evidence.reduce((sum, e) => sum + e.confidence, 0) / (protocol.evidence.length || 1);
    conclusions.push({
      type: 'general-findings',
      summary: `Research completed with ${protocol.evidence.length} pieces of evidence`,
      confidence: Math.min(avgConfidence, PHI_INV),
      quality: avgConfidence >= PHI_INV ? EvidenceQuality.STRONG :
               avgConfidence >= PHI_INV_2 ? EvidenceQuality.MODERATE :
               EvidenceQuality.WEAK,
    });

    protocol.conclusions = conclusions;

    this.emit('conclusions:completed', {
      protocolId: protocol.id,
      conclusionCount: conclusions.length,
    });
  }

  /**
   * Detect contradictions in evidence
   */
  _detectContradictions(evidence) {
    // Simple contradiction detection: evidence with high confidence but opposing claims
    // (This is a stub — real implementation would use semantic analysis)
    const contradictions = [];

    // Group by source
    const bySource = new Map();
    for (const e of evidence) {
      if (!bySource.has(e.source)) {
        bySource.set(e.source, []);
      }
      bySource.get(e.source).push(e);
    }

    // Check for opposing high-confidence claims from same source
    for (const [source, items] of bySource.entries()) {
      const highConf = items.filter(e => e.confidence >= PHI_INV);
      if (highConf.length > 1) {
        // Potential contradiction
        contradictions.push({
          source,
          evidenceIds: highConf.map(e => e.id),
        });
      }
    }

    return contradictions;
  }

  /**
   * Promise that rejects after timeout
   */
  _timeout(ms) {
    return new Promise((_, reject) =>
      setTimeout(() => reject(new Error('Step timeout')), ms)
    );
  }

  /**
   * Update aggregate statistics
   */
  _updateStats(protocol) {
    this.stats.totalEvidence += protocol.evidence.length;

    // Update average confidence (EMA with α = φ⁻¹)
    const protocolConfidence = protocol.evidence.reduce((sum, e) => sum + e.confidence, 0) / (protocol.evidence.length || 1);
    if (this.stats.avgConfidence === 0) {
      this.stats.avgConfidence = protocolConfidence;
    } else {
      this.stats.avgConfidence =
        PHI_INV * protocolConfidence + (1 - PHI_INV) * this.stats.avgConfidence;
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STATS & HEALTH
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Get runner statistics
   */
  getStats() {
    return {
      ...this.stats,
      activeProtocols: Array.from(this.protocols.values()).filter(p => p.status === 'running').length,
      totalReports: this.reports.size,
      avgConfidence: Math.round(this.stats.avgConfidence * 1000) / 1000,
    };
  }

  /**
   * Health check
   */
  async health() {
    return {
      protocolCount: this.protocols.size,
      reportCount: this.reports.size,
      avgConfidence: this.stats.avgConfidence,
      successRate: this.stats.totalProtocols > 0
        ? this.stats.completedProtocols / this.stats.totalProtocols
        : 0,
      healthy: this.stats.avgConfidence >= PHI_INV_2, // Healthy: avg confidence > φ⁻²
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getResearchRunner(options) {
  if (!_singleton) {
    _singleton = new ResearchRunner(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  if (_singleton) {
    _singleton.removeAllListeners();
  }
  _singleton = null;
}
