/**
 * CYNIC Chaos Engineering Module
 *
 * "Un système qui survit au hasard survit à tout"
 *
 * Provides controlled chaos injection for testing system robustness.
 *
 * @module @cynic/node/chaos
 */

'use strict';

export {
  ChaosGenerator,
  ChaosResult,
  ChaosEventType,
  CHAOS_CONFIG,
  createChaosGenerator,
  getChaosGenerator,
  _resetChaosGeneratorForTesting,
} from './chaos-generator.js';
