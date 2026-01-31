/**
 * Routing Module
 *
 * Intelligent request routing with complexity-based tiering.
 *
 * @module @cynic/node/routing
 */

'use strict';

export {
  ComplexityClassifier,
  createComplexityClassifier,
  ComplexityTier,
  COMPLEXITY_THRESHOLDS,
} from './complexity-classifier.js';

export {
  TieredRouter,
  createTieredRouter,
  HANDLER_COSTS,
  HANDLER_LATENCIES,
} from './tiered-router.js';

export {
  AgentBooster,
  createAgentBooster,
  TransformIntent,
  TransformStatus,
} from './agent-booster.js';

// Re-export for convenience
import { ComplexityClassifier, createComplexityClassifier, ComplexityTier, COMPLEXITY_THRESHOLDS } from './complexity-classifier.js';
import { TieredRouter, createTieredRouter, HANDLER_COSTS, HANDLER_LATENCIES } from './tiered-router.js';
import { AgentBooster, createAgentBooster, TransformIntent, TransformStatus } from './agent-booster.js';

export default {
  ComplexityClassifier,
  createComplexityClassifier,
  ComplexityTier,
  COMPLEXITY_THRESHOLDS,
  TieredRouter,
  createTieredRouter,
  HANDLER_COSTS,
  HANDLER_LATENCIES,
  AgentBooster,
  createAgentBooster,
  TransformIntent,
  TransformStatus,
};
