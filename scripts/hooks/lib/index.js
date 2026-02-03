/**
 * CYNIC Hooks Library
 *
 * Shared utilities for CYNIC hooks.
 *
 * @module scripts/hooks/lib
 */

'use strict';

// Core hook utilities
export * from './base-hook.js';
export * from './pattern-detector.js';

// Session state management (Phase 22)
export { SessionStateManager, getSessionState } from './session-state.js';

// Temporal Perception (Time awareness for psychology)
export {
  TemporalPerception,
  getTemporalPerception,
  resetTemporalPerception,
  TEMPORAL_THRESHOLDS,
  TemporalState,
  TemporalTrend,
} from './temporal-perception.js';

// Error Perception (Tool failure awareness for psychology)
export {
  ErrorPerception,
  getErrorPerception,
  resetErrorPerception,
  ERROR_THRESHOLDS,
  ErrorSeverity,
  ErrorPattern,
} from './error-perception.js';

// Orchestration client (Phase 22)
export { OrchestrationClient, getOrchestrationClient, initOrchestrationClient } from './orchestration-client.js';

// Feedback collection (Phase 22)
export { FeedbackCollector, getFeedbackCollector, ANTI_PATTERNS } from './feedback-collector.js';

// Suggestion engine (Phase 22)
export { SuggestionEngine, getSuggestionEngine } from './suggestion-engine.js';

// Auto-Orchestrator (Automatic Dog consultation)
export {
  AutoOrchestrator,
  getAutoOrchestrator,
  getAutoOrchestratorSync,
  CONFIG as AUTO_ORCHESTRATOR_CONFIG,
} from './auto-orchestrator.js';

// Rules loader (S1: Skill auto-activation via rules.json)
export {
  loadRulesFile,
  getSkillTriggers,
  getRulesSettings,
  detectSkillTriggersFromRules,
  clearRulesCache,
} from './rules-loader.js';

// Implicit Feedback Detection (Task #72)
export {
  ImplicitFeedbackDetector,
  getImplicitFeedback,
  resetImplicitFeedback,
  FeedbackType,
  FeedbackSentiment,
  IMPLICIT_FEEDBACK_CONFIG,
} from './implicit-feedback.js';

// Harmonic Feedback System (Kabbalistic + Cybernetic + Bayesian synthesis)
export {
  HarmonicFeedbackSystem,
  getHarmonicFeedback,
  resetHarmonicFeedback,
  ThompsonSampler,
  ConfidenceCalibrator,  // Task #71: Confidence calibration
  calculateCoherence,
  calculateResonance,
  calculateEntrainment,
  temporalDecay,
  eligibilityTrace,
  SEFIROT_CHANNELS,
  FeedbackState,
  PROMOTION_CONFIG,      // Task #70: Pattern-to-heuristic promotion
  CALIBRATION_CONFIG,    // Task #71: Confidence calibration config
} from './harmonic-feedback.js';

// ReasoningBank (P1.2: Trajectory learning)
let _reasoningBank = null;

export function getReasoningBank() {
  if (_reasoningBank) return _reasoningBank;

  try {
    // Dynamic import to avoid circular dependencies
    const { createReasoningBank } = require('@cynic/node/learning');
    _reasoningBank = createReasoningBank();
    return _reasoningBank;
  } catch (e) {
    // ReasoningBank not available - return null
    return null;
  }
}

// FactExtractor (M2: Auto fact extraction)
let _factExtractor = null;

export function getFactExtractor() {
  if (_factExtractor) return _factExtractor;

  try {
    // Dynamic import to avoid circular dependencies
    const { createFactExtractor } = require('@cynic/persistence/services');
    const { getPool } = require('@cynic/persistence');
    const pool = getPool();
    if (pool) {
      _factExtractor = createFactExtractor({ pool });
      return _factExtractor;
    }
  } catch (e) {
    // FactExtractor not available - return null
  }
  return null;
}

// FactsRepository (M2.1: Cross-session fact retrieval)
let _factsRepository = null;

export function getFactsRepository() {
  if (_factsRepository) return _factsRepository;

  try {
    const { FactsRepository } = require('@cynic/persistence/postgres/repositories/facts');
    const { getPool } = require('@cynic/persistence');
    const pool = getPool();
    if (pool) {
      _factsRepository = new FactsRepository(pool);
      return _factsRepository;
    }
  } catch (e) {
    // FactsRepository not available - return null
  }
  return null;
}

// ArchitecturalDecisionsRepository (Self-Knowledge: Decision awareness)
let _archDecisionsRepository = null;

export function getArchitecturalDecisionsRepository() {
  if (_archDecisionsRepository) return _archDecisionsRepository;

  try {
    const { ArchitecturalDecisionsRepository } = require('@cynic/persistence/postgres/repositories/architectural-decisions');
    const { getPool } = require('@cynic/persistence');
    const pool = getPool();
    if (pool) {
      _archDecisionsRepository = new ArchitecturalDecisionsRepository(pool);
      return _archDecisionsRepository;
    }
  } catch (e) {
    // ArchitecturalDecisionsRepository not available - return null
  }
  return null;
}

// CodebaseIndexer (Self-Knowledge: Codebase awareness)
const _codebaseIndexer = null;

export function getCodebaseIndexer(options = {}) {
  // Always create fresh to allow different options
  try {
    const { createCodebaseIndexer } = require('@cynic/persistence/services');
    return createCodebaseIndexer(options);
  } catch (e) {
    // CodebaseIndexer not available - return null
  }
  return null;
}

// BurnAnalyzer (Vision → Compréhension → Burn)
const _burnAnalyzer = null;

export function getBurnAnalyzer(options = {}) {
  // Always create fresh to allow different options
  try {
    const { createBurnAnalyzer } = require('@cynic/persistence/services/burn-analyzer');
    return createBurnAnalyzer(options);
  } catch (e) {
    // BurnAnalyzer not available - return null
  }
  return null;
}

// SessionRepository (GAP #1 FIX: Direct PostgreSQL session persistence)
let _sessionRepository = null;

export function getSessionRepository() {
  if (_sessionRepository) return _sessionRepository;

  try {
    const { SessionRepository } = require('@cynic/persistence/postgres/repositories/sessions');
    const { getPool } = require('@cynic/persistence');
    const pool = getPool();
    if (pool) {
      _sessionRepository = new SessionRepository(pool);
      return _sessionRepository;
    }
  } catch (e) {
    // SessionRepository not available - return null
  }
  return null;
}

// TelemetryCollector (Stats, frictions, benchmarking)
let _telemetry = null;

export function getTelemetryCollector() {
  if (_telemetry) return _telemetry;

  try {
    const { getTelemetry } = require('@cynic/persistence/services');
    _telemetry = getTelemetry();
    return _telemetry;
  } catch (e) {
    // Telemetry not available - return null
  }
  return null;
}

// Shorthand telemetry helpers
export function recordMetric(name, value, labels = {}) {
  const t = getTelemetryCollector();
  if (t) t.increment(name, value, labels);
}

export function recordTiming(name, durationMs, labels = {}) {
  const t = getTelemetryCollector();
  if (t) t.timing(name, durationMs, labels);
}

export function recordFriction(name, severity, details = {}) {
  const t = getTelemetryCollector();
  if (t) t.friction(name, severity, details);
}
