/**
 * PoJ (Proof of Judgment) Anchoring Strategy
 *
 * Hybrid φ-based strategy for deciding what gets anchored
 * to the immutable PoJ Chain vs DAG-only storage.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/scripts/poj-strategy
 */

'use strict';

// φ Constants
const PHI = 1.618033988749895;
const PHI_INV = 1 / PHI;         // 0.618 - confidence threshold
const PHI_INV_2 = 1 / (PHI * PHI); // 0.382 - lower threshold
const PHI_INV_3 = 1 / (PHI * PHI * PHI); // 0.236 - minimal threshold

// ═══════════════════════════════════════════════════════════════════════════
// ANCHORING TIERS (Based on Q-Score and Impact)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Anchoring tiers determine WHERE a judgment is stored:
 *
 * TIER 1 (VOLATILE): DAG only, can be compacted/pruned
 *   - Q-Score < 38.2% (φ⁻²)
 *   - Low confidence, experimental, noise
 *
 * TIER 2 (DURABLE): DAG + PostgreSQL index
 *   - Q-Score 38.2% - 61.8%
 *   - Searchable, archivable, but not chain-anchored
 *
 * TIER 3 (IMMUTABLE): PoJ Chain + DAG + PostgreSQL
 *   - Q-Score ≥ 61.8% (φ⁻¹)
 *   - OR: Critical category (governance, security, tier-change)
 *   - Merkle proofs available, fully verifiable
 */
