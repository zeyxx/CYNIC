/**
 * ENTROPY CALCULATOR - Information Theory for Context Scoring
 *
 * "High entropy = noise. Low entropy = signal." - κυνικός
 *
 * Entropy types calculated:
 * - Shannon Entropy (H): Information density via character/word distribution
 * - Lexical Entropy (L): Vocabulary richness (unique/total tokens)
 * - Structural Entropy (S): Pattern regularity (1 - compression_ratio)
 *
 * Combined into an Entropy Factor (E) for C-Score:
 * - Low entropy (focused) → E ≈ 1.0 (retain)
 * - High entropy (diffuse) → E ≈ 0.5 (compress candidate)
 *
 * @module @cynic/core/context/entropy
 * @philosophy Entropy-guided attention reduces collapse by 80%
 */

'use strict';

import {
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
} from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

/**
 * φ-aligned entropy thresholds
 * Research shows healthy entropy around 60% matches φ⁻¹
 */
export const ENTROPY_THRESHOLDS = {
  OPTIMAL: PHI_INV,           // 61.8% - healthy entropy
  LOW: PHI_INV_2,             // 38.2% - too focused (might miss nuance)
  HIGH: PHI_INV + PHI_INV_2,  // ~100% - too diffuse (noise)
};

/**
 * Weights for combining entropy types (φ-weighted)
 */
export const ENTROPY_WEIGHTS = {
  SHANNON: PHI_INV,      // 61.8% - primary signal
  LEXICAL: PHI_INV_2,    // 38.2% - secondary
  STRUCTURAL: PHI_INV_3, // 23.6% - tertiary
};

// =============================================================================
// SHANNON ENTROPY
// =============================================================================

/**
 * Calculate Shannon entropy of text
 *
 * H = -Σ p(x) × log₂(p(x))
 *
 * Measures information density via character distribution.
 * Normalized to 0-1 range: H_norm = H / log₂(n)
 *
 * @param {string} text - Text to analyze
 * @returns {number} Normalized Shannon entropy (0-1)
 */
export function calculateShannonEntropy(text) {
  if (!text || typeof text !== 'string' || text.length === 0) {
    return 0;
  }

  // Count character frequencies
  const freq = new Map();
  for (const char of text) {
    freq.set(char, (freq.get(char) || 0) + 1);
  }

  const n = text.length;
  const uniqueChars = freq.size;

  // Calculate entropy: H = -Σ p(x) × log₂(p(x))
  let entropy = 0;
  for (const count of freq.values()) {
    const p = count / n;
    if (p > 0) {
      entropy -= p * Math.log2(p);
    }
  }

  // Normalize by maximum possible entropy (uniform distribution)
  // Max entropy = log₂(uniqueChars)
  const maxEntropy = Math.log2(uniqueChars);
  if (maxEntropy === 0) {
    return 0;
  }

  return Math.round((entropy / maxEntropy) * 1000) / 1000;
}

/**
 * Calculate word-level Shannon entropy
 *
 * More meaningful for natural language than character-level
 *
 * @param {string} text - Text to analyze
 * @returns {number} Normalized word entropy (0-1)
 */
export function calculateWordEntropy(text) {
  if (!text || typeof text !== 'string') {
    return 0;
  }

  // Tokenize into words (lowercase, filter short)
  const words = text.toLowerCase()
    .split(/\W+/)
    .filter(w => w.length > 1);

  if (words.length === 0) {
    return 0;
  }

  // Count word frequencies
  const freq = new Map();
  for (const word of words) {
    freq.set(word, (freq.get(word) || 0) + 1);
  }

  const n = words.length;
  const uniqueWords = freq.size;

  // Calculate entropy
  let entropy = 0;
  for (const count of freq.values()) {
    const p = count / n;
    if (p > 0) {
      entropy -= p * Math.log2(p);
    }
  }

  // Normalize
  const maxEntropy = Math.log2(uniqueWords);
  if (maxEntropy === 0) {
    return 0;
  }

  return Math.round((entropy / maxEntropy) * 1000) / 1000;
}

// =============================================================================
// LEXICAL ENTROPY
// =============================================================================

/**
 * Calculate lexical entropy (vocabulary richness)
 *
 * L = unique_tokens / total_tokens
 *
 * High L = diverse vocabulary = information-dense
 * Low L = repetitive vocabulary = filler
 *
 * Uses Type-Token Ratio (TTR) adjusted for text length
 * (longer texts naturally have lower TTR)
 *
 * @param {string} text - Text to analyze
 * @returns {number} Lexical entropy (0-1)
 */
