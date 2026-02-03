/**
 * JSON Stream Exporter
 *
 * Server-Sent Events (SSE) stream for real-time metrics.
 * "Le chien observe en temps réel"
 *
 * @module @cynic/observatory/exporters/json-stream
 */

'use strict';

import { EventEmitter } from 'events';

/**
 * JSON Stream exporter using Server-Sent Events
 */
export class JSONStreamExporter extends EventEmitter {
  /**
   * @param {Object} pool - PostgreSQL pool
   */
  constructor(pool) {
    super();
    this.pool = pool;
    this.clients = new Set();
    this._interval = null;
  }

  /**
   * Add an SSE client connection
   * @param {http.ServerResponse} res - Response to stream to
   */
  addClient(res) {
    // Set SSE headers
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
      'Access-Control-Allow-Origin': '*',
    });

    // Send initial connection event
    res.write('event: connected\n');
    res.write(`data: ${JSON.stringify({ timestamp: new Date().toISOString() })}\n\n`);

    this.clients.add(res);

    // Remove on close
    res.on('close', () => {
      this.clients.delete(res);
    });

    // Start streaming if first client
    if (this.clients.size === 1 && !this._interval) {
      this._startStreaming();
    }
  }

  /**
   * Broadcast an event to all clients
   * @param {string} event - Event name
   * @param {Object} data - Event data
   */
  broadcast(event, data) {
    const message = `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
    for (const client of this.clients) {
      try {
        client.write(message);
      } catch (e) {
        this.clients.delete(client);
      }
    }
  }

  /**
   * Start periodic metric streaming
   * @private
   */
  _startStreaming() {
    // Stream metrics every 5 seconds
    this._interval = setInterval(async () => {
      if (this.clients.size === 0) {
        clearInterval(this._interval);
        this._interval = null;
        return;
      }

      try {
        const metrics = await this._fetchMetrics();
        this.broadcast('metrics', metrics);
      } catch (e) {
        this.broadcast('error', { message: e.message });
      }
    }, 5000);
  }

  /**
   * Fetch current metrics
   * @private
   */
  async _fetchMetrics() {
    const [qstats, health, patterns] = await Promise.all([
      this._fetchQStats(),
      this._fetchHealth(),
      this._fetchPatterns(),
    ]);

    return {
      qlearning: qstats,
      health,
      patterns,
      timestamp: new Date().toISOString(),
    };
  }

  async _fetchQStats() {
    try {
      const result = await this.pool.query(`
        SELECT
          (SELECT count(*) FROM qlearning_episodes) as total_episodes,
          (SELECT avg(reward) FROM qlearning_episodes WHERE created_at > NOW() - INTERVAL '1 hour') as avg_reward_1h
      `);
      return result.rows[0] || {};
    } catch (e) {
      return { error: e.message };
    }
  }

  async _fetchHealth() {
    try {
      const result = await this.pool.query(`
        SELECT
          count(*) as tool_calls_1h,
          sum(CASE WHEN success THEN 1 ELSE 0 END)::float / NULLIF(count(*), 0) as success_rate
        FROM tool_usage
        WHERE created_at > NOW() - INTERVAL '1 hour'
      `);
      return result.rows[0] || {};
    } catch (e) {
      return { error: e.message };
    }
  }

  async _fetchPatterns() {
    try {
      const result = await this.pool.query(`
        SELECT count(*) as total FROM patterns
      `);
      return result.rows[0] || {};
    } catch (e) {
      return { error: e.message };
    }
  }

  /**
   * Stop streaming
   */
  stop() {
    if (this._interval) {
      clearInterval(this._interval);
      this._interval = null;
    }
    for (const client of this.clients) {
      client.end();
    }
    this.clients.clear();
  }
}
