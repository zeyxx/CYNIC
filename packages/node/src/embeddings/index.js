/**
 * Embeddings Module
 *
 * Vector embeddings and representation learning utilities.
 *
 * @module @cynic/node/embeddings
 */

'use strict';

export {
  HyperbolicSpace,
  createHyperbolicSpace,
  PoincareOperations,
  HYPERBOLIC_CONFIG,
} from './hyperbolic.js';

// Re-export for convenience
import {
  HyperbolicSpace,
  createHyperbolicSpace,
  PoincareOperations,
  HYPERBOLIC_CONFIG,
} from './hyperbolic.js';

export default {
  HyperbolicSpace,
  createHyperbolicSpace,
  PoincareOperations,
  HYPERBOLIC_CONFIG,
};
