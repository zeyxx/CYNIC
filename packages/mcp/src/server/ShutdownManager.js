/**
 * ShutdownManager - Graceful Multi-Component Shutdown
 *
 * Orchestrates orderly teardown of 15+ components with
 * consistent error handling via _safeStop pattern.
 *
 * "Le chien dort" - κυνικός
 *
 * @module @cynic/mcp/server/ShutdownManager
 */

'use strict';

import { saveCollectiveState } from '@cynic/node';

const SHUTDOWN_TIMEOUT_MS = 10000;

export class ShutdownManager {
  /**
   * @param {Object} options
   * @param {Object} options.server - MCPServer instance (dependency container)
   */
  constructor({ server }) {
    this._server = server;
  }

  /**
   * Perform graceful shutdown of all components
   */
  async shutdown() {
    const s = this._server;

    console.error('CYNIC MCP Server shutting down...');

    // Stop HTTP adapter (handles SSE clients and graceful shutdown)
    await this._safeStop('HTTP', async () => {
      if (s._httpAdapter) {
        await s._httpAdapter.stop(SHUTDOWN_TIMEOUT_MS);
      }
    });

    // Flush PoJ chain (create final block from pending judgments)
    await this._safeStop('PoJ', async () => {
      if (s.pojChainManager) await s.pojChainManager.close();
    });

    // Stop periodic scheduler
    await this._safeStop('Scheduler', () => {
      if (s.scheduler) {
        s.scheduler.stopAll();
        console.error('   Scheduler stopped');
      }
    });

    // Stop discovery health checks
    await this._safeStop('Discovery', async () => {
      if (s.discovery) await s.discovery.shutdown();
    });

    // Stop automation executor
    await this._safeStop('AutomationExecutor', async () => {
      if (s.automationExecutor) {
        await s.automationExecutor.stop();
        console.error('   Automation executor stopped');
      }
    });

    // Stop autonomous daemon
    await this._safeStop('AutonomousDaemon', async () => {
      if (s.autonomousDaemon) {
        await s.autonomousDaemon.stop();
        console.error('   Autonomous daemon stopped');
      }
    });

    // Stop observability stack
    await this._safeStop('HeartbeatService', () => {
      if (s.heartbeatService) {
        const status = s.heartbeatService.getStatus();
        s.heartbeatService.stop();
        console.error(`   HeartbeatService stopped (uptime: ${(status.metrics.systemUptime * 100).toFixed(2)}%)`);
      }
    });

    await this._safeStop('SLATracker', () => {
      if (s.slaTracker) {
        const report = s.slaTracker.getReport();
        console.error(`   SLATracker stopped (compliance: ${report.compliance}%, violations: ${report.totalViolations})`);
      }
    });

    // Stop emergence detector
    await this._safeStop('EmergenceDetector', () => {
      if (s.emergenceDetector) {
        const stats = s.emergenceDetector.getStats();
        s.emergenceDetector.stop();
        console.error(`   EmergenceDetector stopped (patterns: ${stats.patternCount}, facts: ${stats.factsStored})`);
      }
    });

    // Stop X proxy service
    await this._safeStop('XProxy', async () => {
      if (s.xProxy) {
        const stats = s.xProxy.getStats();
        await s.xProxy.stop();
        console.error(`   X Proxy stopped (captured ${stats.tweetsCaptured} tweets, ${stats.usersCaptured} users)`);
      }
    });

    // Close local privacy stores (SQLite)
    await this._safeStop('LocalXStore', () => {
      if (s.localXStore) {
        const stats = s.localXStore.getStats();
        s.localXStore.close();
        console.error(`   LocalXStore closed (${stats.tweets} tweets, ${stats.users} users local)`);
      }
    });

    await this._safeStop('LocalPrivacyStore', () => {
      if (s.localPrivacyStore) {
        s.localPrivacyStore.close();
        console.error('   LocalPrivacyStore closed');
      }
    });

    // Flush and stop tracing (before persistence closes)
    await this._safeStop('Tracing', async () => {
      if (s.traceStorage) {
        await s.traceStorage.stop();
        const stats = s.traceStorage.stats;
        console.error(`   Tracing stopped (${stats.spansStored} spans stored, ${stats.flushCount} flushes)`);
      }
    });

    // Save SharedMemory state before closing persistence
    await this._safeStop('CollectiveState', async () => {
      if (s.persistence) {
        await saveCollectiveState(s.persistence);
        console.error('   Collective state saved');
      }
    });

    // Close persistence connections
    await this._safeStop('Persistence', async () => {
      if (s.persistence) await s.persistence.close();
    });

    console.error('CYNIC MCP Server stopped');

    // Only exit process in stdio mode (HTTP mode should stay alive for graceful restart)
    if (s.mode === 'stdio') {
      process.exit(0);
    }
  }

  /**
   * Safely stop a component with error handling
   * @private
   */
  async _safeStop(name, fn) {
    try {
      await fn();
    } catch (e) {
      console.error(`Error stopping ${name}: ${e.message}`);
    }
  }
}
