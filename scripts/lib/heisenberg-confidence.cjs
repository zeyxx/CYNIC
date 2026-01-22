/**
 * CYNIC Heisenberg Confidence Module (Phase 7A)
 *
 * "On ne peut pas tout savoir avec certitude" - κυνικός
 *
 * Applies the uncertainty principle to CYNIC's assertions:
 * - confidence × specificity ≤ φ (1.618)
 * - High confidence requires low specificity
 * - High specificity requires low confidence
 *
 * This prevents CYNIC from being both very confident AND very specific,
 * which would be intellectually dishonest.
 *
 * @module cynic/lib/heisenberg-confidence
 */

'use strict';

const fs = require('fs');
const path = require('path');
const os = require('os');

// Import φ constants
const phiMath = require('./phi-math.cjs');
const { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } = phiMath;

// =============================================================================
// CONSTANTS (φ-derived)
// =============================================================================

/** Heisenberg limit: confidence × specificity ≤ φ */
const HEISENBERG_LIMIT = PHI;

/** Maximum confidence when specificity is 1.0 */
const MAX_CONFIDENCE_HIGH_SPECIFICITY = PHI;

/** Maximum specificity when confidence is 1.0 */
const MAX_SPECIFICITY_HIGH_CONFIDENCE = PHI;

/** Balanced point: both at √φ ≈ 1.272, product = φ */
const BALANCED_POINT = Math.sqrt(PHI);

/** Minimum uncertainty (always present) - φ⁻³ */
const MINIMUM_UNCERTAINTY = PHI_INV_3;

// =============================================================================
// STORAGE
// =============================================================================

const HEISENBERG_DIR = path.join(os.homedir(), '.cynic', 'heisenberg');
const STATE_FILE = path.join(HEISENBERG_DIR, 'state.json');

// =============================================================================
// STATE
// =============================================================================

const heisenbergState = {
  assertions: [],      // Recent assertions with confidence/specificity
  calibration: {
    overconfidentCount: 0,   // Times confidence was too high
    overspecificCount: 0,    // Times specificity was too high
    balancedCount: 0,        // Times within bounds
  },
  stats: {
    totalAssertions: 0,
    avgConfidence: 0,
    avgSpecificity: 0,
    avgUncertainty: 0,
  },
};

// =============================================================================
// FILE OPERATIONS
// =============================================================================

function ensureDir() {
  if (!fs.existsSync(HEISENBERG_DIR)) {
    fs.mkdirSync(HEISENBERG_DIR, { recursive: true });
  }
}

function loadState() {
  ensureDir();
  if (!fs.existsSync(STATE_FILE)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
  } catch {
    return null;
  }
}

function saveState() {
  ensureDir();
  fs.writeFileSync(STATE_FILE, JSON.stringify({
    calibration: heisenbergState.calibration,
    stats: heisenbergState.stats,
  }, null, 2));
}

// =============================================================================
// SPECIFICITY MEASUREMENT
// =============================================================================

/**
 * Measure specificity of an assertion
 * High specificity = precise, detailed, narrow scope
 * Low specificity = vague, general, broad scope
 *
 * @param {string} assertion - The assertion text
 * @param {Object} context - Additional context
 * @returns {number} Specificity 0-1
 */
