/**
 * CYNIC Boot Manager
 *
 * Orchestrates the boot sequence for all CYNIC components.
 * Handles dependency resolution, initialization order, and graceful shutdown.
 *
 * "The pack awakens as one" - κυνικός
 *
 * @module @cynic/core/boot/boot-manager
 */

'use strict';

import { EventEmitter } from 'node:events';
import { CYNICError, ErrorCode } from '../errors.js';
import { createLogger } from '../logger.js';

const log = createLogger('BootManager');
import { LifecycleState, HealthStatus } from './lifecycle.js';

/**
 * Boot manager states
 */
export const BootState = {
  IDLE: 'idle',
  BOOTING: 'booting',
  RUNNING: 'running',
  SHUTTING_DOWN: 'shutting_down',
  SHUTDOWN: 'shutdown',
  FAILED: 'failed',
};

/**
 * Boot manager events
 */
export const BootEvent = {
  COMPONENT_REGISTERED: 'component:registered',
  COMPONENT_INITIALIZING: 'component:initializing',
  COMPONENT_INITIALIZED: 'component:initialized',
  COMPONENT_STARTING: 'component:starting',
  COMPONENT_STARTED: 'component:started',
  COMPONENT_STOPPING: 'component:stopping',
  COMPONENT_STOPPED: 'component:stopped',
  COMPONENT_FAILED: 'component:failed',
  BOOT_STARTED: 'boot:started',
  BOOT_COMPLETED: 'boot:completed',
  BOOT_FAILED: 'boot:failed',
  SHUTDOWN_STARTED: 'shutdown:started',
  SHUTDOWN_COMPLETED: 'shutdown:completed',
  HEALTH_CHECK: 'health:check',
};

/**
 * CYNIC Boot Manager
 *
 * Manages lifecycle of all CYNIC components with:
 * - Auto-discovery via registration
 * - Dependency resolution and ordering
 * - Coordinated startup/shutdown
 * - Health aggregation
 */
export class BootManager extends EventEmitter {
  /**
   * Registered components
   * @type {Map<string, Lifecycle>}
   */
  #components = new Map();

  /**
   * Resolved boot order
   * @type {string[]}
   */
  #bootOrder = [];

  /**
   * Current boot state
   * @type {string}
   */
  #state = BootState.IDLE;

  /**
   * Boot start time
   * @type {number|null}
   */
  #bootStartedAt = null;

  /**
   * Boot completion time
   * @type {number|null}
   */
  #bootCompletedAt = null;

  constructor(options = {}) {
    super();
    const { handleSignals = false } = options;
    if (handleSignals) {
      this.#setupSignalHandlers();
    }
  }

  /**
   * Get current state
   */
  get state() {
    return this.#state;
  }

