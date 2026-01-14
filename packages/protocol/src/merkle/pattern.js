/**
 * Pattern Management
 *
 * Patterns are validated knowledge that emerge from multiple sources
 *
 * @module @cynic/protocol/merkle/pattern
 */

'use strict';

import { MIN_PATTERN_SOURCES, AXIOMS, PHI_INV } from '@cynic/core';
import { sha256Prefixed, hashObject, randomHex } from '../crypto/hash.js';

/**
 * Generate pattern ID
 * @returns {string} Unique pattern ID
 */
export function generatePatternId() {
  return `pat_${randomHex(16)}`;
}

/**
 * Generate learning ID
 * @returns {string} Unique learning ID
 */
export function generateLearningId() {
  return `lrn_${randomHex(16)}`;
}

/**
 * Create a pattern
 * @param {Object} params - Pattern parameters
 * @param {string} params.content - Pattern content (will be hashed)
 * @param {string} params.axiom - Associated axiom
 * @param {number} params.strength - Pattern strength (0-1)
 * @param {number} params.sources - Number of independent sources
 * @param {Object} [params.metadata] - Additional metadata
 * @returns {Object} Pattern object
 */
export function createPattern({ content, axiom, strength, sources, metadata = {} }) {
  if (!AXIOMS[axiom]) {
    throw new Error(`Invalid axiom: ${axiom}`);
  }

  return {
    id: generatePatternId(),
    content_hash: sha256Prefixed(typeof content === 'string' ? content : hashObject(content)),
    axiom,
    strength: Math.min(strength, PHI_INV), // φ-bounded
    sources,
    created_at: Date.now(),
    ...metadata,
  };
}

/**
 * Create a learning
 * @param {Object} params - Learning parameters
 * @param {string} params.type - Learning type (insight, error, decision)
 * @param {string} params.content - Learning content (will be hashed)
 * @param {string} params.axiom - Associated axiom
 * @param {number} params.confidence - Confidence level (0-1)
 * @param {string} [params.contributor] - Contributor ID (will be hashed)
 * @returns {Object} Learning object
 */
export function createLearning({ type, content, axiom, confidence, contributor = null }) {
  if (!AXIOMS[axiom]) {
    throw new Error(`Invalid axiom: ${axiom}`);
  }

  const learning = {
    id: generateLearningId(),
    type,
    content_hash: sha256Prefixed(typeof content === 'string' ? content : hashObject(content)),
    axiom,
    confidence: Math.min(confidence, PHI_INV), // φ-bounded
    created_at: Date.now(),
  };

  if (contributor) {
    // Privacy: hash contributor ID
    learning.contributor = sha256Prefixed(contributor);
  }

  return learning;
}

/**
 * Validate pattern meets emergence criteria
 * @param {Object} pattern - Pattern to validate
 * @returns {{ valid: boolean, errors: string[] }} Validation result
 */
export function validatePatternEmergence(pattern) {
  const errors = [];

  // Required fields
  const required = ['id', 'content_hash', 'axiom', 'strength', 'sources'];
  for (const field of required) {
    if (pattern[field] === undefined) {
      errors.push(`Missing required field: ${field}`);
    }
  }

  // ID format
  if (pattern.id && !pattern.id.startsWith('pat_')) {
    errors.push('Invalid pattern ID format');
  }

  // Content hash format
  if (pattern.content_hash && !pattern.content_hash.startsWith('sha256:')) {
    errors.push('Invalid content_hash format');
  }

  // Valid axiom
  if (pattern.axiom && !AXIOMS[pattern.axiom]) {
    errors.push(`Invalid axiom: ${pattern.axiom}`);
  }

  // Strength bounds
  if (typeof pattern.strength === 'number') {
    if (pattern.strength < 0 || pattern.strength > 1) {
      errors.push('strength must be between 0 and 1');
    }
  }

  // Minimum sources for pattern validation
  if (typeof pattern.sources === 'number') {
    if (pattern.sources < MIN_PATTERN_SOURCES) {
      errors.push(`Pattern requires at least ${MIN_PATTERN_SOURCES} independent sources`);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Check if observations form a pattern
 * @param {Object[]} observations - Array of similar observations
 * @returns {{ isPattern: boolean, strength: number, sources: number }} Pattern check result
 */
export function checkPatternFormation(observations) {
  // Count unique sources
  const uniqueSources = new Set(observations.map((o) => o.source || o.operator || 'unknown'));
  const sources = uniqueSources.size;

  // Pattern requires MIN_PATTERN_SOURCES independent sources
  const isPattern = sources >= MIN_PATTERN_SOURCES;

  // Calculate strength based on agreement
  const strength = isPattern
    ? Math.min(sources / (MIN_PATTERN_SOURCES * 2), PHI_INV) // Scale up to φ⁻¹
    : 0;

  return {
    isPattern,
    strength,
    sources,
  };
}

/**
 * Merge similar patterns
 * @param {Object[]} patterns - Patterns to merge
 * @returns {Object} Merged pattern
 */
export function mergePatterns(patterns) {
  if (patterns.length === 0) {
    throw new Error('Cannot merge empty patterns');
  }

  if (patterns.length === 1) {
    return patterns[0];
  }

  // Use first pattern as base
  const base = patterns[0];

  // Aggregate sources and strength
  const totalSources = patterns.reduce((sum, p) => sum + (p.sources || 1), 0);
  const avgStrength =
    patterns.reduce((sum, p) => sum + (p.strength || 0), 0) / patterns.length;

  return createPattern({
    content: { merged_from: patterns.map((p) => p.id) },
    axiom: base.axiom,
    strength: Math.min(avgStrength * 1.1, PHI_INV), // Slight boost for corroboration
    sources: totalSources,
    metadata: {
      merged_at: Date.now(),
      source_patterns: patterns.length,
    },
  });
}

/**
 * Calculate pattern relevance decay over time
 * @param {Object} pattern - Pattern to evaluate
 * @param {number} [halfLifeMs=604800000] - Half-life in ms (default 1 week)
 * @returns {number} Decayed strength
 */
export function calculatePatternDecay(pattern, halfLifeMs = 604800000) {
  const age = Date.now() - (pattern.created_at || 0);
  const decayFactor = Math.pow(0.5, age / halfLifeMs);
  return (pattern.strength || 0) * decayFactor;
}

export default {
  generatePatternId,
  generateLearningId,
  createPattern,
  createLearning,
  validatePatternEmergence,
  checkPatternFormation,
  mergePatterns,
  calculatePatternDecay,
};