function measureSpecificity(assertion, context = {}) {
  let specificity = 0.5; // Start neutral

  // Text-based indicators
  const text = assertion.toLowerCase();

  // High specificity indicators (+)
  const specificIndicators = [
    { pattern: /exactly|precisely|specifically|always|never|must|definitely/, weight: 0.15 },
    { pattern: /\d+(\.\d+)?%/, weight: 0.1 },           // Percentages
    { pattern: /\d+\s*(lines?|files?|bytes?)/, weight: 0.1 }, // Counts
    { pattern: /line\s*\d+/, weight: 0.12 },            // Line numbers
    { pattern: /function\s+\w+|class\s+\w+/, weight: 0.1 }, // Code symbols
    { pattern: /\/.+\.(js|ts|py|go|rs)/, weight: 0.1 }, // File paths
    { pattern: /"[^"]+"/, weight: 0.08 },               // Quoted strings
    { pattern: /\bonly\b|\bjust\b/, weight: 0.08 },     // Exclusive terms
  ];

  // Low specificity indicators (-)
  const vagueIndicators = [
    { pattern: /maybe|perhaps|probably|might|could|possibly/, weight: -0.12 },
    { pattern: /some|few|several|many|various/, weight: -0.08 },
    { pattern: /generally|usually|typically|often/, weight: -0.08 },
    { pattern: /kind of|sort of|somewhat/, weight: -0.1 },
    { pattern: /around|about|approximately/, weight: -0.08 },
    { pattern: /or so|more or less/, weight: -0.1 },
    { pattern: /\?$/, weight: -0.15 },                  // Questions
  ];

  // Apply indicators
  for (const { pattern, weight } of specificIndicators) {
    if (pattern.test(text)) {
      specificity += weight;
    }
  }

  for (const { pattern, weight } of vagueIndicators) {
    if (pattern.test(text)) {
      specificity += weight; // weight is negative
    }
  }

  // Context-based adjustments
  if (context.hasCodeReference) specificity += 0.1;
  if (context.hasLineNumber) specificity += 0.1;
  if (context.hasFilePath) specificity += 0.08;
  if (context.isQuestion) specificity -= 0.15;

  // Clamp to 0-1
  return Math.max(0, Math.min(1, specificity));
}

/**
 * Measure confidence from assertion language
 * @param {string} assertion - The assertion text
 * @returns {number} Implied confidence 0-1
 */