  /**
   * Get uptime in milliseconds
   */
  get uptime() {
    if (!this.#bootCompletedAt) return 0;
    return Date.now() - this.#bootCompletedAt;
  }

  /**
   * Register a component for lifecycle management
   *
   * @param {Lifecycle} component - Component implementing Lifecycle interface
   * @returns {BootManager} this (for chaining)
   */
  register(component) {
    if (!component.name) {
      throw new CYNICError(
        'Component must have a name',
        ErrorCode.INVALID_INPUT
      );
    }

    if (this.#components.has(component.name)) {
      throw new CYNICError(
        `Component "${component.name}" already registered`,
        ErrorCode.DUPLICATE
      );
    }

    if (this.#state !== BootState.IDLE) {
      throw new CYNICError(
        'Cannot register components after boot started',
        ErrorCode.INVALID_STATE
      );
    }

    this.#components.set(component.name, component);
    this.emit(BootEvent.COMPONENT_REGISTERED, { name: component.name });

    return this;
  }

  /**
   * Register multiple components
   *
   * @param {Lifecycle[]} components
   * @returns {BootManager}
   */
  registerAll(components) {
    for (const component of components) {
      this.register(component);
    }
    return this;
  }

  /**
   * Check if a component is registered
   */
  has(name) {
    return this.#components.has(name);
  }

  /**
   * Get a registered component
   */
  get(name) {
    return this.#components.get(name);
  }

  /**
   * Get all component names
   */
  getComponentNames() {
    return Array.from(this.#components.keys());
  }

  /**
   * Boot all components in dependency order
   *
   * @returns {Promise<Object>} Boot result
   */
  async boot() {
    if (this.#state !== BootState.IDLE) {
      throw new CYNICError(
        `Cannot boot: already ${this.#state}`,
        ErrorCode.INVALID_STATE
      );
    }

    this.#state = BootState.BOOTING;
    this.#bootStartedAt = Date.now();
    this.emit(BootEvent.BOOT_STARTED);

    try {
      // Resolve dependency order
      this.#bootOrder = this.#resolveDependencyOrder();

      // Phase 1: Initialize all components
      for (const name of this.#bootOrder) {
        const component = this.#components.get(name);
        this.emit(BootEvent.COMPONENT_INITIALIZING, { name });

        try {
          await component.initialize();
          this.emit(BootEvent.COMPONENT_INITIALIZED, { name });
        } catch (error) {
          this.emit(BootEvent.COMPONENT_FAILED, { name, error, phase: 'initialize' });
          throw new CYNICError(
            `Failed to initialize ${name}: ${error.message}`,
            ErrorCode.INITIALIZATION_FAILED,
            { cause: error }
          );
        }
      }

      // Phase 2: Start all components
      for (const name of this.#bootOrder) {
        const component = this.#components.get(name);
        this.emit(BootEvent.COMPONENT_STARTING, { name });

        try {
          await component.start();
          this.emit(BootEvent.COMPONENT_STARTED, { name });
        } catch (error) {
          this.emit(BootEvent.COMPONENT_FAILED, { name, error, phase: 'start' });
          throw new CYNICError(
            `Failed to start ${name}: ${error.message}`,
            ErrorCode.INITIALIZATION_FAILED,
            { cause: error }
          );
        }
      }

      this.#state = BootState.RUNNING;
      this.#bootCompletedAt = Date.now();
      const duration = this.#bootCompletedAt - this.#bootStartedAt;

      const result = {
        success: true,
        duration,
        components: this.#bootOrder,
        startedAt: new Date(this.#bootCompletedAt).toISOString(),
      };

      this.emit(BootEvent.BOOT_COMPLETED, result);
      return result;

    } catch (error) {
      this.#state = BootState.FAILED;
      this.emit(BootEvent.BOOT_FAILED, { error });

      // Try to stop any components that started
      await this.#emergencyShutdown();

      throw error;
    }
  }

  /**
   * Gracefully shutdown all components in reverse order
   *
   * @returns {Promise<Object>} Shutdown result
   */
  async shutdown() {
    if (this.#state !== BootState.RUNNING &&
        this.#state !== BootState.FAILED) {
      // Nothing to shutdown
      return { success: true, components: [] };
    }

    this.#state = BootState.SHUTTING_DOWN;
    this.emit(BootEvent.SHUTDOWN_STARTED);

    const stopped = [];
    const errors = [];

    // Shutdown in reverse order
    const shutdownOrder = [...this.#bootOrder].reverse();

    for (const name of shutdownOrder) {
      const component = this.#components.get(name);
      this.emit(BootEvent.COMPONENT_STOPPING, { name });

      try {
        await component.stop();
        stopped.push(name);
        this.emit(BootEvent.COMPONENT_STOPPED, { name });
      } catch (error) {
        errors.push({ name, error: error.message });
        // Continue shutting down other components
      }
    }

    this.#state = BootState.SHUTDOWN;

    const result = {
      success: errors.length === 0,
      stopped,
      errors,
    };

    this.emit(BootEvent.SHUTDOWN_COMPLETED, result);
    return result;
  }

  /**
   * Get aggregated health status for all components
   *
   * @returns {Promise<Object>}
   */
  async health() {
    const components = {};
    let overallStatus = HealthStatus.HEALTHY;
    let unhealthyCount = 0;
    let degradedCount = 0;

    for (const [name, component] of this.#components) {
      const health = await component.health();
      components[name] = health;

      if (health.status === HealthStatus.UNHEALTHY) {
        unhealthyCount++;
        overallStatus = HealthStatus.UNHEALTHY;
      } else if (health.status === HealthStatus.DEGRADED) {
        degradedCount++;
        if (overallStatus !== HealthStatus.UNHEALTHY) {
          overallStatus = HealthStatus.DEGRADED;
        }
      }
    }

    const result = {
      status: overallStatus,
      state: this.#state,
      uptime: this.uptime,
      components,
      summary: {
        total: this.#components.size,
        healthy: this.#components.size - unhealthyCount - degradedCount,
        degraded: degradedCount,
        unhealthy: unhealthyCount,
      },
    };

    this.emit(BootEvent.HEALTH_CHECK, result);
    return result;
  }

  /**
   * Resolve component dependency order using topological sort
   *
   * @returns {string[]} Components in boot order
   * @private
   */
  #resolveDependencyOrder() {
    const visited = new Set();
    const visiting = new Set();
    const order = [];

    const visit = (name) => {
      if (visited.has(name)) return;

      if (visiting.has(name)) {
        throw new CYNICError(
          `Circular dependency detected involving: ${name}`,
          ErrorCode.CIRCULAR_DEPENDENCY
        );
      }

      const component = this.#components.get(name);
      if (!component) {
        throw new CYNICError(
          `Unknown dependency: ${name}`,
          ErrorCode.NOT_FOUND
        );
      }

      visiting.add(name);

      // Visit dependencies first
      for (const dep of component.dependencies) {
        if (!this.#components.has(dep)) {
          throw new CYNICError(
            `Component "${name}" depends on unknown component "${dep}"`,
            ErrorCode.DEPENDENCY_MISSING
          );
        }
        visit(dep);
      }

      visiting.delete(name);
      visited.add(name);
      order.push(name);
    };

    // Visit all components
    for (const name of this.#components.keys()) {
      visit(name);
    }

    return order;
  }

  /**
   * Emergency shutdown for failed boot
   * @private
   */
  async #emergencyShutdown() {
    for (const [_name, component] of this.#components) {
      if (component.state === LifecycleState.RUNNING ||
          component.state === LifecycleState.INITIALIZED) {
        try {
          await component.stop();
        } catch {
          // Ignore errors during emergency shutdown
        }
      }
    }
  }

  /**
   * Setup signal handlers for graceful shutdown
   * @private
   */
  #setupSignalHandlers() {
    const gracefulShutdown = async (signal) => {
      log.info('Received signal, initiating graceful shutdown', { signal });
      try {
        await this.shutdown();
        process.exit(0);
      } catch (error) {
        log.error('Error during shutdown', { error: error.message });
        process.exit(1);
      }
    };

    process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));
    process.on('SIGINT', () => gracefulShutdown('SIGINT'));
  }
}

/**
 * Global boot manager instance
 * Signal handlers enabled for production use
 */
export const globalBootManager = new BootManager({ handleSignals: true });
