/**
 * Learning Module
 *
 * Exports SONA, ReasoningBank, and related learning components.
 *
 * @module @cynic/node/learning
 */

'use strict';

export {
  SONA,
  createSONA,
  SONA_CONFIG,
} from './sona.js';

export {
  ReasoningBank,
  Trajectory,
  TrajectoryState,
  TrajectoryAction,
  TrajectoryOutcome,
  TrajectoryType,
  OutcomeType,
  REASONING_BANK_CONFIG,
  createReasoningBank,
} from './reasoning-bank.js';

export default {
  SONA,
  createSONA,
  SONA_CONFIG,
  ReasoningBank,
  Trajectory,
  TrajectoryState,
  TrajectoryAction,
  TrajectoryOutcome,
  TrajectoryType,
  OutcomeType,
  REASONING_BANK_CONFIG,
  createReasoningBank,
};

// P2.3: Behavior Modifier
export {
  BehaviorModifier,
  BehaviorAdjustment,
  AdjustmentType,
  BEHAVIOR_CONFIG,
  createBehaviorModifier,
} from './behavior-modifier.js';

// P3.4: Meta-Cognitive Monitoring
export {
  MetaCognition,
  CognitiveState,
  StrategyType,
  ActionRecord,
  StrategyRecord,
  META_CONFIG,
  createMetaCognition,
  getMetaCognition,
} from './meta-cognition.js';

// Re-export from sona for convenience
import { SONA, createSONA, SONA_CONFIG } from './sona.js';
import {
  ReasoningBank,
  Trajectory,
  TrajectoryState,
  TrajectoryAction,
  TrajectoryOutcome,
  TrajectoryType,
  OutcomeType,
  REASONING_BANK_CONFIG,
  createReasoningBank,
} from './reasoning-bank.js';
import {
  BehaviorModifier,
  BehaviorAdjustment,
  AdjustmentType,
  BEHAVIOR_CONFIG,
  createBehaviorModifier,
} from './behavior-modifier.js';
