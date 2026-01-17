/**
 * @cynic/node - Collective Deployer Agent
 *
 * "Je fais passer du code au réel. L'infra est mon territoire.
 *  Ce qui est pensé, je le manifeste." - κυνικός Deployer
 *
 * Hod (הוד) - Splendor/Glory - The 8th Sefirah
 *
 * Deployer transforms code into running infrastructure:
 * - Deployment orchestration and pipelines
 * - Infrastructure management (Docker, K8s, Render)
 * - Health monitoring and rollbacks
 * - CI/CD integration
 *
 * φ-aligned: φ⁻² (38.2%) activation + Guardian approval required
 *
 * @module @cynic/node/agents/collective/deployer
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { BaseAgent, AgentTrigger, AgentBehavior, AgentResponse } from '../base.js';
import {
  AgentId,
  DeployStartedEvent,
  DeployCompletedEvent,
  DeployFailedEvent,
  RollbackInitiatedEvent,
  HealthCheckEvent,
} from '../events.js';
import { ProfileLevel } from '../../profile/calculator.js';

/**
 * φ-aligned constants for Deployer
 * All bounds based on Fibonacci numbers
 */
export const DEPLOYER_CONSTANTS = {
  /** Max deployment history (Fib(13) = 233) */
  MAX_DEPLOYMENT_HISTORY: 233,

  /** Health check interval (Fib(8) = 21 seconds) */
  HEALTH_CHECK_INTERVAL_MS: 21000,

  /** Rollback window (Fib(5) = 5 deployments) */
  ROLLBACK_WINDOW: 5,

  /** Max concurrent deploys (Fib(3) = 2) */
  MAX_CONCURRENT_DEPLOYS: 2,

  /** Deploy timeout (Fib(13) = 233 seconds) */
  DEPLOY_TIMEOUT_MS: 233000,

  /** Build timeout (Fib(10) = 55 seconds) */
  BUILD_TIMEOUT_MS: 55000,

  /** Verification retries (Fib(5) = 5) */
  VERIFICATION_RETRIES: 5,

  /** Retry delay (Fib(5) = 5 seconds) */
  RETRY_DELAY_MS: 5000,

  /** Min confidence for deploy (φ⁻²) */
  DEPLOY_CONFIDENCE_THRESHOLD: PHI_INV_2,
};

/**
 * Deployment state machine
 */
export const DeploymentState = {
  PENDING: 'pending',
  BUILDING: 'building',
  DEPLOYING: 'deploying',
  VERIFYING: 'verifying',
  LIVE: 'live',
  FAILED: 'failed',
  ROLLED_BACK: 'rolled_back',
  CANCELLED: 'cancelled',
};

/**
 * Deployment target platforms
 */
export const DeployTarget = {
  RENDER: 'render',
  DOCKER: 'docker',
  LOCAL: 'local',
  KUBERNETES: 'kubernetes',
  GITHUB_ACTIONS: 'github_actions',
};

/**
 * Health check status
 */
export const HealthStatus = {
  HEALTHY: 'healthy',
  DEGRADED: 'degraded',
  UNHEALTHY: 'unhealthy',
  UNKNOWN: 'unknown',
};

/**
 * CollectiveDeployer - The Hod Sefirah Dog
 *
 * Transforms code into running infrastructure with φ-aligned safety.
 */
export class CollectiveDeployer extends BaseAgent {
  /**
   * Create a Deployer agent
   * @param {Object} options - Configuration options
   */
  constructor(options = {}) {
    super({
      name: 'Deployer',
      trigger: AgentTrigger.ON_DEMAND,
      behavior: AgentBehavior.BLOCKING,
      ...options,
    });

    // Sefirah identity
    this.sefirah = 'Hod';
    this.sefirahMeaning = 'Splendor/Glory';

    // Deployment history
    this.deployments = [];
    this.currentDeploy = null;
    this.concurrentDeploys = 0;

    // Health tracking
    this.healthChecks = new Map();
    this.healthHistory = new Map();

    // Statistics
    this.stats = {
      totalDeploys: 0,
      successfulDeploys: 0,
      failedDeploys: 0,
      rollbacks: 0,
      healthChecks: 0,
      avgDeployTime: 0,
    };

    // Profile-based settings
    this.profileLevel = options.profileLevel || ProfileLevel.PRACTITIONER;
    this._applyProfileSettings();

    // Event bus reference (set by CollectivePack)
    this.eventBus = options.eventBus || null;

    // Guardian reference for approval (set by CollectivePack)
    this.guardian = options.guardian || null;

    // Service configs
    this.serviceConfigs = options.serviceConfigs || new Map();
  }

