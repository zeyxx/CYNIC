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

// Self-Refinement (critique and improve judgments)
export * from './refinement/index.js';

// Background Agent Orchestration (multi-agent coordination)
export * from './orchestration/index.js';

// Vector Search / Embeddings (semantic search)
export * from './vector/index.js';

// Learning Loop (feedback → calibration → improvement)
export * from './learning/index.js';

// Auto-Judgment Triggers (event → judge → learn)
export * from './triggers/index.js';

// Ecosystem Monitor (external sources → updates → E-Score)
export * from './ecosystem/index.js';

// Custom Error Types (programmatic error handling)
export * from './errors.js';

// Cryptographic Utilities (secure random generation)
export * from './crypto-utils.js';
