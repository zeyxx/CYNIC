/**
 * Claude Flow Service Factories
 *
 * Creates services for Claude Flow integration (Phase 21):
 * - ComplexityClassifier
 * - TieredRouter
 * - AgentBooster
 * - TokenOptimizer
 * - HyperbolicSpace
 * - SONA
 *
 * @module @cynic/mcp/server/service-initializer/claude-flow-factories
 */

'use strict';

import {
  createSONA, createTieredRouter, createAgentBooster,
  createTokenOptimizer, createHyperbolicSpace, createComplexityClassifier,
  createBehaviorModifier, createMetaCognition,
} from '@cynic/node';
import { createLogger } from '@cynic/core';
import fs from 'fs';
import path from 'path';
import os from 'os';

const log = createLogger('ClaudeFlowFactories');

/**
 * Create Complexity Classifier
 * Classifies requests into LOCAL/LIGHT/FULL tiers
 */
export function createComplexityClassifierFactory() {
  const classifier = createComplexityClassifier();
  log.debug('ComplexityClassifier ready');
  return classifier;
}

/**
 * Create Tiered Router
 * Routes requests to appropriate handlers based on complexity
 */
export function createTieredRouterFactory(services) {
  const router = createTieredRouter({
    classifier: services.complexityClassifier,
  });
  log.debug('TieredRouter ready');
  return router;
}

/**
 * Create Agent Booster
 * Fast code transforms without LLM (< 1ms, $0)
 */
export function createAgentBoosterFactory() {
  const booster = createAgentBooster();
  log.debug('AgentBooster ready', { transforms: 12 });
  return booster;
}

/**
 * Create Token Optimizer
 * Compression and caching for token efficiency
 */
export function createTokenOptimizerFactory() {
  const optimizer = createTokenOptimizer();
  log.debug('TokenOptimizer ready', { strategies: 4 });
  return optimizer;
}

/**
 * Create Hyperbolic Space
 * Poincaré ball model for hierarchical embeddings
 */
export function createHyperbolicSpaceFactory() {
  const space = createHyperbolicSpace({ dim: 8 });
  log.debug('HyperbolicSpace ready', { dim: 8 });
  return space;
}

/**
 * Create SONA (Self-Optimizing Neural Adaptation)
 * Correlates patterns to dimensions for adaptive learning
 */
export function createSONAFactory(services) {
  const sona = createSONA({
    learningService: services.learningService,
  });

  // Load persisted state if available
  const stateFile = path.join(os.homedir(), '.cynic', 'sona', 'state.json');
  try {
    if (fs.existsSync(stateFile)) {
      const raw = fs.readFileSync(stateFile, 'utf8');
      const state = JSON.parse(raw);
      if (state && state.stats) {
        // Restore stats only (observations are ephemeral, correlations rebuild)
        Object.assign(sona.stats, state.stats);
        log.info('SONA state restored', { observations: state.stats.totalObservations });
      }
    }
  } catch (e) {
    log.debug('SONA state load skipped', { error: e.message });
  }

  // P4: Start adaptation loop (was never called — timer dormant)
  sona.start();

  log.debug('SONA ready + started', { adaptationRate: 0.236 });
  return sona;
}

/**
 * Create BehaviorModifier (feedback → actual behavior changes)
 * Connects USER_FEEDBACK to routing/judgment/confidence adjustments
 */
export function createBehaviorModifierFactory(services) {
  const modifier = createBehaviorModifier({
    qRouter: services.tieredRouter || null,
  });
  log.debug('BehaviorModifier ready');
  return modifier;
}

/**
 * Create MetaCognition (self-monitoring + strategy switching)
 * Tracks tool actions, detects stuck/thrashing, recommends strategies
 */
export function createMetaCognitionFactory() {
  const mcDir = path.join(os.homedir(), '.cynic', 'metacognition');
  const stateFile = path.join(mcDir, 'state.json');

  const meta = createMetaCognition({
    onStateChange: (newState, prevState, reason) => {
      log.info('MetaCognition state change', { from: prevState, to: newState, reason });
    },
    // P5: Persist state on strategy switch (so Router sees mid-session changes)
    onStrategySwitch: (strategy, reason) => {
      log.info('MetaCognition strategy switch', { strategy, reason });
      try {
        fs.mkdirSync(mcDir, { recursive: true });
        fs.writeFileSync(stateFile, JSON.stringify(meta.exportState(), null, 2));
      } catch { /* Non-blocking */ }
    },
  });
  log.debug('MetaCognition ready');
  return meta;
}
