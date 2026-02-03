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
const PHI_INV_2 = PHI_INV * PHI_INV; // 0.382
const FIB = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144];

/**
 * Alert levels for threshold violations
 * Task #62: Add telemetry thresholds and alerts
 */
export const AlertLevel = {
  INFO: 'info',         // Informational
  WARNING: 'warning',   // Approaching threshold
  CRITICAL: 'critical', // Threshold exceeded
};

/**
 * Default φ-derived thresholds
 * Task #62: Add telemetry thresholds and alerts
 */
export const DEFAULT_THRESHOLDS = Object.freeze({
  // Latency thresholds (ms)
  llm_latency_p95: 5000,          // 5s p95 latency warning
  llm_latency_p99: 10000,         // 10s p99 latency critical
  judgment_latency_p95: 1000,     // 1s p95 for judgments
  tool_latency_p95: 2000,         // 2s p95 for tools

  // Error rate thresholds (percentage as decimal)
  error_rate_warning: PHI_INV_2,  // 38.2% errors = warning
  error_rate_critical: PHI_INV,   // 61.8% errors = critical

  // Token thresholds (per session)
  tokens_per_session: 100000,     // 100k tokens warning
  tokens_per_session_critical: 500000, // 500k critical

  // Friction thresholds (count per hour)
  frictions_per_hour_warning: 10,
  frictions_per_hour_critical: 25,

  // Memory thresholds (MB)
  memory_heap_warning: 512,       // 512MB warning
  memory_heap_critical: 1024,     // 1GB critical

  // Session thresholds
  session_duration_warning: 3600000,  // 1 hour
  session_duration_critical: 7200000, // 2 hours

  // Judgment quality thresholds
  judgment_qscore_min: 30,        // Alert if Q-score drops below 30
  judgment_confidence_min: 20,    // Alert if confidence drops below 20%
});

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

    // Task #62: Thresholds and alerts
    this.thresholds = { ...DEFAULT_THRESHOLDS, ...options.thresholds };
    this.alerts = []; // Recent alerts
    this.alertsEnabled = options.alertsEnabled !== false;
    this._alertCooldowns = new Map(); // Prevent alert spam

    // Flush timer
    this._flushTimer = null;
    if (this.flushInterval > 0) {
      this._flushTimer = setInterval(() => this.flush(), this.flushInterval);
    }

    // Task #62: Start threshold monitoring
    this._thresholdTimer = null;
    if (this.alertsEnabled) {
      this._thresholdTimer = setInterval(() => this._checkThresholds(), 30000); // Every 30s
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

    // Persist to PostgreSQL if pool available
    if (this.pool && this.persist) {
      this.pool.query(`
        INSERT INTO frictions (session_id, name, severity, category, details)
        VALUES ($1, $2, $3, $4, $5)
      `, [
        this.sessionId,
        name,
        severity,
        details.category || 'system',
        JSON.stringify(details),
      ]).catch(() => {
        // Silently ignore persistence errors
      });
    }

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

    // Persist to PostgreSQL if pool available
    if (this.pool && this.persist) {
      this.pool.query(`
        INSERT INTO llm_usage (session_id, model, input_tokens, output_tokens, latency_ms, cached, success, error)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      `, [
        this.sessionId,
        model,
        inputTokens,
        outputTokens,
        latencyMs || null,
        cached,
        success,
        error ? (typeof error === 'string' ? error : error.message || String(error)) : null,
      ]).catch(() => {
        // Silently ignore persistence errors
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

    // Persist to PostgreSQL if pool available
    if (this.pool && this.persist) {
      this.pool.query(`
        INSERT INTO tool_usage (session_id, tool_name, success, latency_ms, error)
        VALUES ($1, $2, $3, $4, $5)
      `, [
        this.sessionId,
        tool,
        success,
        latencyMs || null,
        error ? (typeof error === 'string' ? error : error.message || String(error)) : null,
      ]).catch(() => {
        // Silently ignore persistence errors
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
    if (this._thresholdTimer) {
      clearInterval(this._thresholdTimer);
      this._thresholdTimer = null;
    }
    this.flush();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // THRESHOLDS & ALERTS (Task #62)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Set a threshold value
   * @param {string} name - Threshold name
   * @param {number} value - Threshold value
   */
  setThreshold(name, value) {
    this.thresholds[name] = value;
  }

  /**
   * Get current thresholds
   * @returns {Object} Current thresholds
   */
  getThresholds() {
    return { ...this.thresholds };
  }

  /**
   * Trigger an alert
   * @param {string} name - Alert name
   * @param {string} level - Alert level (info, warning, critical)
   * @param {string} message - Alert message
   * @param {Object} [data] - Additional data
   * @private
   */
  _triggerAlert(name, level, message, data = {}) {
    // Check cooldown (5 minutes per alert type)
    const cooldownKey = `${name}:${level}`;
    const lastAlert = this._alertCooldowns.get(cooldownKey) || 0;
    const now = Date.now();

    if (now - lastAlert < 300000) { // 5 minute cooldown
      return null;
    }

    this._alertCooldowns.set(cooldownKey, now);

    const alert = {
      id: `alert_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      name,
      level,
      message,
      data,
      timestamp: now,
      sessionId: this.sessionId,
    };

    this.alerts.push(alert);
    if (this.alerts.length > 100) {
      this.alerts.shift();
    }

    this.emit('alert', alert);

    // Also record as friction if warning or critical
    if (level !== AlertLevel.INFO) {
      this.friction(`threshold_${name}`, level === AlertLevel.CRITICAL ? FrictionSeverity.HIGH : FrictionSeverity.MEDIUM, {
        category: Category.SYSTEM,
        alert: name,
        ...data,
      });
    }

    return alert;
  }

  /**
   * Check all thresholds and trigger alerts as needed
   * @private
   */
  _checkThresholds() {
    if (!this.alertsEnabled) return;

    const checks = [];

    // Check latency thresholds
    for (const [key, timing] of this.timings) {
      if (key.includes('llm_latency')) {
        if (timing.p95 > this.thresholds.llm_latency_p95) {
          checks.push(this._triggerAlert('llm_latency_p95', AlertLevel.WARNING,
            `LLM p95 latency (${Math.round(timing.p95)}ms) exceeds threshold (${this.thresholds.llm_latency_p95}ms)`,
            { current: timing.p95, threshold: this.thresholds.llm_latency_p95 }
          ));
        }
        if (timing.p99 > this.thresholds.llm_latency_p99) {
          checks.push(this._triggerAlert('llm_latency_p99', AlertLevel.CRITICAL,
            `LLM p99 latency (${Math.round(timing.p99)}ms) exceeds critical threshold`,
            { current: timing.p99, threshold: this.thresholds.llm_latency_p99 }
          ));
        }
      }
      if (key.includes('judgment_latency') && timing.p95 > this.thresholds.judgment_latency_p95) {
        checks.push(this._triggerAlert('judgment_latency', AlertLevel.WARNING,
          `Judgment p95 latency (${Math.round(timing.p95)}ms) is high`,
          { current: timing.p95, threshold: this.thresholds.judgment_latency_p95 }
        ));
      }
      if (key.includes('tool_latency') && timing.p95 > this.thresholds.tool_latency_p95) {
        checks.push(this._triggerAlert('tool_latency', AlertLevel.WARNING,
          `Tool p95 latency (${Math.round(timing.p95)}ms) is high`,
          { current: timing.p95, threshold: this.thresholds.tool_latency_p95 }
        ));
      }
    }

    // Check error rate
    if (this.stats.totalEvents > 10) { // Only check after enough events
      const errorRate = this.stats.totalErrors / this.stats.totalEvents;
      if (errorRate > this.thresholds.error_rate_critical) {
        checks.push(this._triggerAlert('error_rate', AlertLevel.CRITICAL,
          `Error rate (${(errorRate * 100).toFixed(1)}%) exceeds critical threshold`,
          { current: errorRate, threshold: this.thresholds.error_rate_critical }
        ));
      } else if (errorRate > this.thresholds.error_rate_warning) {
        checks.push(this._triggerAlert('error_rate', AlertLevel.WARNING,
          `Error rate (${(errorRate * 100).toFixed(1)}%) is elevated`,
          { current: errorRate, threshold: this.thresholds.error_rate_warning }
        ));
      }
    }

    // Check friction rate (per hour)
    const sessionHours = (Date.now() - this.sessionStart) / 3600000;
    if (sessionHours > 0.1) { // Only after 6 minutes
      const frictionsPerHour = this.frictions.length / sessionHours;
      if (frictionsPerHour > this.thresholds.frictions_per_hour_critical) {
        checks.push(this._triggerAlert('friction_rate', AlertLevel.CRITICAL,
          `Friction rate (${Math.round(frictionsPerHour)}/hr) is critical`,
          { current: frictionsPerHour, threshold: this.thresholds.frictions_per_hour_critical }
        ));
      } else if (frictionsPerHour > this.thresholds.frictions_per_hour_warning) {
        checks.push(this._triggerAlert('friction_rate', AlertLevel.WARNING,
          `Friction rate (${Math.round(frictionsPerHour)}/hr) is elevated`,
          { current: frictionsPerHour, threshold: this.thresholds.frictions_per_hour_warning }
        ));
      }
    }

    // Check memory
    const heapGauge = this.gauges.get('system_memory_heap_used{category=system}');
    if (heapGauge) {
      const heapMB = heapGauge.value;
      if (heapMB > this.thresholds.memory_heap_critical) {
        checks.push(this._triggerAlert('memory_heap', AlertLevel.CRITICAL,
          `Heap memory (${Math.round(heapMB)}MB) exceeds critical threshold`,
          { current: heapMB, threshold: this.thresholds.memory_heap_critical }
        ));
      } else if (heapMB > this.thresholds.memory_heap_warning) {
        checks.push(this._triggerAlert('memory_heap', AlertLevel.WARNING,
          `Heap memory (${Math.round(heapMB)}MB) is elevated`,
          { current: heapMB, threshold: this.thresholds.memory_heap_warning }
        ));
      }
    }

    // Check session duration
    const sessionDuration = Date.now() - this.sessionStart;
    if (sessionDuration > this.thresholds.session_duration_critical) {
      checks.push(this._triggerAlert('session_duration', AlertLevel.CRITICAL,
        `Session duration (${Math.round(sessionDuration / 60000)} min) is very long`,
        { current: sessionDuration, threshold: this.thresholds.session_duration_critical }
      ));
    } else if (sessionDuration > this.thresholds.session_duration_warning) {
      checks.push(this._triggerAlert('session_duration', AlertLevel.WARNING,
        `Session duration (${Math.round(sessionDuration / 60000)} min) is getting long`,
        { current: sessionDuration, threshold: this.thresholds.session_duration_warning }
      ));
    }

    // Check token usage
    const inputTokens = this.counters.get('llm_input_tokens_total{model=*,category=llm}');
    const outputTokens = this.counters.get('llm_output_tokens_total{model=*,category=llm}');
    let totalTokens = 0;
    for (const [key, counter] of this.counters) {
      if (key.includes('tokens_total')) {
        totalTokens += counter.value;
      }
    }
    if (totalTokens > this.thresholds.tokens_per_session_critical) {
      checks.push(this._triggerAlert('token_usage', AlertLevel.CRITICAL,
        `Token usage (${totalTokens.toLocaleString()}) is critical`,
        { current: totalTokens, threshold: this.thresholds.tokens_per_session_critical }
      ));
    } else if (totalTokens > this.thresholds.tokens_per_session) {
      checks.push(this._triggerAlert('token_usage', AlertLevel.WARNING,
        `Token usage (${totalTokens.toLocaleString()}) is high`,
        { current: totalTokens, threshold: this.thresholds.tokens_per_session }
      ));
    }

    return checks.filter(Boolean);
  }

  /**
   * Get recent alerts
   * @param {Object} [options]
   * @param {string} [options.level] - Filter by level
   * @param {number} [options.limit=10] - Max alerts to return
   * @returns {Object[]} Recent alerts
   */
  getAlerts(options = {}) {
    let alerts = [...this.alerts];

    if (options.level) {
      alerts = alerts.filter(a => a.level === options.level);
    }

    const limit = options.limit || 10;
    return alerts.slice(-limit).reverse();
  }

  /**
   * Get current health status based on thresholds
   * @returns {Object} Health status
   */
  getHealth() {
    const checks = this._checkThresholds() || [];
    const criticalAlerts = checks.filter(a => a?.level === AlertLevel.CRITICAL);
    const warningAlerts = checks.filter(a => a?.level === AlertLevel.WARNING);

    let status = 'healthy';
    if (criticalAlerts.length > 0) {
      status = 'critical';
    } else if (warningAlerts.length > 0) {
      status = 'degraded';
    }

    return {
      status,
      healthy: status === 'healthy',
      criticalCount: criticalAlerts.length,
      warningCount: warningAlerts.length,
      alerts: checks.filter(Boolean),
      checkedAt: Date.now(),
    };
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