export function calculateLexicalEntropy(text) {
  if (!text || typeof text !== 'string') {
    return 0;
  }

  // Tokenize
  const tokens = text.toLowerCase()
    .split(/\W+/)
    .filter(w => w.length > 1);

  if (tokens.length === 0) {
    return 0;
  }

  const uniqueTokens = new Set(tokens).size;
  const totalTokens = tokens.length;

  // Basic TTR
  const ttr = uniqueTokens / totalTokens;

  // Adjust for text length using root TTR (RTTR)
  // RTTR = unique / √total (more stable for varying lengths)
  const rttr = uniqueTokens / Math.sqrt(totalTokens);

  // Normalize RTTR to 0-1 range
  // Typical RTTR values range from 2 to 15
  // We scale so that RTTR of 10 ≈ 0.8 (healthy diversity)
  const normalizedRttr = Math.min(1, rttr / 12);

  // Combine basic TTR and adjusted RTTR
  // φ-weighted: 61.8% RTTR + 38.2% TTR
  const combined = normalizedRttr * PHI_INV + ttr * PHI_INV_2;

  return Math.round(combined * 1000) / 1000;
}

// =============================================================================
// STRUCTURAL ENTROPY
// =============================================================================

/**
 * Calculate structural entropy (pattern regularity)
 *
 * S = 1 - compression_ratio (estimated)
 *
 * Low compression (high S) = high randomness = keep
 * High compression (low S) = repetitive = summarize candidate
 *
 * Uses a simple heuristic based on:
 * - Line repetition
 * - N-gram repetition
 * - Whitespace patterns
 *
 * @param {string} text - Text to analyze
 * @returns {number} Structural entropy (0-1)
 */
export function calculateStructuralEntropy(text) {
  if (!text || typeof text !== 'string' || text.length < 10) {
    return 0.5; // Default neutral for very short text
  }

  let structureScore = 1.0;

  // 1. Line repetition analysis
  const lines = text.split('\n').map(l => l.trim()).filter(l => l.length > 0);
  if (lines.length > 1) {
    const uniqueLines = new Set(lines).size;
    const lineRepetition = 1 - (uniqueLines / lines.length);
    // High repetition reduces structural entropy
    structureScore -= lineRepetition * 0.3;
  }

  // 2. Trigram repetition (3-word sequences)
  const words = text.toLowerCase().split(/\W+/).filter(w => w.length > 1);
  if (words.length >= 3) {
    const trigrams = [];
    for (let i = 0; i < words.length - 2; i++) {
      trigrams.push(`${words[i]} ${words[i + 1]} ${words[i + 2]}`);
    }
    if (trigrams.length > 0) {
      const uniqueTrigrams = new Set(trigrams).size;
      const trigramRepetition = 1 - (uniqueTrigrams / trigrams.length);
      structureScore -= trigramRepetition * 0.3;
    }
  }

  // 3. Character-level pattern detection (simple RLE heuristic)
  // Count runs of repeated characters
  let runs = 0;
  let currentRun = 1;
  for (let i = 1; i < text.length; i++) {
    if (text[i] === text[i - 1]) {
      currentRun++;
    } else {
      if (currentRun > 3) runs++;
      currentRun = 1;
    }
  }
  const runRatio = runs / (text.length / 10);
  structureScore -= Math.min(0.2, runRatio * 0.1);

  // 4. Structural markers (code-like patterns boost structural entropy)
  const structuralMarkers = [
    /\{[\s\S]*?\}/g,     // Braces
    /\[[\s\S]*?\]/g,     // Brackets
    /\([\s\S]*?\)/g,     // Parentheses
    /^\s*[-*]\s+/gm,     // List items
    /\w+:\s*\w+/g,       // Key-value pairs
  ];

  let markerCount = 0;
  for (const pattern of structuralMarkers) {
    const matches = text.match(pattern);
    if (matches) markerCount += matches.length;
  }
  // Structured content has higher information density
  const markerBonus = Math.min(0.2, markerCount * 0.01);
  structureScore += markerBonus;

  return Math.min(1, Math.max(0, Math.round(structureScore * 1000) / 1000));
}

// =============================================================================
// COMBINED ENTROPY FACTOR
// =============================================================================