const ANCHORING_TIERS = {
  VOLATILE: {
    name: 'volatile',
    minQScore: 0,
    maxQScore: PHI_INV_2 * 100, // 38.2
    storage: ['DAG'],
    canCompact: true,
    merkleProof: false,
    retention: '30d', // Pruned after 30 days if not referenced
  },
  DURABLE: {
    name: 'durable',
    minQScore: PHI_INV_2 * 100, // 38.2
    maxQScore: PHI_INV * 100,   // 61.8
    storage: ['DAG', 'PostgreSQL'],
    canCompact: false,
    merkleProof: false,
    retention: '1y', // Archived after 1 year
  },
  IMMUTABLE: {
    name: 'immutable',
    minQScore: PHI_INV * 100, // 61.8
    maxQScore: 100,
    storage: ['PoJ', 'DAG', 'PostgreSQL'],
    canCompact: false,
    merkleProof: true,
    retention: 'forever',
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// CRITICAL CATEGORIES (Always anchored regardless of Q-Score)
// ═══════════════════════════════════════════════════════════════════════════

const CRITICAL_CATEGORIES = new Set([
  'governance',          // Any governance decision
  'security',            // Security alerts, GROWL
  'tier_change',         // E-Score tier changes (A→B, etc.)
  'k_score_significant', // K-Score changes > 10 points
  'token_status',        // Token acceptance/rejection changes
  'contributor_major',   // Major contributor changes (new primary, removal)
  'protocol_upgrade',    // Protocol version changes
  'emergency',           // Emergency actions
]);

/**
 * Impact thresholds for auto-escalation to IMMUTABLE tier
 */
const IMPACT_THRESHOLDS = {
  // Financial impact (in $)
  financial: 10000,

  // User impact (number of users affected)
  users: 100,

  // Code impact (lines changed)
  codeLines: 500,

  // Dependency impact (number of dependents)
  dependents: 5,
};

// ═══════════════════════════════════════════════════════════════════════════
// DETERMINATION LOGIC
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Determine anchoring tier for a judgment
 *
 * @param {Object} judgment - The judgment to classify
 * @param {number} judgment.qScore - Quality score (0-100)
 * @param {string} judgment.category - Judgment category
 * @param {Object} [judgment.impact] - Impact metrics
 * @returns {{tier: string, reason: string, storage: string[]}}
 */
function determineAnchoringTier(judgment) {
  const { qScore, category, impact = {} } = judgment;

  // Rule 1: Critical categories are ALWAYS immutable
  if (CRITICAL_CATEGORIES.has(category)) {
    return {
      tier: 'IMMUTABLE',
      reason: `Critical category: ${category}`,
      storage: ANCHORING_TIERS.IMMUTABLE.storage,
      merkleProof: true,
    };
  }

  // Rule 2: High impact is ALWAYS immutable
  if (impact.financial && impact.financial >= IMPACT_THRESHOLDS.financial) {
    return {
      tier: 'IMMUTABLE',
      reason: `High financial impact: $${impact.financial}`,
      storage: ANCHORING_TIERS.IMMUTABLE.storage,
      merkleProof: true,
    };
  }

  if (impact.users && impact.users >= IMPACT_THRESHOLDS.users) {
    return {
      tier: 'IMMUTABLE',
      reason: `High user impact: ${impact.users} users`,
      storage: ANCHORING_TIERS.IMMUTABLE.storage,
      merkleProof: true,
    };
  }

  // Rule 3: Q-Score based tiering
  if (qScore >= PHI_INV * 100) {
    return {
      tier: 'IMMUTABLE',
      reason: `High confidence: Q-Score ${qScore.toFixed(1)}% ≥ ${(PHI_INV * 100).toFixed(1)}%`,
      storage: ANCHORING_TIERS.IMMUTABLE.storage,
      merkleProof: true,
    };
  }

  if (qScore >= PHI_INV_2 * 100) {
    return {
      tier: 'DURABLE',
      reason: `Medium confidence: Q-Score ${qScore.toFixed(1)}%`,
      storage: ANCHORING_TIERS.DURABLE.storage,
      merkleProof: false,
    };
  }

  return {
    tier: 'VOLATILE',
    reason: `Low confidence: Q-Score ${qScore.toFixed(1)}% < ${(PHI_INV_2 * 100).toFixed(1)}%`,
    storage: ANCHORING_TIERS.VOLATILE.storage,
    merkleProof: false,
  };
}

/**
 * Check if a judgment should be anchored to PoJ Chain
 *
 * @param {Object} judgment - The judgment
 * @returns {boolean} True if should anchor to PoJ
 */
function shouldAnchorToPoJ(judgment) {
  const { tier } = determineAnchoringTier(judgment);
  return tier === 'IMMUTABLE';
}

/**
 * Get retention policy for a judgment
 *
 * @param {Object} judgment - The judgment
 * @returns {string} Retention duration
 */
function getRetentionPolicy(judgment) {
  const { tier } = determineAnchoringTier(judgment);
  return ANCHORING_TIERS[tier].retention;
}

// ═══════════════════════════════════════════════════════════════════════════
// E-SCORE TIER CHANGE DETECTION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * E-Score tiers (from asdf-manifesto)
 */
const E_SCORE_TIERS = {
  ALPHA: { min: 90, label: 'Alpha Contributor' },
  BETA: { min: 70, label: 'Beta Contributor' },
  GAMMA: { min: 50, label: 'Gamma Contributor' },
  DELTA: { min: 30, label: 'Delta Contributor' },
  EPSILON: { min: 0, label: 'New Contributor' },
};

/**
 * Get E-Score tier name
 */
function getEScoreTier(eScore) {
  for (const [name, config] of Object.entries(E_SCORE_TIERS)) {
    if (eScore >= config.min) return name;
  }
  return 'EPSILON';
}

/**
 * Check if E-Score change is tier-significant
 */
function isEScoreTierChange(oldScore, newScore) {
  return getEScoreTier(oldScore) !== getEScoreTier(newScore);
}

/**
 * Check if K-Score change is significant (> 10 points)
 */
function isKScoreSignificant(oldScore, newScore) {
  return Math.abs(newScore - oldScore) >= 10;
}

// ═══════════════════════════════════════════════════════════════════════════
// STATISTICS AND MONITORING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Calculate expected distribution of judgments across tiers
 * Based on φ distribution: ~38% volatile, ~24% durable, ~38% immutable
 */
function getExpectedDistribution() {
  return {
    VOLATILE: PHI_INV_2 * 100,    // 38.2%
    DURABLE: (PHI_INV - PHI_INV_2) * 100, // 23.6%
    IMMUTABLE: (1 - PHI_INV) * 100, // 38.2%
  };
}

/**
 * Check if actual distribution is healthy
 */
function isDistributionHealthy(actual) {
  const expected = getExpectedDistribution();

  // Allow 20% deviation
  const tolerance = 20;

  for (const tier of Object.keys(expected)) {
    const diff = Math.abs(actual[tier] - expected[tier]);
    if (diff > tolerance) {
      return {
        healthy: false,
        issue: `${tier} tier is ${diff.toFixed(1)}% off expected`,
      };
    }
  }

  return { healthy: true };
}

// ═══════════════════════════════════════════════════════════════════════════
// PRINT SUMMARY
// ═══════════════════════════════════════════════════════════════════════════

function printStrategy() {
  console.log('═══════════════════════════════════════════════════════════════════');
  console.log('🐕 PoJ ANCHORING STRATEGY');
  console.log('═══════════════════════════════════════════════════════════════════\n');

  console.log('TIERS:');
  for (const [name, config] of Object.entries(ANCHORING_TIERS)) {
    console.log(`\n  ${name}:`);
    console.log(`    Q-Score: ${config.minQScore.toFixed(1)}% - ${config.maxQScore.toFixed(1)}%`);
    console.log(`    Storage: ${config.storage.join(' + ')}`);
    console.log(`    Merkle Proof: ${config.merkleProof ? '✓' : '✗'}`);
    console.log(`    Retention: ${config.retention}`);
  }

  console.log('\n\nCRITICAL CATEGORIES (Always IMMUTABLE):');
  for (const cat of CRITICAL_CATEGORIES) {
    console.log(`  • ${cat}`);
  }

  console.log('\n\nφ-BASED THRESHOLDS:');
  console.log(`  Volatile < ${(PHI_INV_2 * 100).toFixed(1)}% (1/φ²)`);
  console.log(`  Durable  ≥ ${(PHI_INV_2 * 100).toFixed(1)}% (1/φ²)`);
  console.log(`  Immutable≥ ${(PHI_INV * 100).toFixed(1)}% (1/φ)`);

  console.log('\n\nEXPECTED DISTRIBUTION:');
  const dist = getExpectedDistribution();
  console.log(`  Volatile:  ${dist.VOLATILE.toFixed(1)}%`);
  console.log(`  Durable:   ${dist.DURABLE.toFixed(1)}%`);
  console.log(`  Immutable: ${dist.IMMUTABLE.toFixed(1)}%`);

  console.log('\n═══════════════════════════════════════════════════════════════════');
}

// ═══════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,

  // Tiers
  ANCHORING_TIERS,
  CRITICAL_CATEGORIES,
  IMPACT_THRESHOLDS,
  E_SCORE_TIERS,

  // Functions
  determineAnchoringTier,
  shouldAnchorToPoJ,
  getRetentionPolicy,
  getEScoreTier,
  isEScoreTierChange,
  isKScoreSignificant,
  getExpectedDistribution,
  isDistributionHealthy,

  // Display
  printStrategy,
};
