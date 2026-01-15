/**
 * @cynic/core - Core constants, axioms, scoring, identity, and worlds
 *
 * "φ qui se méfie de φ" - κυνικός
 *
 * This package provides:
 * - PHI constants (source of truth for φ)
 * - Q-Score calculation (knowledge quality)
 * - CYNIC Identity (personality, voice, verdicts)
 * - 4 Worlds (Kabbalah framework)
 *
 * @module @cynic/core
 */

// Axioms and constants (source of truth for φ)
export * from './axioms/index.js';

// Q-Score calculation
export * from './qscore/index.js';

// CYNIC Identity (personality, voice, verdicts)
export * from './identity/index.js';

// 4 Worlds (Kabbalah framework)
export * from './worlds/index.js';

// Configuration management (security by design)
export * from './config/index.js';
