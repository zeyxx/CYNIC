/**
 * CYNIC TUI Dashboard - Data Fetcher
 *
 * Polls MCP API for real-time data
 * φ-aligned polling interval: 1618ms
 *
 * @module @cynic/node/cli/dashboard/data-fetcher
 */

'use strict';

import { PHI } from '@cynic/core';

// φ-aligned poll interval (1.618 seconds)
const PHI_POLL_INTERVAL = Math.round(PHI * 1000);

/**
 * DataFetcher - Fetches data from MCP server
 */
export class DataFetcher {
  constructor(options = {}) {
    this.baseUrl = options.baseUrl || 'http://localhost:3618';
    this.pollInterval = options.pollInterval || PHI_POLL_INTERVAL;
    this.intervalId = null;
    this.cache = {};
    this.connected = false;
    this.onError = options.onError || (() => {});
    this.onConnect = options.onConnect || (() => {});
    this.onDisconnect = options.onDisconnect || (() => {});
  }

  /**
   * Make API request to MCP server
   */
  async callTool(toolName, args = {}) {
    try {
      const res = await fetch(`${this.baseUrl}/api/tools/${toolName}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args),
      });

      if (!res.ok) {
        const error = await res.json().catch(() => ({ error: 'Unknown error' }));
        return { success: false, error: error.error || 'Request failed' };
      }

      const data = await res.json();
      return { success: true, result: data.result };
    } catch (err) {
      return { success: false, error: err.message };
    }
  }

  /**
   * Check if MCP server is reachable
   */
  async checkHealth() {
    try {
      const res = await fetch(`${this.baseUrl}/health`, {
        signal: AbortSignal.timeout(3000),
      });
      if (res.ok) {
        if (!this.connected) {
          this.connected = true;
          this.onConnect();
        }
        return await res.json();
      }
      return null;
    } catch {
      if (this.connected) {
        this.connected = false;
        this.onDisconnect();
      }
      return null;
    }
  }

  /**
   * Fetch all dashboard data
   */
  async fetchAll() {
    const [health, collective, patterns, chain, metrics] = await Promise.all([
      this.callTool('brain_health', { verbose: true }),
      this.callTool('brain_collective_status', { verbose: false }),
      this.callTool('brain_patterns', { category: 'all', limit: 5 }),
      this.callTool('brain_poj_chain', { action: 'status' }),
      this.callTool('brain_metrics', { action: 'collect' }),
    ]);

    const data = {
      health: health.success ? health.result : this.getFallbackHealth(),
      collective: collective.success ? collective.result : this.getFallbackCollective(),
      patterns: patterns.success ? patterns.result : this.getFallbackPatterns(),
      chain: chain.success ? chain.result : this.getFallbackChain(),
      metrics: metrics.success ? metrics.result : this.getFallbackMetrics(),
      timestamp: Date.now(),
      connected: this.connected,
    };

    this.cache = data;
    return data;
  }

  /**
   * Start polling for data
   */
  startPolling(callback) {
    // Initial fetch
    this.fetchAll().then(callback).catch((err) => this.onError(err));

    // Set up interval
    this.intervalId = setInterval(async () => {
      try {
        // Check health first
        const healthCheck = await this.checkHealth();
        if (!healthCheck) {
          callback({
            ...this.cache,
            connected: false,
            timestamp: Date.now(),
          });
          return;
        }

        const data = await this.fetchAll();
        callback(data);
      } catch (err) {
        this.onError(err);
      }
    }, this.pollInterval);
  }

  /**
   * Stop polling
   */
  stopPolling() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  /**
   * Get cached data
   */
  getCache() {
    return this.cache;
  }

  // ═══════════════════════════════════════════════════════════
  // FALLBACK DATA (when server unavailable)
  // ═══════════════════════════════════════════════════════════

  getFallbackHealth() {
    return {
      status: 'disconnected',
      identity: { name: 'CYNIC', greek: 'κυνικός' },
      phi: { maxConfidence: 0.618 },
      node: { status: 'unknown', uptime: 0 },
      persistence: { status: 'unknown' },
      timestamp: Date.now(),
    };
  }

  getFallbackCollective() {
    return {
      status: 'unavailable',
      dogCount: 11,
      dogs: {
        guardian: { sefira: 'Gevurah', active: false },
        analyst: { sefira: 'Binah', active: false },
        scholar: { sefira: 'Daat', active: false },
        architect: { sefira: 'Chesed', active: false },
        sage: { sefira: 'Chochmah', active: false },
        cynic: { sefira: 'Keter', active: false },
        janitor: { sefira: 'Yesod', active: false },
        scout: { sefira: 'Netzach', active: false },
        cartographer: { sefira: 'Malkhut', active: false },
        oracle: { sefira: 'Tiferet', active: false },
        deployer: { sefira: 'Hod', active: false },
      },
      timestamp: Date.now(),
    };
  }

  getFallbackPatterns() {
    return {
      patterns: [],
      total: 0,
      timestamp: Date.now(),
    };
  }

  getFallbackChain() {
    return {
      initialized: false,
      headSlot: 0,
      pendingJudgments: 0,
      totalBlocks: 0,
      timestamp: Date.now(),
    };
  }

  getFallbackMetrics() {
    return {
      metrics: {},
      alerts: [],
      timestamp: Date.now(),
    };
  }
}

export default DataFetcher;
