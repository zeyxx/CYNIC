/**
 * CYNIC Core Constants - φ derives all
 *
 * "All ratios derive from 1.618..."
 *
 * This is the SINGLE SOURCE OF TRUTH for φ constants.
 * Import from here, never redefine.
 *
 * @module @cynic/core/axioms/constants
 */

'use strict';

// =============================================================================
// PHI - THE GOLDEN RATIO (SINGLE SOURCE OF TRUTH)
// =============================================================================

/**
 * φ (phi) - The Golden Ratio
 * Irrational number: (1 + √5) / 2
 */
export const PHI = 1.618033988749895;

/**
 * φ⁻¹ - Inverse of phi (also equals φ - 1)
 * Maximum confidence threshold: 61.8%
 */
export const PHI_INV = 0.618033988749895;

/**
 * φ⁻² - Second inverse power
 * Minimum doubt threshold: 38.2%
 * Also: 1 - φ⁻¹
 */
export const PHI_INV_2 = 0.381966011250105;

/**
 * φ⁻³ - Third inverse power
 * Critical threshold: 23.6%
 */
export const PHI_INV_3 = 0.236067977499790;

/**
 * φ² - Phi squared
 * Also equals φ + 1 = 2.618...
 */
export const PHI_2 = 2.618033988749895;

/**
 * φ³ - Phi cubed
 */
export const PHI_3 = 4.236067977499790;

// =============================================================================
// TIMING CONSTANTS (φ-HIERARCHICAL, BASE 100ms)
// =============================================================================

/**
 * Base timing unit in milliseconds
 * All other timings derive from this via φ
 */
export const TIMING_BASE_MS = 100;

/**
 * TICK - Atomic events
 * 23.6ms (base × φ⁻³)
 */
export const TICK_MS = Math.round(TIMING_BASE_MS * PHI_INV_3 * 10) / 10;

/**
 * MICRO - Acknowledgments
 * 38.2ms (base × φ⁻²)
 */
export const MICRO_MS = Math.round(TIMING_BASE_MS * PHI_INV_2 * 10) / 10;

/**
 * SLOT - Block proposal window
 * 61.8ms (base × φ⁻¹)
 */
export const SLOT_MS = Math.round(TIMING_BASE_MS * PHI_INV * 10) / 10;

/**
 * BLOCK - Finalization
 * 100ms (base)
 */
export const BLOCK_MS = TIMING_BASE_MS;

/**
 * EPOCH - Checkpoint
 * 161.8ms (base × φ)
 */
export const EPOCH_MS = Math.round(TIMING_BASE_MS * PHI * 10) / 10;

/**
 * CYCLE - Governance window
 * 261.8ms (base × φ²)
 */
export const CYCLE_MS = Math.round(TIMING_BASE_MS * PHI_2 * 10) / 10;

// =============================================================================
// NETWORK CONSTANTS (φ-DERIVED)
// =============================================================================

/**
 * Gossip fanout - number of peers per hop
 * Fib(7) = 13
 */
export const GOSSIP_FANOUT = 13;

/**
 * Consensus threshold
 * φ⁻¹ = 61.8%
 */
export const CONSENSUS_THRESHOLD = PHI_INV;

/**
 * Minimum sources for pattern validation
 * Fib(4) = 3
 */
export const MIN_PATTERN_SOURCES = 3;

/**
 * Governance quorum
 * Fib(5) = 5
 */
export const GOVERNANCE_QUORUM = 5;

// =============================================================================
// FIBONACCI SEQUENCE (for reference)
// =============================================================================

/**
 * First 15 Fibonacci numbers
 * Used for scaling, fanout, thresholds
 */
export const FIBONACCI = [1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610];

/**
 * Get Fibonacci number by index (1-indexed for readability)
 * Fib(7) = 13, Fib(11) = 89, etc.
 */
export function fib(n) {
  if (n < 1 || n > FIBONACCI.length) {
    throw new Error(`Fibonacci index must be 1-${FIBONACCI.length}`);
  }
  return FIBONACCI[n - 1];
}

// =============================================================================
// THRESHOLDS
// =============================================================================

export const THRESHOLDS = {
  // Verdict thresholds
  HOWL: 80,           // Exceptional
  WAG: 50,            // Passes
  GROWL: PHI_INV_2 * 100,  // 38.2 - Needs work
  BARK: 0,            // Critical

  // Confidence bounds
  MAX_CONFIDENCE: PHI_INV,      // 61.8%
  MIN_DOUBT: PHI_INV_2,         // 38.2%

  // Health thresholds
  HEALTHY: 62,        // φ⁻¹ rounded
  WARNING: 38,        // φ⁻²
  CRITICAL: 24,       // φ⁻³

  // Anomaly detection
  ANOMALY: PHI_INV_2,           // 38.2% residual = anomaly
};

// =============================================================================
// THE 4 AXIOMS
// =============================================================================

export const AXIOMS = {
  PHI: {
    symbol: 'φ',
    name: 'PHI',
    principle: 'All ratios derive from 1.618...',
    world: 'ATZILUT',
    color: '#FFD700'
  },
  VERIFY: {
    symbol: '✓',
    name: 'VERIFY',
    principle: "Don't trust, verify",
    world: 'BERIAH',
    color: '#4169E1'
  },
  CULTURE: {
    symbol: '⛩',
    name: 'CULTURE',
    principle: 'Culture is a moat',
    world: 'YETZIRAH',
    color: '#228B22'
  },
  BURN: {
    symbol: '🔥',
    name: 'BURN',
    principle: "Don't extract, burn",
    world: 'ASSIAH',
    color: '#DC143C'
  }
};

// =============================================================================
// EXPORT ALL (for destructuring convenience)
// =============================================================================

export default {
  // PHI constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,
  PHI_2,
  PHI_3,

  // Timing
  TIMING_BASE_MS,
  TICK_MS,
  MICRO_MS,
  SLOT_MS,
  BLOCK_MS,
  EPOCH_MS,
  CYCLE_MS,

  // Network
  GOSSIP_FANOUT,
  CONSENSUS_THRESHOLD,
  MIN_PATTERN_SOURCES,
  GOVERNANCE_QUORUM,

  // Helpers
  FIBONACCI,
  fib,
  THRESHOLDS,
  AXIOMS,
};
