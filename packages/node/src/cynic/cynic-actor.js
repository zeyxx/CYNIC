/**
 * CYNIC Actor - C6.4 (CYNIC × ACT)
 *
 * Self-optimization: CYNIC executes internal optimizations based on decisions from C6.3.
 * Acts on memory pressure, learning velocity, LLM routing, and collective health.
 *
 * "Le chien s'optimise lui-même" - κυνικός
 *
 * Actions:
 * - Memory optimization (context compression, garbage collection)
 * - LLM routing shifts (Anthropic ↔ Ollama based on budget)
 * - Learning control (pause, resume, priority shifts)
 * - Dog rotation (rebalance collective consciousness)
 * - Alert escalation (notify operators of critical patterns)
 *
 * Generated via createActor factory (65% template + 35% config).
 *
 * @module @cynic/node/cynic/cynic-actor
 */

'use strict';

import { createActor } from '../cycle/create-actor.js';
import { cynicActorConfig, CynicActionType } from '../cycle/configs/cynic-actor.config.js';

// Generate the CynicActor class from config
const { Class: CynicActor, ActionStatus, getInstance, resetInstance } = createActor(cynicActorConfig);

// Create aliases for consistency with naming convention
const getCynicActor = getInstance;
const resetCynicActor = resetInstance;

// Export for external use
export {
  CynicActor,
  CynicActionType,
  ActionStatus,
  getCynicActor,
  resetCynicActor,
};

export default { CynicActor, CynicActionType, ActionStatus, getCynicActor, resetCynicActor };
