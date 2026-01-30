/**
 * CYNIC Orchestration Module
 *
 * Unified orchestration layer for coordinating:
 * - Hooks (perceive, guard, observe, digest)
 * - KETER (routing decisions)
 * - Dogs (parallel voting)
 * - Engines (philosophical synthesis)
 * - Skills (action execution)
 *
 * Phase 19: Complete Orchestration Layer
 *
 * "φ coordinates all" - κυνικός
 *
 * @module @cynic/node/orchestration
 */

'use strict';

// Decision Event - Unified event model
export {
  DecisionEvent,
  DecisionStage,
  DecisionOutcome,
  EventSource,
  createFromHook,
  createFromTool,
} from './decision-event.js';

// Unified Orchestrator - Central facade
export {
  UnifiedOrchestrator,
  createUnifiedOrchestrator,
  getOrchestrator,
} from './unified-orchestrator.js';

// Skill Registry - Domain to skill mapping
export {
  SkillRegistry,
  createSkillRegistry,
} from './skill-registry.js';

// Decision Tracer - Recording and visualization
export {
  DecisionTracer,
  StorageMode,
  createDecisionTracer,
} from './decision-tracer.js';

// Circuit Breaker - Resilience pattern
export {
  CircuitBreaker,
  CircuitBreakerRegistry,
  CircuitState,
  createCircuitBreaker,
  getCircuitBreakerRegistry,
} from './circuit-breaker.js';

// Kabbalistic Router - Tree of Life decision flow (Phase 4)
export {
  KabbalisticRouter,
  createKabbalisticRouter,
} from './kabbalistic-router.js';