/**
 * Convert raw entropy to retention factor
 *
 * The relationship between entropy and retention is non-linear:
 * - Very low entropy (too focused) → slight penalty (might miss context)
 * - Optimal entropy (φ⁻¹ range) → high retention
 * - High entropy (too diffuse) → strong penalty (noise)
 *
 * @param {number} entropy - Combined entropy (0-1)
 * @returns {number} Retention factor (0-1)
 */
export function entropyToRetentionFactor(entropy) {
  // Optimal entropy is around φ⁻¹ (61.8%)
  const optimal = ENTROPY_THRESHOLDS.OPTIMAL;

  // Distance from optimal
  const distance = Math.abs(entropy - optimal);

  // Non-linear penalty: closer to optimal = higher retention
  // Using a smooth curve that peaks at optimal
  // E = 1 - (distance / optimal)^1.5
  const penalty = Math.pow(distance / optimal, 1.5);
  let factor = 1 - penalty * 0.5; // Max penalty is 50%

  // Extra penalty for very high entropy (noise)
  if (entropy > ENTROPY_THRESHOLDS.HIGH * 0.9) {
    factor *= 0.7; // Additional 30% penalty
  }

  // Slight boost for very low entropy (focused content like code)
  if (entropy < ENTROPY_THRESHOLDS.LOW) {
    factor = Math.min(1, factor * 1.1); // Up to 10% boost
  }

  return Math.round(Math.max(0.3, Math.min(1, factor)) * 1000) / 1000;
}

/**
 * Calculate combined entropy factor for C-Score
 *
 * Combines Shannon, Lexical, and Structural entropy
 * using φ-weighted combination:
 *
 * Combined = Shannon × φ⁻¹ + Lexical × φ⁻² + Structural × φ⁻³
 * (Weights sum to ~1.236, we normalize)
 *
 * Then converts to retention factor where:
 * - Healthy entropy → 1.0 (retain)
 * - High entropy → lower (compress candidate)
 *
 * @param {Object|string} content - Content object or text string
 * @returns {Object} Entropy analysis result
 */
export function calculateEntropyFactor(content) {
  // Extract text
  const text = typeof content === 'string'
    ? content
    : (content?.text || content?.content || '');

  if (!text || text.length === 0) {
    return {
      entropy: 0,
      factor: 0.5,
      breakdown: {
        shannon: 0,
        wordEntropy: 0,
        lexical: 0,
        structural: 0.5,
      },
    };
  }

  // Calculate component entropies
  const shannon = calculateShannonEntropy(text);
  const wordEntropy = calculateWordEntropy(text);
  const lexical = calculateLexicalEntropy(text);
  const structural = calculateStructuralEntropy(text);

  // Use word entropy for natural text, character entropy for code
  // Detect code-like content
  const codeIndicators = /function|const|let|var|import|export|class|\{|\}|=>|;$/gm;
  const isCodeLike = (text.match(codeIndicators) || []).length > 3;

  // φ-weighted combination
  // For code: emphasize character-level Shannon
  // For prose: emphasize word-level entropy
  const weightedShannon = isCodeLike
    ? shannon * ENTROPY_WEIGHTS.SHANNON
    : wordEntropy * ENTROPY_WEIGHTS.SHANNON;

  const combined = (
    weightedShannon +
    lexical * ENTROPY_WEIGHTS.LEXICAL +
    structural * ENTROPY_WEIGHTS.STRUCTURAL
  );

  // Normalize (weights sum to ~1.236)
  const weightSum = ENTROPY_WEIGHTS.SHANNON + ENTROPY_WEIGHTS.LEXICAL + ENTROPY_WEIGHTS.STRUCTURAL;
  const normalizedEntropy = combined / weightSum;

  // Convert to retention factor
  const factor = entropyToRetentionFactor(normalizedEntropy);

  return {
    entropy: Math.round(normalizedEntropy * 1000) / 1000,
    factor,
    breakdown: {
      shannon: Math.round(shannon * 100),
      wordEntropy: Math.round(wordEntropy * 100),
      lexical: Math.round(lexical * 100),
      structural: Math.round(structural * 100),
    },
    isCodeLike,
  };
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Constants
  ENTROPY_THRESHOLDS,
  ENTROPY_WEIGHTS,

  // Component entropies
  calculateShannonEntropy,
  calculateWordEntropy,
  calculateLexicalEntropy,
  calculateStructuralEntropy,

  // Combined
  entropyToRetentionFactor,
  calculateEntropyFactor,
};
