/**
 * Telemetry Collector
 *
 * Collects usage statistics, frictions, and performance data
 * across the entire CYNIC ecosystem for benchmarking and fine-tuning.
 *
 * "φ mesure tout, φ apprend de tout"
 *
 * Collects:
 * - Usage metrics (calls, latency, errors)
 * - Friction points (failures, retries, timeouts)
 * - LLM performance (tokens, latency, quality)
 * - User patterns (session length, actions)
 * - System health (memory, CPU, connections)
 *
 * @module @cynic/persistence/services/telemetry-collector
 */

'use strict';

import { EventEmitter } from 'events';

// φ constants for thresholds
const PHI_INV = 0.618033988749895;
const FIB = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144];

/**
 * Metric types
 */
export const MetricType = {
  COUNTER: 'counter',     // Cumulative count
  GAUGE: 'gauge',         // Point-in-time value
  HISTOGRAM: 'histogram', // Distribution
  TIMING: 'timing',       // Latency
  FRICTION: 'friction',   // Error/failure
};

/**
 * Friction severity
 */
export const FrictionSeverity = {
  LOW: 'low',           // Minor inconvenience
  MEDIUM: 'medium',     // Noticeable issue
  HIGH: 'high',         // Significant problem
  CRITICAL: 'critical', // System failure
};

/**
 * Telemetry categories
 */
export const Category = {
  LLM: 'llm',
  JUDGMENT: 'judgment',
  MEMORY: 'memory',
  TOOL: 'tool',
  SESSION: 'session',
  SYSTEM: 'system',
  USER: 'user',
  PATTERN: 'pattern',
  CONSENSUS: 'consensus',
  HOOK: 'hook',
};

/**
 * Histogram bucket boundaries (φ-aligned, in ms)
 */
const LATENCY_BUCKETS = [10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];

/**
 * Telemetry Collector
 */
export class TelemetryCollector extends EventEmitter {
  constructor(options = {}) {
    super();

    this.name = options.name || 'cynic';
    this.flushInterval = options.flushInterval || 60000; // 1 minute
    this.maxBufferSize = options.maxBufferSize || 10000;
    this.persist = options.persist !== false;
    this.pool = options.pool || null;

    // Metrics storage
    this.counters = new Map();
    this.gauges = new Map();
    this.histograms = new Map();
    this.timings = new Map();
    this.frictions = [];

    // Session tracking
    this.sessionId = options.sessionId || this._generateSessionId();
    this.sessionStart = Date.now();
    this.actionCount = 0;

    // Aggregated stats
    this.stats = {
      totalEvents: 0,
      totalErrors: 0,
      totalLatencyMs: 0,
      categories: {},
    };

    // Active timers (for timing operations)
    this.activeTimers = new Map();

    // Flush timer
    this._flushTimer = null;
    if (this.flushInterval > 0) {
      this._flushTimer = setInterval(() => this.flush(), this.flushInterval);
    }
  }

  _generateSessionId() {
    return `sess_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
  }

  /**
   * Increment a counter
   */
  increment(name, value = 1, labels = {}) {
    const key = this._makeKey(name, labels);
    const current = this.counters.get(key) || { value: 0, labels };
    current.value += value;
    this.counters.set(key, current);

    this.stats.totalEvents++;
    this._trackCategory(labels.category);

    this.emit('metric', { type: MetricType.COUNTER, name, value, labels });
  }

  /**
   * Set a gauge value
   */
  gauge(name, value, labels = {}) {
    const key = this._makeKey(name, labels);
    this.gauges.set(key, { value, labels, timestamp: Date.now() });

    this.emit('metric', { type: MetricType.GAUGE, name, value, labels });
  }

  /**
   * Record a histogram value
   */
  histogram(name, value, labels = {}) {
    const key = this._makeKey(name, labels);
    let hist = this.histograms.get(key);

    if (!hist) {
      hist = {
        labels,
        count: 0,
        sum: 0,
        min: Infinity,
        max: -Infinity,
        buckets: new Array(LATENCY_BUCKETS.length + 1).fill(0),
      };
      this.histograms.set(key, hist);
    }

    hist.count++;
    hist.sum += value;
    hist.min = Math.min(hist.min, value);
    hist.max = Math.max(hist.max, value);

    // Update buckets
    for (let i = 0; i < LATENCY_BUCKETS.length; i++) {
      if (value <= LATENCY_BUCKETS[i]) {
        hist.buckets[i]++;
        break;
      }
    }
    if (value > LATENCY_BUCKETS[LATENCY_BUCKETS.length - 1]) {
      hist.buckets[LATENCY_BUCKETS.length]++;
    }

    this.emit('metric', { type: MetricType.HISTOGRAM, name, value, labels });
  }

  /**
   * Start a timer
   */
  startTimer(name, labels = {}) {
    const id = `${name}_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
    this.activeTimers.set(id, {
      name,
      labels,
      start: process.hrtime.bigint(),
    });
    return id;
  }