  /**
   * Apply profile-based settings
   * @private
   */
  _applyProfileSettings() {
    const levelSettings = {
      [ProfileLevel.NOVICE]: {
        requireManualApproval: true,
        autoRollback: false,
        maxConcurrent: 1,
        verificationRetries: 3,
      },
      [ProfileLevel.APPRENTICE]: {
        requireManualApproval: true,
        autoRollback: true,
        maxConcurrent: 1,
        verificationRetries: 3,
      },
      [ProfileLevel.PRACTITIONER]: {
        requireManualApproval: false,
        autoRollback: true,
        maxConcurrent: 2,
        verificationRetries: 5,
      },
      [ProfileLevel.EXPERT]: {
        requireManualApproval: false,
        autoRollback: true,
        maxConcurrent: 2,
        verificationRetries: 5,
      },
      [ProfileLevel.MASTER]: {
        requireManualApproval: false,
        autoRollback: true,
        maxConcurrent: 2,
        verificationRetries: 8,
      },
    };

    const settings = levelSettings[this.profileLevel] || levelSettings[ProfileLevel.PRACTITIONER];
    this.settings = settings;
  }

  /**
   * Update profile level
   * @param {number} level - New profile level
   */
  setProfileLevel(level) {
    this.profileLevel = level;
    this._applyProfileSettings();
  }

  /**
   * Check if agent should trigger for event
   * @param {Object} event - Event to check
   * @returns {boolean} True if should trigger
   */
  shouldTrigger(event) {
    // On-demand only - explicit deploy requests
    if (event.type === 'DEPLOY_REQUEST' || event.action === 'deploy') {
      return true;
    }

    // Health check requests
    if (event.type === 'HEALTH_CHECK_REQUEST' || event.action === 'health_check') {
      return true;
    }

    // Rollback requests
    if (event.type === 'ROLLBACK_REQUEST' || event.action === 'rollback') {
      return true;
    }

    return false;
  }

  /**
   * Process deployment event
   * @param {Object} event - Event to process
   * @param {Object} context - Processing context
   * @returns {Promise<Object>} Processing result
   */
  async process(event, context = {}) {
    switch (event.action || event.type) {
      case 'deploy':
      case 'DEPLOY_REQUEST':
        return this.deploy(event.options || event);

      case 'rollback':
      case 'ROLLBACK_REQUEST':
        return this.rollback(event.options || {});

      case 'health_check':
      case 'HEALTH_CHECK_REQUEST':
        return this.checkHealth(event.service || event.options?.service);

      default:
        return {
          success: false,
          response: AgentResponse.PASS,
          message: `Unknown action: ${event.action || event.type}`,
        };
    }
  }

