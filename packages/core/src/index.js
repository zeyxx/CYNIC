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
export { ECOSYSTEM_SEED } from './ecosystem/asdfasdfa-ecosystem.js';

// Custom Error Types (programmatic error handling)
export * from './errors.js';

// Harmony Module (φ-based economic engine, adopted from HolDex)
export * from './harmony.js';

// Cryptographic Utilities (secure random generation)
export * from './crypto-utils.js';

// Structured Logging (phi-aligned log levels)
export * from './logger.js';

// Dependency Injection Container (SOLID foundation)
export * from './container.js';

// Engine System (philosophical and analytical engines)
export * from './engines/index.js';

// Boot System (lifecycle management, auto-discovery, unified startup)
export * from './boot/index.js';

// Communication Bus (N-tier, SOLID, inter-layer messaging)
export * from './bus/index.js';

// Circuit Breaker (resilience pattern for service protection)
export * from './circuit-breaker.js';

// Retry Policy (automatic retry logic with exponential backoff)
export * from './retry/index.js';

// Context Intelligence (C-Score, budget management, context assembly)
export * from './context/index.js';

// CLI Utilities (v1.1: colors, progress, status display)
export * from './cli/index.js';

// LLM Provider Abstraction (Claude, Ollama, OpenAI)
export * from './llm/index.js';

// Distributed Tracing (φ-aligned spans, sampling, middleware)
export * from './tracing/index.js';

// System Topology (self-awareness: mode, services, capabilities, 7×7 matrix)
export * from './topology/index.js';

// Intelligence (prompt classification, context routing)
export * from './intelligence/index.js';

// Timer Tracker (test-only debugging for leaked timers)
export { timerTracker } from './timers.js';
