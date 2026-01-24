/**
 * Residual Detector
 *
 * Discovers new dimensions from unexplained variance
 *
 * "THE UNNAMEABLE = what exists before being named"
 *
 * When residual > 38.2% (φ⁻²), something is not being captured
 * by existing dimensions - a new dimension may be emerging.
 *
 * @module @cynic/node/judge/residual
 */

'use strict';

import { PHI_INV_2, PHI_INV, AXIOMS, MIN_PATTERN_SOURCES } from '@cynic/core';
import { calculateResidual } from '@cynic/protocol';
import { dimensionRegistry } from './dimensions.js';

/**
 * Residual Detector - Discovers new dimensions
 */
export class ResidualDetector {
  /**
   * @param {Object} [options] - Detector options
   * @param {number} [options.threshold] - Anomaly threshold (default: φ⁻²)
   * @param {number} [options.minSamples] - Min samples for pattern (default: 3)
   * @param {number} [options.maxAnomalies] - Max anomalies to store (default: 1000)
   * @param {number} [options.maxCandidates] - Max candidates to store (default: 100)
   * @param {number} [options.maxDiscoveries] - Max discoveries to store (default: 100)
   */
  constructor(options = {}) {
    this.threshold = options.threshold || PHI_INV_2;
    this.minSamples = options.minSamples || MIN_PATTERN_SOURCES;
    this.maxAnomalies = options.maxAnomalies || 1000;
    this.maxCandidates = options.maxCandidates || 100;
    this.maxDiscoveries = options.maxDiscoveries || 100;

    // Anomaly storage (bounded)
    this.anomalies = [];

    // Candidate dimensions (bounded)
    this.candidates = new Map();

    // Discovery history (bounded)
    this.discoveries = [];
  }

  /**
   * Analyze judgment for residual anomalies
   * @param {Object} judgment - Judgment to analyze
   * @param {Object} [metadata] - Additional metadata
   * @returns {Object} Analysis result
   */
  analyze(judgment, metadata = {}) {
    const residual = calculateResidual(judgment);

    const result = {
      judgmentId: judgment.id,
      residual,
      isAnomaly: residual > this.threshold,
      timestamp: Date.now(),
    };

    if (result.isAnomaly) {
      this._recordAnomaly(judgment, residual, metadata);
    }

    return result;
  }

  /**
   * Record anomaly
   * @private
   */
  _recordAnomaly(judgment, residual, metadata) {
    const anomaly = {
      judgmentId: judgment.id,
      residual,
      dimensions: judgment.dimensions,
      globalScore: judgment.global_score,
      metadata,
      timestamp: Date.now(),
    };

    this.anomalies.push(anomaly);

    // Keep bounded
    if (this.anomalies.length > this.maxAnomalies) {
      this.anomalies.shift();
    }

    // Try to find patterns
    this._detectPatterns();
  }

  /**
   * Detect patterns in anomalies
   * @private
   */
  _detectPatterns() {
    if (this.anomalies.length < this.minSamples) {
      return;
    }

    // Cluster anomalies by dimension score patterns
    const clusters = this._clusterAnomalies();

    for (const cluster of clusters) {
      if (cluster.samples.length >= this.minSamples) {
        this._evaluateCandidate(cluster);
      }
    }
  }

  /**
   * Cluster anomalies by similar characteristics
   * @private
   */
  _clusterAnomalies() {
    // Simple clustering: group by which dimensions are weak
    const clusters = new Map();

    // φ⁻² threshold for weak dimensions (38.2%)
    const weakThreshold = Math.round(PHI_INV_2 * 100);

    for (const anomaly of this.anomalies) {
      const weakDims = Object.entries(anomaly.dimensions || {})
        .filter(([, score]) => score < weakThreshold)
        .map(([name]) => name)
        .sort()
        .join(',');

      const key = weakDims || 'general';
      if (!clusters.has(key)) {
        clusters.set(key, {
          key,
          weakDimensions: weakDims.split(',').filter(Boolean),
          samples: [],
        });
      }
      clusters.get(key).samples.push(anomaly);
    }

    return Array.from(clusters.values());
  }

  /**
   * Evaluate candidate dimension
   * @private
   */
  _evaluateCandidate(cluster) {
    // Use stable key based on cluster pattern (not timestamp)
    const key = `candidate_${cluster.key}`;

    // Calculate average residual
    const avgResidual =
      cluster.samples.reduce((sum, s) => sum + s.residual, 0) / cluster.samples.length;

    // Only consider if residual is significant
    if (avgResidual < this.threshold) {
      return;
    }

    const existing = this.candidates.get(key);
    const candidate = {
      key,
      weakDimensions: cluster.weakDimensions,
      sampleCount: cluster.samples.length,
      avgResidual,
      suggestedAxiom: this._suggestAxiom(cluster),
      suggestedName: existing?.suggestedName || this._suggestName(cluster),
      confidence: Math.min(cluster.samples.length / 10, PHI_INV),
      detectedAt: existing?.detectedAt || Date.now(),
      updatedAt: Date.now(),
    };

    this.candidates.set(key, candidate);

    // Evict oldest candidates if over limit
    if (this.candidates.size > this.maxCandidates) {
      this._evictOldestCandidate();
    }
  }