function measureImpliedConfidence(assertion) {
  let confidence = 0.5; // Start neutral

  const text = assertion.toLowerCase();

  // High confidence indicators
  const confidentIndicators = [
    { pattern: /definitely|certainly|absolutely|clearly/, weight: 0.2 },
    { pattern: /i('m| am) sure|i know|it is/, weight: 0.15 },
    { pattern: /must be|has to be|will be/, weight: 0.12 },
    { pattern: /obvious|evident|plain/, weight: 0.1 },
    { pattern: /!$/, weight: 0.08 },  // Exclamation
  ];

  // Low confidence indicators
  const uncertainIndicators = [
    { pattern: /i think|i believe|i guess/, weight: -0.12 },
    { pattern: /seems|appears|looks like/, weight: -0.1 },
    { pattern: /not sure|uncertain|unclear/, weight: -0.15 },
    { pattern: /might|may|could/, weight: -0.1 },
    { pattern: /\?$/, weight: -0.12 }, // Questions
  ];

  for (const { pattern, weight } of confidentIndicators) {
    if (pattern.test(text)) {
      confidence += weight;
    }
  }

  for (const { pattern, weight } of uncertainIndicators) {
    if (pattern.test(text)) {
      confidence += weight;
    }
  }

  return Math.max(0, Math.min(1, confidence));
}

// =============================================================================
// HEISENBERG ENFORCEMENT
// =============================================================================

/**
 * Apply Heisenberg limit to confidence given specificity
 * @param {number} confidence - Desired confidence
 * @param {number} specificity - Measured specificity
 * @returns {number} Adjusted confidence
 */
function applyHeisenbergLimit(confidence, specificity) {
  // Heisenberg limit: confidence × specificity ≤ φ
  // Therefore: max_confidence = φ / specificity

  // Ensure minimum uncertainty
  const effectiveSpecificity = Math.max(specificity, MINIMUM_UNCERTAINTY);

  // Calculate maximum allowed confidence
  const maxConfidence = Math.min(PHI_INV, HEISENBERG_LIMIT / effectiveSpecificity);

  // Return the lesser of desired and max
  return Math.min(confidence, maxConfidence);
}

/**
 * Calculate the uncertainty band around a value
 * @param {number} confidence - Confidence level
 * @param {number} specificity - Specificity level
 * @returns {Object} Uncertainty band
 */
function calculateUncertaintyBand(confidence, specificity) {
  // Product determines how "tight" the band is
  const product = confidence * specificity;

  // Uncertainty is inversely related to product
  // At product = φ, uncertainty is minimal (but still present)
  // At product < φ, uncertainty is larger
  const uncertainty = Math.max(
    MINIMUM_UNCERTAINTY,
    1 - (product / HEISENBERG_LIMIT)
  );

  return {
    uncertainty,
    lowerBound: Math.max(0, 1 - uncertainty),
    upperBound: 1,
    bandWidth: uncertainty,
    interpretation: interpretUncertainty(uncertainty),
  };
}

/**
 * Interpret uncertainty level
 * @param {number} uncertainty - Uncertainty value
 * @returns {string} Interpretation
 */
function interpretUncertainty(uncertainty) {
  if (uncertainty < PHI_INV_3) {
    return 'PRECISE';      // < 23.6% - very confident
  } else if (uncertainty < PHI_INV_2) {
    return 'CONFIDENT';    // 23.6-38.2% - reasonably confident
  } else if (uncertainty < PHI_INV) {
    return 'UNCERTAIN';    // 38.2-61.8% - notable uncertainty
  } else {
    return 'SPECULATIVE';  // > 61.8% - highly uncertain
  }
}

// =============================================================================
// ASSERTION PROCESSING
// =============================================================================

/**
 * Process an assertion and apply Heisenberg limits
 * @param {string} assertion - The assertion text
 * @param {number} claimedConfidence - Claimed confidence (optional)
 * @param {Object} context - Additional context
 * @returns {Object} Processed assertion with adjusted confidence
 */
function processAssertion(assertion, claimedConfidence = null, context = {}) {
  // Measure specificity
  const specificity = measureSpecificity(assertion, context);

  // Get confidence (claimed or implied)
  const rawConfidence = claimedConfidence !== null
    ? claimedConfidence
    : measureImpliedConfidence(assertion);

  // Apply Heisenberg limit
  const adjustedConfidence = applyHeisenbergLimit(rawConfidence, specificity);

  // Was confidence reduced?
  const wasReduced = adjustedConfidence < rawConfidence;

  // Calculate uncertainty band
  const uncertaintyBand = calculateUncertaintyBand(adjustedConfidence, specificity);

  // Track for calibration
  heisenbergState.assertions.push({
    assertion: assertion.slice(0, 100),
    specificity,
    rawConfidence,
    adjustedConfidence,
    wasReduced,
    timestamp: Date.now(),
  });

  // Keep only last 50 assertions
  if (heisenbergState.assertions.length > 50) {
    heisenbergState.assertions.shift();
  }

  // Update calibration stats
  if (wasReduced) {
    heisenbergState.calibration.overconfidentCount++;
  } else {
    heisenbergState.calibration.balancedCount++;
  }

  // Update running stats
  const n = heisenbergState.stats.totalAssertions + 1;
  heisenbergState.stats.avgConfidence =
    (heisenbergState.stats.avgConfidence * (n - 1) + adjustedConfidence) / n;
  heisenbergState.stats.avgSpecificity =
    (heisenbergState.stats.avgSpecificity * (n - 1) + specificity) / n;
  heisenbergState.stats.avgUncertainty =
    (heisenbergState.stats.avgUncertainty * (n - 1) + uncertaintyBand.uncertainty) / n;
  heisenbergState.stats.totalAssertions = n;

  saveState();

  return {
    specificity,
    rawConfidence,
    adjustedConfidence,
    wasReduced,
    reduction: wasReduced ? rawConfidence - adjustedConfidence : 0,
    uncertaintyBand,
    product: adjustedConfidence * specificity,
    heisenbergLimit: HEISENBERG_LIMIT,
    warning: wasReduced
      ? `Confiance réduite de ${Math.round(rawConfidence * 100)}% à ${Math.round(adjustedConfidence * 100)}% (spécificité: ${Math.round(specificity * 100)}%)`
      : null,
  };
}

/**
 * Suggest how to phrase an assertion more appropriately
 * @param {Object} result - Result from processAssertion
 * @returns {string} Suggestion
 */
function suggestRephrasing(result) {
  if (!result.wasReduced) {
    return null;
  }

  if (result.specificity > 0.7) {
    // High specificity - suggest reducing certainty
    return 'Assertion très spécifique. Considérez: "Il semble que..." ou "Probablement..."';
  } else if (result.rawConfidence > 0.8) {
    // High confidence - suggest being less specific
    return 'Haute confiance. Considérez des termes plus généraux pour rester honnête.';
  } else {
    return 'Ajustez soit la précision soit la certitude de cette affirmation.';
  }
}

// =============================================================================
// VISUAL DISPLAY
// =============================================================================

/**
 * Format uncertainty for display
 * @param {Object} result - Result from processAssertion
 * @returns {string} Formatted display
 */
function formatUncertainty(result) {
  const conf = Math.round(result.adjustedConfidence * 100);
  const spec = Math.round(result.specificity * 100);
  const uncert = Math.round(result.uncertaintyBand.uncertainty * 100);

  const confBar = '█'.repeat(Math.round(conf / 10)) + '░'.repeat(10 - Math.round(conf / 10));
  const specBar = '█'.repeat(Math.round(spec / 10)) + '░'.repeat(10 - Math.round(spec / 10));

  const lines = [
    '── HEISENBERG ─────────────────────────────────────────────',
    `   Confiance:  [${confBar}] ${conf}%`,
    `   Spécificité:[${specBar}] ${spec}%`,
    `   Incertitude: ${uncert}% (${result.uncertaintyBand.interpretation})`,
  ];

  if (result.wasReduced) {
    lines.push(`   ⚠️ ${result.warning}`);
  }

  return lines.join('\n');
}

/**
 * Get emoji for uncertainty level
 * @param {string} interpretation - Uncertainty interpretation
 * @returns {string} Emoji
 */
function getUncertaintyEmoji(interpretation) {
  const emojis = {
    PRECISE: '🎯',
    CONFIDENT: '✓',
    UNCERTAIN: '❓',
    SPECULATIVE: '🔮',
  };
  return emojis[interpretation] || '📊';
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Initialize the Heisenberg module
 */
function init() {
  ensureDir();
  const saved = loadState();
  if (saved) {
    heisenbergState.calibration = saved.calibration || heisenbergState.calibration;
    heisenbergState.stats = saved.stats || heisenbergState.stats;
  }
}

/**
 * Get calibration statistics
 * @returns {Object} Stats
 */
function getStats() {
  return {
    ...heisenbergState.stats,
    calibration: heisenbergState.calibration,
    recentAssertions: heisenbergState.assertions.length,
    heisenbergLimit: HEISENBERG_LIMIT,
  };
}

/**
 * Quick check if an assertion needs Heisenberg warning
 * @param {string} assertion - Assertion to check
 * @param {number} confidence - Claimed confidence
 * @returns {boolean} Whether warning is needed
 */
function needsWarning(assertion, confidence) {
  const specificity = measureSpecificity(assertion);
  const maxConfidence = applyHeisenbergLimit(1.0, specificity);
  return confidence > maxConfidence;
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Constants
  HEISENBERG_LIMIT,
  MINIMUM_UNCERTAINTY,
  BALANCED_POINT,

  // Core functions
  init,
  getStats,

  // Measurement
  measureSpecificity,
  measureImpliedConfidence,

  // Heisenberg enforcement
  applyHeisenbergLimit,
  calculateUncertaintyBand,
  processAssertion,
  needsWarning,

  // Display
  formatUncertainty,
  getUncertaintyEmoji,
  suggestRephrasing,

  // Interpretation
  interpretUncertainty,
};