  /**
   * Deploy to target
   * @param {Object} options - Deployment options
   * @returns {Promise<Object>} Deployment result
   */
  async deploy(options = {}) {
    const { target, service, version, config, isRollback = false } = options;

    // Check concurrent deploy limit
    if (this.concurrentDeploys >= this.settings.maxConcurrent) {
      return {
        success: false,
        reason: `Max concurrent deploys (${this.settings.maxConcurrent}) reached`,
        blocked: true,
        response: AgentResponse.BLOCK,
      };
    }

    // Request Guardian approval
    const guardianApproval = await this._requestGuardianApproval({
      action: 'deploy',
      target,
      service,
      version,
      isRollback,
    });

    if (!guardianApproval.approved) {
      return {
        success: false,
        reason: guardianApproval.reason,
        blocked: true,
        response: AgentResponse.BLOCK,
      };
    }

    // Create deployment record
    const deployment = {
      id: `dep_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      target: target || DeployTarget.LOCAL,
      service: service || 'unknown',
      version: version || 'latest',
      config: config || {},
      state: DeploymentState.PENDING,
      isRollback,
      startedAt: Date.now(),
      completedAt: null,
      duration: null,
      error: null,
      healthChecks: [],
      logs: [],
    };

    // Track deployment
    this.deployments.unshift(deployment);
    this._trimHistory();
    this.currentDeploy = deployment;
    this.concurrentDeploys++;
    this.stats.totalDeploys++;

    // Emit deploy started event
    this._emitEvent(new DeployStartedEvent({
      id: deployment.id,
      target: deployment.target,
      environment: deployment.config?.environment || 'production',
      version: deployment.version,
      initiatedBy: deployment.config?.initiatedBy || 'system',
      guardianApproved: true,
    }));

    try {
      // Build phase
      deployment.state = DeploymentState.BUILDING;
      deployment.logs.push({ phase: 'build', started: Date.now() });
      await this._build(deployment);
      deployment.logs[deployment.logs.length - 1].completed = Date.now();

      // Deploy phase
      deployment.state = DeploymentState.DEPLOYING;
      deployment.logs.push({ phase: 'deploy', started: Date.now() });
      await this._deploy(deployment);
      deployment.logs[deployment.logs.length - 1].completed = Date.now();

      // Verify phase
      deployment.state = DeploymentState.VERIFYING;
      deployment.logs.push({ phase: 'verify', started: Date.now() });
      const healthy = await this._verify(deployment);
      deployment.logs[deployment.logs.length - 1].completed = Date.now();

      if (healthy) {
        deployment.state = DeploymentState.LIVE;
        deployment.completedAt = Date.now();
        deployment.duration = deployment.completedAt - deployment.startedAt;
        this.stats.successfulDeploys++;
        this._updateAvgDeployTime(deployment.duration);

        // Emit success event
        this._emitEvent(new DeployCompletedEvent({
          id: deployment.id,
          target: deployment.target,
          environment: deployment.config?.environment || 'production',
          version: deployment.version,
          duration: deployment.duration,
          healthChecks: deployment.healthChecks,
        }));

        return {
          success: true,
          deployment,
          response: AgentResponse.ALLOW,
        };
      } else {
        throw new Error('Health verification failed');
      }
    } catch (error) {
      deployment.state = DeploymentState.FAILED;
      deployment.error = error.message;
      deployment.completedAt = Date.now();
      deployment.duration = deployment.completedAt - deployment.startedAt;
      this.stats.failedDeploys++;

      // Emit failure event
      this._emitEvent(new DeployFailedEvent({
        id: deployment.id,
        target: deployment.target,
        environment: deployment.config?.environment || 'production',
        error: error.message,
        rollbackInitiated: this.settings.autoRollback && !isRollback && this._canRollback(),
      }));

      // Auto-rollback if enabled and possible
      if (this.settings.autoRollback && !isRollback && this._canRollback()) {
        const rollbackResult = await this.rollback({
          reason: error.message,
          automatic: true,
        });

        return {
          success: false,
          error: error.message,
          deployment,
          rollback: rollbackResult,
          response: AgentResponse.WARN,
        };
      }

      return {
        success: false,
        error: error.message,
        deployment,
        response: AgentResponse.WARN,
      };
    } finally {
      this.concurrentDeploys--;
      if (this.currentDeploy?.id === deployment.id) {
        this.currentDeploy = null;
      }
    }
  }

  /**
   * Rollback to previous version
   * @param {Object} options - Rollback options
   * @returns {Promise<Object>} Rollback result
   */
  async rollback(options = {}) {
    const { reason, automatic = false } = options;

    // Find last good deployment
    const lastGood = this.deployments.find(
      d => d.state === DeploymentState.LIVE && !d.isRollback
    );

    if (!lastGood) {
      return {
        success: false,
        reason: 'No rollback target available',
        response: AgentResponse.WARN,
      };
    }

    // Get current failing deployment
    const current = this.deployments.find(
      d => d.state === DeploymentState.FAILED ||
           d.state === DeploymentState.DEPLOYING
    );

    // Emit rollback event
    this._emitEvent(new RollbackInitiatedEvent({
      fromVersion: current?.version || 'current',
      toVersion: lastGood.version,
      reason: reason || 'Manual rollback',
      automatic,
    }));

    this.stats.rollbacks++;

    // Perform rollback deployment
    const result = await this.deploy({
      target: lastGood.target,
      service: lastGood.service,
      version: lastGood.version,
      config: lastGood.config,
      isRollback: true,
    });

    if (result.success) {
      result.deployment.state = DeploymentState.ROLLED_BACK;
    }

    return {
      ...result,
      automatic,
      rolledBackFrom: current?.version,
      rolledBackTo: lastGood.version,
    };
  }

  /**
   * Check service health
   * @param {string} service - Service name
   * @returns {Promise<Object>} Health check result
   */
  async checkHealth(service) {
    if (!service) {
      // Check all known services
      const results = {};
      for (const [svc, config] of this.serviceConfigs) {
        results[svc] = await this._performHealthCheck(svc, config);
      }
      return {
        overall: this._calculateOverallHealth(results),
        services: results,
        timestamp: Date.now(),
      };
    }

    const config = this.serviceConfigs.get(service) || {};
    const result = await this._performHealthCheck(service, config);

    // Track health history
    if (!this.healthHistory.has(service)) {
      this.healthHistory.set(service, []);
    }
    this.healthHistory.get(service).push({
      ...result,
      timestamp: Date.now(),
    });

    // Trim history
    const history = this.healthHistory.get(service);
    if (history.length > DEPLOYER_CONSTANTS.MAX_DEPLOYMENT_HISTORY) {
      this.healthHistory.set(
        service,
        history.slice(-DEPLOYER_CONSTANTS.MAX_DEPLOYMENT_HISTORY)
      );
    }

    this.stats.healthChecks++;

    // Emit health check event
    this._emitEvent(new HealthCheckEvent({
      service,
      status: result.status,
      healthy: result.healthy,
      checks: result.checks,
    }));

    return result;
  }

  /**
   * Register a service configuration
   * @param {string} service - Service name
   * @param {Object} config - Service configuration
   */
  registerService(service, config) {
    this.serviceConfigs.set(service, {
      endpoint: config.endpoint,
      healthPath: config.healthPath || '/health',
      expectedStatus: config.expectedStatus || 200,
      timeout: config.timeout || 5000,
      ...config,
    });
  }

  /**
   * Get deployment history
   * @param {Object} options - Filter options
   * @returns {Array} Filtered deployments
   */
  getDeploymentHistory(options = {}) {
    let deployments = [...this.deployments];

    if (options.service) {
      deployments = deployments.filter(d => d.service === options.service);
    }

    if (options.state) {
      deployments = deployments.filter(d => d.state === options.state);
    }

    if (options.target) {
      deployments = deployments.filter(d => d.target === options.target);
    }

    if (options.limit) {
      deployments = deployments.slice(0, options.limit);
    }

    return deployments;
  }

  /**
   * Get current deployment
   * @returns {Object|null} Current deployment or null
   */
  getCurrentDeployment() {
    return this.currentDeploy;
  }

  /**
   * Cancel current deployment
   * @returns {Promise<Object>} Cancellation result
   */
  async cancelDeployment() {
    if (!this.currentDeploy) {
      return {
        success: false,
        reason: 'No active deployment',
      };
    }

    if (this.currentDeploy.state === DeploymentState.VERIFYING ||
        this.currentDeploy.state === DeploymentState.LIVE) {
      return {
        success: false,
        reason: 'Cannot cancel deployment in verification or live state',
      };
    }

    this.currentDeploy.state = DeploymentState.CANCELLED;
    this.currentDeploy.completedAt = Date.now();
    this.currentDeploy.error = 'Cancelled by user';

    const cancelled = this.currentDeploy;
    this.currentDeploy = null;
    this.concurrentDeploys--;

    return {
      success: true,
      deployment: cancelled,
    };
  }

  /**
   * Request Guardian approval for deployment
   * @private
   */
  async _requestGuardianApproval(params) {
    // If no guardian available, assume approved with warning
    if (!this.guardian) {
      return {
        approved: true,
        warning: 'No Guardian configured - deploy without safety check',
      };
    }

    try {
      const analysis = await this.guardian.analyze({
        type: 'DEPLOY_REQUEST',
        action: 'deploy',
        ...params,
      }, {});

      return {
        approved: analysis.response !== AgentResponse.BLOCK,
        reason: analysis.message,
        riskLevel: analysis.riskLevel,
      };
    } catch (error) {
      // On error, be cautious - require manual approval
      if (this.settings.requireManualApproval) {
        return {
          approved: false,
          reason: `Guardian check failed: ${error.message}`,
        };
      }
      return {
        approved: true,
        warning: `Guardian check failed: ${error.message}`,
      };
    }
  }

  /**
   * Build deployment
   * @private
   */
  async _build(deployment) {
    // Simulate build process (override in subclasses for real builds)
    const buildConfig = deployment.config.build || {};

    // Basic build simulation
    await this._simulatePhase('build', buildConfig.duration || 100);

    deployment.logs[deployment.logs.length - 1].success = true;
    return true;
  }

  /**
   * Deploy to target
   * @private
   */
  async _deploy(deployment) {
    const target = deployment.target;

    switch (target) {
      case DeployTarget.RENDER:
        return this._deployToRender(deployment);
      case DeployTarget.DOCKER:
        return this._deployToDocker(deployment);
      case DeployTarget.KUBERNETES:
        return this._deployToKubernetes(deployment);
      case DeployTarget.LOCAL:
      default:
        return this._deployToLocal(deployment);
    }
  }

  /**
   * Deploy to Render
   * @private
   */
  async _deployToRender(deployment) {
    // Simulate Render deployment
    await this._simulatePhase('render', 200);
    deployment.logs[deployment.logs.length - 1].success = true;
    deployment.logs[deployment.logs.length - 1].target = 'render';
    return true;
  }

  /**
   * Deploy to Docker
   * @private
   */
  async _deployToDocker(deployment) {
    // Simulate Docker deployment
    await this._simulatePhase('docker', 150);
    deployment.logs[deployment.logs.length - 1].success = true;
    deployment.logs[deployment.logs.length - 1].target = 'docker';
    return true;
  }

  /**
   * Deploy to Kubernetes
   * @private
   */
  async _deployToKubernetes(deployment) {
    // Simulate K8s deployment
    await this._simulatePhase('kubernetes', 300);
    deployment.logs[deployment.logs.length - 1].success = true;
    deployment.logs[deployment.logs.length - 1].target = 'kubernetes';
    return true;
  }

  /**
   * Deploy to local
   * @private
   */
  async _deployToLocal(deployment) {
    // Simulate local deployment
    await this._simulatePhase('local', 50);
    deployment.logs[deployment.logs.length - 1].success = true;
    deployment.logs[deployment.logs.length - 1].target = 'local';
    return true;
  }

  /**
   * Verify deployment health
   * @private
   */
  async _verify(deployment) {
    const retries = this.settings.verificationRetries;

    for (let i = 0; i < retries; i++) {
      const health = await this._performHealthCheck(deployment.service, {
        ...this.serviceConfigs.get(deployment.service),
        timeout: 3000,
      });

      deployment.healthChecks.push(health);

      if (health.status === HealthStatus.HEALTHY) {
        return true;
      }

      // Wait before retry
      if (i < retries - 1) {
        await this._simulatePhase('retry-wait', DEPLOYER_CONSTANTS.RETRY_DELAY_MS);
      }
    }

    return false;
  }

  /**
   * Perform health check on service
   * @private
   */
  async _performHealthCheck(service, config = {}) {
    const checks = [];

    // Endpoint check
    const endpointCheck = {
      name: 'endpoint',
      healthy: true, // Simulated - override for real checks
      responseTime: Math.random() * 100 + 10,
    };
    checks.push(endpointCheck);

    // Resource check
    const resourceCheck = {
      name: 'resources',
      healthy: true,
      cpu: Math.random() * 50 + 10,
      memory: Math.random() * 60 + 20,
    };
    checks.push(resourceCheck);

    // Dependency check
    const depCheck = {
      name: 'dependencies',
      healthy: true,
      dependencies: [],
    };
    checks.push(depCheck);

    // Calculate overall status
    const allHealthy = checks.every(c => c.healthy);
    const someHealthy = checks.some(c => c.healthy);

    let status;
    if (allHealthy) {
      status = HealthStatus.HEALTHY;
    } else if (someHealthy) {
      status = HealthStatus.DEGRADED;
    } else {
      status = HealthStatus.UNHEALTHY;
    }

    return {
      service,
      status,
      healthy: allHealthy,
      checks,
      timestamp: Date.now(),
    };
  }

  /**
   * Calculate overall health from multiple services
   * @private
   */
  _calculateOverallHealth(results) {
    const statuses = Object.values(results).map(r => r.status);

    if (statuses.every(s => s === HealthStatus.HEALTHY)) {
      return HealthStatus.HEALTHY;
    }
    if (statuses.every(s => s === HealthStatus.UNHEALTHY)) {
      return HealthStatus.UNHEALTHY;
    }
    return HealthStatus.DEGRADED;
  }

  /**
   * Check if rollback is possible
   * @private
   */
  _canRollback() {
    const liveDeployments = this.deployments.filter(
      d => d.state === DeploymentState.LIVE && !d.isRollback
    );
    return liveDeployments.length > 0;
  }

  /**
   * Trim deployment history
   * @private
   */
  _trimHistory() {
    if (this.deployments.length > DEPLOYER_CONSTANTS.MAX_DEPLOYMENT_HISTORY) {
      this.deployments = this.deployments.slice(
        0,
        DEPLOYER_CONSTANTS.MAX_DEPLOYMENT_HISTORY
      );
    }
  }

  /**
   * Update average deploy time
   * @private
   */
  _updateAvgDeployTime(duration) {
    const successfulCount = this.stats.successfulDeploys;
    const currentAvg = this.stats.avgDeployTime;

    // Running average
    this.stats.avgDeployTime =
      (currentAvg * (successfulCount - 1) + duration) / successfulCount;
  }

  /**
   * Simulate a deployment phase
   * @private
   */
  async _simulatePhase(name, duration) {
    // In real implementation, this would be actual work
    // For now, just wait (useful for testing)
    await new Promise(resolve => setTimeout(resolve, Math.min(duration, 10)));
  }

  /**
   * Emit event to event bus
   * @private
   */
  _emitEvent(event) {
    if (this.eventBus) {
      this.eventBus.publish(event);
    }
  }

  /**
   * Get agent summary
   * @returns {Object} Summary statistics
   */
  getSummary() {
    const liveDeployments = this.deployments.filter(
      d => d.state === DeploymentState.LIVE
    );

    return {
      name: this.name,
      sefirah: this.sefirah,
      sefirahMeaning: this.sefirahMeaning,
      profileLevel: this.profileLevel,
      stats: {
        ...this.stats,
        successRate: this.stats.totalDeploys > 0
          ? (this.stats.successfulDeploys / this.stats.totalDeploys * 100).toFixed(1) + '%'
          : 'N/A',
      },
      currentDeploy: this.currentDeploy ? {
        id: this.currentDeploy.id,
        service: this.currentDeploy.service,
        state: this.currentDeploy.state,
        started: this.currentDeploy.startedAt,
      } : null,
      liveServices: liveDeployments.map(d => ({
        service: d.service,
        version: d.version,
        deployedAt: d.completedAt,
      })),
      registeredServices: Array.from(this.serviceConfigs.keys()),
      settings: this.settings,
    };
  }

  /**
   * Clear deployment history
   */
  clear() {
    this.deployments = [];
    this.currentDeploy = null;
    this.concurrentDeploys = 0;
    this.healthChecks.clear();
    this.healthHistory.clear();
    this.stats = {
      totalDeploys: 0,
      successfulDeploys: 0,
      failedDeploys: 0,
      rollbacks: 0,
      healthChecks: 0,
      avgDeployTime: 0,
    };
  }
}

export default CollectiveDeployer;
