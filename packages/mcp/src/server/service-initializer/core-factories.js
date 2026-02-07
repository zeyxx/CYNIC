/**
 * Core Service Factories
 *
 * Creates basic services: EScore, Learning, Persistence, Session, Judge
 *
 * @module @cynic/mcp/server/service-initializer/core-factories
 */

'use strict';

import {
  CYNICJudge, LearningService, LearningManager,
  createEScoreCalculator, createEngineIntegration,
  createAutomationExecutor, getEventBus,
} from '@cynic/node';
import { PeriodicScheduler, EngineRegistry, loadPhilosophyEngines, createLogger } from '@cynic/core';

import { PersistenceManager } from '../../persistence.js';
import { SessionManager } from '../../session-manager.js';
import { MetricsService } from '../../metrics-service.js';

const log = createLogger('CoreFactories');

/**
 * Create E-Score calculator
 */
export function createEScoreCalculatorFactory() {
  return createEScoreCalculator({
    burnScale: 1e9,
    minJudgments: 10,
  });
}

/**
 * Create Learning Service
 */
export function createLearningServiceFactory(services) {
  return new LearningService({
    persistence: services.persistence,
    learningRate: 0.236,  // φ⁻³
    decayRate: 0.146,     // φ⁻⁴
    minFeedback: 5,
  });
}

/**
 * Create Engine Registry with 73 philosophy engines
 */
export async function createEngineRegistryFactory() {
  const registry = new EngineRegistry();
  const result = loadPhilosophyEngines({ registry, silent: true });
  log.info('Philosophy engines loaded', { count: result.registered });
  return registry;
}

/**
 * Create CYNIC Judge
 */
export function createJudgeFactory(services) {
  const engineIntegration = services.engineRegistry
    ? createEngineIntegration({ registry: services.engineRegistry })
    : null;

  return new CYNICJudge({
    eScoreProvider: services.eScoreCalculator,
    learningService: services.learningService,
    engineIntegration,
    consultEngines: !!engineIntegration,
  });
}

/**
 * Create Persistence Manager
 */
export async function createPersistenceFactory(config) {
  const persistence = new PersistenceManager({
    dataDir: config.dataDir,
  });
  await persistence.initialize();
  return persistence;
}

/**
 * Create Session Manager
 */
export function createSessionManagerFactory(services) {
  return new SessionManager(services.persistence);
}

/**
 * Create Periodic Scheduler
 */
export function createSchedulerFactory(config) {
  const scheduler = new PeriodicScheduler({
    onError: config.onSchedulerError || ((task, error) => {
      log.error('Scheduler task failed', { task: task.name, error: error.message });
    }),
  });
  log.debug('Scheduler ready');
  return scheduler;
}

/**
 * Create Metrics Service
 */
export function createMetricsFactory(services) {
  const metrics = new MetricsService({
    persistence: services.persistence,
    sessionManager: services.sessionManager,
    pojChainManager: services.pojChainManager,
    librarian: services.librarian,
    ecosystem: services.ecosystem,
    integrator: services.integrator,
    judge: services.judge,
    collective: services.collective,
  });
  log.debug('Metrics ready');
  return metrics;
}

/**
 * Create Learning Manager
 */
export async function createLearningManagerFactory(services) {
  const eventBus = getEventBus();

  const learningManager = new LearningManager({
    persistence: services.persistence,
    eventBus,
    learningRate: 0.236,  // φ⁻³
    autoLearn: true,
    minSamples: 5,
  });

  await learningManager.init();
  log.info('Learning manager ready', { autoLearn: true });
  return learningManager;
}

/**
 * Create Automation Executor
 */
export async function createAutomationExecutorFactory(services) {
  const eventBus = getEventBus();

  const triggerManager = services.persistence?.triggers ? {
    getEnabled: async () => services.persistence.triggers.findEnabled(),
  } : null;

  const automationExecutor = createAutomationExecutor({
    learningManager: services.learningManager,
    triggerManager,
    pool: services.persistence?.postgres,
    eventBus,
    goalsRepo: services.persistence?.goals,
    tasksRepo: services.persistence?.tasks,
    notificationsRepo: services.persistence?.notifications,
    // Fix 1: Wire dimension governance to automation
    residualGovernance: services.residualGovernance || null,
    collectivePack: services.collectivePack || null,
  });

  await automationExecutor.start();
  log.info('Automation executor started', {
    learningInterval: '5min',
    triggerInterval: '1min',
    cleanupInterval: '13min',
    tasksInterval: '2min',
    goalsInterval: '8min',
    notificationsInterval: '21min',
    governanceInterval: '34min',
  });

  return automationExecutor;
}
