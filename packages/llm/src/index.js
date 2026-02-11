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