  /**
   * End a timer and record the duration
   */
  endTimer(timerId, extraLabels = {}) {
    const timer = this.activeTimers.get(timerId);
    if (!timer) return null;

    const end = process.hrtime.bigint();
    const durationNs = Number(end - timer.start);
    const durationMs = durationNs / 1_000_000;

    this.activeTimers.delete(timerId);

    const labels = { ...timer.labels, ...extraLabels };
    this.timing(timer.name, durationMs, labels);

    return durationMs;
  }

  /**
   * Record a timing value
   */
  timing(name, durationMs, labels = {}) {
    const key = this._makeKey(name, labels);
    let timing = this.timings.get(key);

    if (!timing) {
      timing = {
        labels,
        count: 0,
        sum: 0,
        min: Infinity,
        max: -Infinity,
        p50: 0,
        p95: 0,
        p99: 0,
        samples: [],
      };
      this.timings.set(key, timing);
    }

    timing.count++;
    timing.sum += durationMs;
    timing.min = Math.min(timing.min, durationMs);
    timing.max = Math.max(timing.max, durationMs);

    // Keep last 1000 samples for percentile calculation
    timing.samples.push(durationMs);
    if (timing.samples.length > 1000) {
      timing.samples.shift();
    }

    // Update percentiles periodically
    if (timing.count % 100 === 0) {
      this._updatePercentiles(timing);
    }

    this.stats.totalLatencyMs += durationMs;

    this.emit('metric', { type: MetricType.TIMING, name, value: durationMs, labels });

    // Also add to histogram
    this.histogram(`${name}_histogram`, durationMs, labels);
  }

  _updatePercentiles(timing) {
    const sorted = [...timing.samples].sort((a, b) => a - b);
    const len = sorted.length;
    timing.p50 = sorted[Math.floor(len * 0.50)] || 0;
    timing.p95 = sorted[Math.floor(len * 0.95)] || 0;
    timing.p99 = sorted[Math.floor(len * 0.99)] || 0;
  }

  /**
   * Record a friction point (error, failure, timeout)
   */
  friction(name, severity, details = {}) {
    const friction = {
      name,
      severity,
      details,
      timestamp: Date.now(),
      sessionId: this.sessionId,
      category: details.category || Category.SYSTEM,
    };

    this.frictions.push(friction);
    this.stats.totalErrors++;

    // Keep buffer bounded
    if (this.frictions.length > this.maxBufferSize) {
      this.frictions.shift();
    }

    this.increment('friction_total', 1, { severity, category: details.category });

    this.emit('friction', friction);

    return friction;
  }

  /**
   * Record LLM call metrics
   */
  recordLLMCall(data) {
    const {
      model,
      inputTokens = 0,
      outputTokens = 0,
      latencyMs,
      success = true,
      cached = false,
      error = null,
    } = data;

    const labels = { model, category: Category.LLM };

    this.increment('llm_calls_total', 1, labels);
    this.increment('llm_input_tokens_total', inputTokens, labels);
    this.increment('llm_output_tokens_total', outputTokens, labels);

    if (latencyMs) {
      this.timing('llm_latency', latencyMs, labels);
    }

    if (cached) {
      this.increment('llm_cache_hits', 1, labels);
    }

    if (!success) {
      this.friction('llm_error', FrictionSeverity.MEDIUM, {
        category: Category.LLM,
        model,
        error: error?.message || 'Unknown error',
      });
    }

    this.emit('llm_call', data);
  }

  /**
   * Record judgment metrics
   */
  recordJudgment(data) {
    const {
      verdict,
      qScore,
      latencyMs,
      dimensions = {},
      confidence = 0.5,
    } = data;

    const labels = { verdict, category: Category.JUDGMENT };

    this.increment('judgments_total', 1, labels);
    this.histogram('judgment_qscore', qScore, labels);
    this.histogram('judgment_confidence', confidence * 100, labels);

    if (latencyMs) {
      this.timing('judgment_latency', latencyMs, labels);
    }

    // Track dimension scores
    for (const [dim, score] of Object.entries(dimensions)) {
      this.histogram(`judgment_dimension_${dim}`, score, labels);
    }

    this.emit('judgment', data);
  }

  /**
   * Record tool usage
   */
  recordToolUse(data) {
    const {
      tool,
      success = true,
      latencyMs,
      error = null,
    } = data;

    const labels = { tool, category: Category.TOOL };

    this.increment('tool_calls_total', 1, labels);

    if (latencyMs) {
      this.timing('tool_latency', latencyMs, labels);
    }

    if (!success) {
      this.friction('tool_error', FrictionSeverity.MEDIUM, {
        category: Category.TOOL,
        tool,
        error: error?.message || 'Unknown error',
      });
    }

    this.emit('tool_use', data);
  }

