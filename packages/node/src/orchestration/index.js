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

// Circuit Breaker - Re-exported from @cynic/core for convenience
// NOTE: The canonical circuit breaker is in @cynic/core
export {
  CircuitBreaker,
  CircuitBreakerRegistry,
  CircuitState,
  createCircuitBreaker,
  withCircuitBreaker,
  getCircuitBreakerRegistry,
} from '@cynic/core';

// Routing Configuration - Centralized KETER routing
export {
  SEFIROT_ROUTING,
  TRUST_THRESHOLDS,
  RISK_INTERVENTIONS,
  DANGER_PATTERNS,
  calculateTrustLevel,
  determineIntervention,
  detectRisk,
  findRouting,
  getDomains,
  getRoutingForDomain,
} from './routing-config.js';

// Kabbalistic Router - Tree of Life decision flow (Phase 4)
export {
  KabbalisticRouter,
  createKabbalisticRouter,
} from './kabbalistic-router.js';

// Q-Learning Router - Learned dog selection (P2.1)
export {
  QLearningRouter,
  QTable,
  Q_CONFIG,
  StateFeatures,
  Actions,
  createQLearningRouter,
  getQLearningRouter,
} from './q-learning-router.js';

// P3.1: Skill Auto-Activation
export {
  SkillActivator,
  SkillMatch,
  ACTIVATOR_CONFIG,
  createSkillActivator,
  getSkillActivator,
  findRulesFile,
} from './skill-activator.js';

// P3.2: MCP Server Instructions
export {
  MCPInstructions,
  ServerInstructions,
  MCP_INSTRUCTIONS_CONFIG,
  createMCPInstructions,
  getMCPInstructions,
  findInstructionsFile,
} from './mcp-instructions.js';

// P3.3: Security Audit Trail
export {
  SecurityAuditTrail,
  AuditEvent,
  AuditEventType,
  SensitivityLevel,
  AUDIT_CONFIG,
  createSecurityAuditTrail,
  getSecurityAuditTrail,
} from './security-audit.js';

// Q-Learning Service - extracted Q-learning from QLearningRouter
export {
  QLearningService,
  QTable as QLearningQTable,  // Renamed to avoid conflict with q-learning-router.js QTable
  StateFeatures as QLearningStateFeatures,
  LEARNING_CONFIG,
  createQLearningService,
  getQLearningService,
  getQLearningServiceAsync,
  _resetQLearningServiceForTesting,
} from './learning-service.js';

// ═══════════════════════════════════════════════════════════════════════════
// BRAIN/OS/CPU ARCHITECTURE (Phase: Foundational)
// ═══════════════════════════════════════════════════════════════════════════

// Brain - Consciousness Layer (dogs, engines, memory)
export {
  Brain,
  BrainState,
  Thought,
  createBrain,
  getBrain,
  _resetBrainForTesting,
} from './brain.js';

// LLM Adapter - CPU Layer (Claude, OSS LLMs, consensus)
export {
  LLMAdapter,
  LLMResponse,
  LLMRouter,
  ConsensusResult,
  ClaudeCodeAdapter,
  OSSLLMAdapter,
  createLLMRouter,
  getLLMRouter,
  _resetLLMRouterForTesting,
  // Task #59: OSS LLM validator factories
  createOllamaValidator,
  createLMStudioValidator,
  createOpenAIValidator,
  createValidatorsFromEnv,
  createValidatorsFromDetection,
  getRouterWithValidators,
  // Task #98: AirLLM integration (large models via disk offloading)
  AirLLMAdapter,
  createAirLLMValidator,
  checkAirLLMAvailability,
  createHybridRouter,
} from './llm-adapter.js';

// LLM Orchestrator - Da'at Bridge (Task #93)
export {
  LLMOrchestrator,
  ExecutionTier,
  createLLMOrchestrator,
} from './llm-orchestrator.js';

// ═══════════════════════════════════════════════════════════════════════════
// PLANNING GATE (Meta-Cognition Layer)
// ═══════════════════════════════════════════════════════════════════════════

// Planning Gate - Think before acting
export {
  PlanningGate,
  PlanningResult,
  PlanningTrigger,
  PlanningDecision,
  PLANNING_THRESHOLDS,
  createPlanningGate,
  getPlanningGate,
  _resetPlanningGateForTesting,
} from './planning-gate.js';

// ═══════════════════════════════════════════════════════════════════════════
// CHAOS ENGINEERING
// ═══════════════════════════════════════════════════════════════════════════

// Chaos Generator - System robustness testing
export {
  ChaosGenerator,
  ChaosResult,
  ChaosEventType,
  CHAOS_CONFIG,
  createChaosGenerator,
  getChaosGenerator,
  _resetChaosGeneratorForTesting,
} from '../chaos/index.js';
