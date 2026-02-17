/**
 * CYNIC LLM Package
 *
 * Unified LLM integration for the CYNIC ecosystem.
 *
 * Provides:
 * - Multi-LLM routing and consensus
 * - Provider adapters (Ollama, Claude Code, OpenAI-compatible)
 * - AirLLM integration for large models
 * - φ-aligned confidence thresholds
 *
 * @module @cynic/llm
 */

'use strict';

// Types
export {
  LLMResponse,
  ConsensusResult,
  ExecutionTier,
  LLMProvider,
  ConfidenceThresholds,
} from './types.js';

// Adapters (standalone in @cynic/llm)
export {
  LLMAdapter,
  ClaudeCodeAdapter,
  OSSLLMAdapter,
  AirLLMAdapter,
  createOllamaValidator,
  createLMStudioValidator,
  createOpenAIValidator,
  createAirLLMValidator,
  checkAirLLMAvailability,
  AnthropicAdapter,
  createAnthropicValidator,
  AnthropicModelMap,
} from './adapters/index.js';

// Router (now standalone in @cynic/llm)
export {
  LLMRouter,
  createLLMRouter,
  getLLMRouter,
  getRouterWithValidators,
  createHybridRouter,
  createValidatorsFromEnv,
  createValidatorsFromDetection,
  _resetLLMRouterForTesting,
  // Cost optimization
  ComplexityClassifier,
  TIER_COSTS,
  TIER_LATENCIES,
} from './router.js';

// Semantic Similarity
export {
  tokenize,
  removeStopwords,
  jaccardSimilarity,
  textSimilarity,
  clusterBySimilarity,
  calculateSemanticAgreement,
  SimilarityThresholds,
} from './similarity.js';

// Perception Router
export {
  PerceptionRouter,
  PerceptionLayer,
  getPerceptionRouter,
  _resetPerceptionRouterForTesting,
} from './perception-router.js';

// Convenience re-exports from @cynic/core
export { PHI_INV, PHI_INV_2 } from '@cynic/core';

// Pricing Oracle - Real-time cost calculation
export {
  PricingOracle,
  createPricingOracle,
  getOracle,
  calculateCost,
  compareCosts,
  PROVIDER_PRICING,
  DEFAULT_CONFIG,
} from './pricing/index.js';

// Retrieval - PageIndex (Reasoning-based RAG)
export {
  PageIndex,
  IndexNode,
  createPageIndex,
} from './retrieval/index.js';

// Orchestration - Prometheus + Atlas
export {
  Prometheus,
  Atlas,
  createPrometheus,
  createAtlas,
  TaskType,
  ExecutionStatus,
  PlanStep,
  ExecutionPlan,
} from './orchestration/index.js';

// Intelligent Switch
export {
  IntelligentSwitch,
  createIntelligentSwitch,
  createForPriority,
  Strategy,
  Priority,
} from './adapters/intelligent-switch.js';

// Learning Engine
export {
  LearningEngine,
  createLearningEngine,
  LearningEvent,
  EventType,
  AdapterTracker,
} from './learning/index.js';

// CYNIC Bridge - Integration Layer
export {
  CYNICBridge,
  createCYNICBridge,
} from './integration/cynic-bridge.js';