  /**
   * Record session event
   */
  recordSessionEvent(event, data = {}) {
    this.actionCount++;

    const labels = { event, category: Category.SESSION };
    this.increment('session_events_total', 1, labels);

    if (event === 'end') {
      const duration = Date.now() - this.sessionStart;
      this.timing('session_duration', duration, { category: Category.SESSION });
      this.gauge('session_actions', this.actionCount, { category: Category.SESSION });
    }

    this.emit('session_event', { event, data, sessionId: this.sessionId });
  }

  /**
   * Record pattern detection
   */
  recordPattern(data) {
    const { type, significance, occurrences } = data;

    const labels = { type, category: Category.PATTERN };
    this.increment('patterns_detected_total', 1, labels);
    this.histogram('pattern_significance', significance * 100, labels);
    this.histogram('pattern_occurrences', occurrences, labels);

    this.emit('pattern', data);
  }

  /**
   * Record system metrics
   */
  recordSystemMetrics() {
    const mem = process.memoryUsage();

    this.gauge('system_memory_heap_used', mem.heapUsed / 1024 / 1024, { category: Category.SYSTEM });
    this.gauge('system_memory_heap_total', mem.heapTotal / 1024 / 1024, { category: Category.SYSTEM });
    this.gauge('system_memory_rss', mem.rss / 1024 / 1024, { category: Category.SYSTEM });

    if (typeof process.cpuUsage === 'function') {
      const cpu = process.cpuUsage();
      this.gauge('system_cpu_user', cpu.user / 1000, { category: Category.SYSTEM });
      this.gauge('system_cpu_system', cpu.system / 1000, { category: Category.SYSTEM });
    }

    this.emit('system_metrics', { memory: mem });
  }

  _makeKey(name, labels) {
    const labelStr = Object.entries(labels)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${k}=${v}`)
      .join(',');
    return labelStr ? `${name}{${labelStr}}` : name;
  }

  _trackCategory(category) {
    if (!category) return;
    this.stats.categories[category] = (this.stats.categories[category] || 0) + 1;
  }

  /**
   * Get current stats summary
   */
  getStats() {
    return {
      sessionId: this.sessionId,
      uptime: Date.now() - this.sessionStart,
      actionCount: this.actionCount,
      ...this.stats,
      counters: this.counters.size,
      gauges: this.gauges.size,
      histograms: this.histograms.size,
      timings: this.timings.size,
      frictions: this.frictions.length,
    };
  }

  /**
   * Get all metrics as exportable object
   */
  export() {
    return {
      timestamp: Date.now(),
      sessionId: this.sessionId,
      stats: this.getStats(),
      counters: Object.fromEntries(this.counters),
      gauges: Object.fromEntries(this.gauges),
      histograms: Object.fromEntries(
        [...this.histograms].map(([k, v]) => [k, {
          ...v,
          avg: v.count > 0 ? v.sum / v.count : 0,
        }])
      ),
      timings: Object.fromEntries(
        [...this.timings].map(([k, v]) => {
          this._updatePercentiles(v);
          return [k, {
            labels: v.labels,
            count: v.count,
            sum: v.sum,
            min: v.min === Infinity ? 0 : v.min,
            max: v.max === -Infinity ? 0 : v.max,
            avg: v.count > 0 ? v.sum / v.count : 0,
            p50: v.p50,
            p95: v.p95,
            p99: v.p99,
          }];
        })
      ),
      frictions: this.frictions.slice(-100), // Last 100 frictions
    };
  }

  /**
   * Flush metrics to storage
   */
  async flush() {
    if (!this.persist || !this.pool) return;

    try {
      const data = this.export();

      await this.pool.query(`
        INSERT INTO telemetry_snapshots (session_id, data, created_at)
        VALUES ($1, $2, NOW())
      `, [this.sessionId, JSON.stringify(data)]);

      this.emit('flush', data);
    } catch (e) {
      // Ignore flush errors
    }
  }

  /**
   * Reset all metrics
   */
  reset() {
    this.counters.clear();
    this.gauges.clear();
    this.histograms.clear();
    this.timings.clear();
    this.frictions = [];
    this.stats = {
      totalEvents: 0,
      totalErrors: 0,
      totalLatencyMs: 0,
      categories: {},
    };
  }

  /**
   * Stop collector
   */
  stop() {
    if (this._flushTimer) {
      clearInterval(this._flushTimer);
      this._flushTimer = null;
    }
    this.flush();
  }
}

/**
 * Global telemetry instance
 */
let _globalTelemetry = null;

export function getTelemetry(options = {}) {
  if (!_globalTelemetry) {
    _globalTelemetry = new TelemetryCollector(options);
  }
  return _globalTelemetry;
}

export function createTelemetryCollector(options = {}) {
  return new TelemetryCollector(options);
}

export default TelemetryCollector;