  /**
   * Evict oldest candidate by detectedAt timestamp
   * @private
   */
  _evictOldestCandidate() {
    let oldestKey = null;
    let oldestTime = Infinity;

    for (const [key, candidate] of this.candidates) {
      if (candidate.detectedAt < oldestTime) {
        oldestTime = candidate.detectedAt;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      this.candidates.delete(oldestKey);
    }
  }

  /**
   * Suggest axiom for candidate
   * @private
   */
  _suggestAxiom(cluster) {
    // Analyze which axioms' dimensions are weak
    const axiomWeakness = { PHI: 0, VERIFY: 0, CULTURE: 0, BURN: 0 };

    for (const sample of cluster.samples) {
      for (const [dim, score] of Object.entries(sample.dimensions || {})) {
        // Determine axiom of dimension (simplified)
        if (['COHERENCE', 'HARMONY', 'STRUCTURE', 'ELEGANCE', 'COMPLETENESS', 'PRECISION'].includes(dim)) {
          if (score < 40) axiomWeakness.PHI++;
        } else if (['ACCURACY', 'VERIFIABILITY', 'TRANSPARENCY', 'REPRODUCIBILITY', 'PROVENANCE', 'INTEGRITY'].includes(dim)) {
          if (score < 40) axiomWeakness.VERIFY++;
        } else if (['AUTHENTICITY', 'RELEVANCE', 'NOVELTY', 'ALIGNMENT', 'IMPACT', 'RESONANCE'].includes(dim)) {
          if (score < 40) axiomWeakness.CULTURE++;
        } else {
          if (score < 40) axiomWeakness.BURN++;
        }
      }
    }

    // Return axiom with most weakness (needs new dimension)
    let maxWeakness = 0;
    let suggestedAxiom = 'VERIFY'; // Default

    for (const [axiom, weakness] of Object.entries(axiomWeakness)) {
      if (weakness > maxWeakness) {
        maxWeakness = weakness;
        suggestedAxiom = axiom;
      }
    }

    return suggestedAxiom;
  }

  /**
   * Suggest name for candidate dimension
   * @private
   */
  _suggestName(cluster) {
    // Generate placeholder name - human should provide real name
    const timestamp = Date.now().toString(36).slice(-4);
    return `UNNAMED_${timestamp}`;
  }

  /**
   * Get candidate dimensions
   * @returns {Object[]} Candidates
   */
  getCandidates() {
    return Array.from(this.candidates.values())
      .filter((c) => c.confidence >= PHI_INV_2)
      .sort((a, b) => b.confidence - a.confidence);
  }

  /**
   * Accept candidate dimension (requires governance)
   * @param {string} candidateKey - Candidate key
   * @param {Object} params - Acceptance params
   * @param {string} params.name - Final dimension name
   * @param {string} params.axiom - Axiom to add to
   * @param {number} [params.weight] - Dimension weight
   * @param {number} [params.threshold] - Score threshold
   * @returns {Object} Accepted dimension
   */
  acceptCandidate(candidateKey, { name, axiom, weight = 1.0, threshold = 50 }) {
    const candidate = this.candidates.get(candidateKey);
    if (!candidate) {
      throw new Error(`Candidate not found: ${candidateKey}`);
    }

    if (!AXIOMS[axiom]) {
      throw new Error(`Invalid axiom: ${axiom}`);
    }

    // Register new dimension
    dimensionRegistry.register(name, axiom, {
      weight,
      threshold,
      description: `Discovered dimension (was ${candidate.suggestedName})`,
    });

    // Record discovery
    const discovery = {
      name,
      axiom,
      weight,
      threshold,
      fromCandidate: candidateKey,
      discoveredAt: Date.now(),
    };
    this.discoveries.push(discovery);

    // Keep discoveries bounded
    if (this.discoveries.length > this.maxDiscoveries) {
      this.discoveries.shift();
    }

    // Remove candidate
    this.candidates.delete(candidateKey);

    return discovery;
  }

  /**
   * Reject candidate
   * @param {string} candidateKey - Candidate key
   */
  rejectCandidate(candidateKey) {
    this.candidates.delete(candidateKey);
  }

  /**
   * Get discoveries
   * @returns {Object[]} Discovered dimensions
   */
  getDiscoveries() {
    return [...this.discoveries];
  }

  /**
   * Get statistics
   * @returns {Object} Statistics
   */
  getStats() {
    return {
      anomalyCount: this.anomalies.length,
      candidateCount: this.candidates.size,
      discoveryCount: this.discoveries.length,
      threshold: this.threshold,
      bounds: {
        maxAnomalies: this.maxAnomalies,
        maxCandidates: this.maxCandidates,
        maxDiscoveries: this.maxDiscoveries,
      },
    };
  }

  /**
   * Export state
   * @returns {Object} Exportable state
   */
  export() {
    return {
      anomalies: this.anomalies.slice(-100), // Keep last 100
      candidates: Object.fromEntries(this.candidates),
      discoveries: this.discoveries,
    };
  }

  /**
   * Import state
   * @param {Object} state - Saved state
   */
  import(state) {
    if (state.anomalies) {
      this.anomalies = state.anomalies;
    }
    if (state.candidates) {
      this.candidates = new Map(Object.entries(state.candidates));
    }
    if (state.discoveries) {
      this.discoveries = state.discoveries;
    }
  }
}

export default { ResidualDetector };
