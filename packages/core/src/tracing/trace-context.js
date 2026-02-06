/**
 * Distributed Tracing Primitives
 *
 * TraceContext, Span, and NoOpSpan for φ-aligned distributed tracing.
 * Traces flow through the event bus, across Dogs, and between nodes.
 *
 * "φ mesure la latence" - kynikos
 *
 * @module @cynic/core/tracing/trace-context
 */

'use strict';

import { randomUUID } from 'crypto';

// ============================================================================
// TRACE CONTEXT
// ============================================================================

/**
 * Immutable trace context propagated across spans and nodes.
 * Each span creates a child context preserving the trace lineage.
 */
export class TraceContext {
  /**
   * @param {Object} options
   * @param {string} [options.traceId] - Root trace identifier
   * @param {string} [options.spanId] - Current span identifier
   * @param {string|null} [options.parentSpanId] - Parent span identifier
   * @param {Object} [options.baggage] - Key-value pairs propagated across boundaries
   * @param {boolean} [options.sampled] - Whether this trace is sampled
   */
  constructor(options = {}) {
    this.traceId = options.traceId || randomUUID();
    this.spanId = options.spanId || randomUUID();
    this.parentSpanId = options.parentSpanId || null;
    this.baggage = Object.freeze({ ...(options.baggage || {}) });
    this.sampled = options.sampled !== false; // default sampled
  }

  /**
   * Create a child context for a new span within the same trace.
   * Inherits traceId, baggage, and sampling decision.
   *
   * @param {Object} [extraBaggage] - Additional baggage to merge
   * @returns {TraceContext}
   */
  child(extraBaggage = {}) {
    return new TraceContext({
      traceId: this.traceId,
      spanId: randomUUID(),
      parentSpanId: this.spanId,
      baggage: { ...this.baggage, ...extraBaggage },
      sampled: this.sampled,
    });
  }

  /**
   * Serialize for cross-node propagation (P2P messages).
   * @returns {Object}
   */
  toJSON() {
    return {
      traceId: this.traceId,
      spanId: this.spanId,
      parentSpanId: this.parentSpanId,
      baggage: { ...this.baggage },
      sampled: this.sampled,
    };
  }

  /**
   * Reconstruct from serialized form.
   * @param {Object} json
   * @returns {TraceContext}
   */
  static fromJSON(json) {
    if (!json || !json.traceId) return null;
    return new TraceContext({
      traceId: json.traceId,
      spanId: json.spanId,
      parentSpanId: json.parentSpanId,
      baggage: json.baggage,
      sampled: json.sampled,
    });
  }
}

// ============================================================================
// SPAN
// ============================================================================

/** Span status values */
export const SpanStatus = {
  UNSET: 'unset',
  OK: 'ok',
  ERROR: 'error',
};

/**
 * A timed operation within a trace.
 * Captures name, duration, attributes, events, and errors.
 */
export class Span {
  /**
   * @param {string} name - Operation name (e.g. 'dog:Guardian:process')
   * @param {TraceContext} context - Trace context for this span
   * @param {Object} [attributes] - Initial key-value attributes
   */
  constructor(name, context, attributes = {}) {
    this.name = name;
    this.context = context;
    this.startTime = Date.now();
    this.endTime = null;
    this.duration = null;
    this.attributes = { ...attributes };
    this.events = [];
    this.status = SpanStatus.UNSET;
    this.error = null;
  }

  /**
   * Set a key-value attribute on this span.
   * @param {string} key
   * @param {*} value
   * @returns {Span} this (for chaining)
   */
  setAttribute(key, value) {
    this.attributes[key] = value;
    return this;
  }

  /**
   * Record a timestamped event within this span.
   * @param {string} name - Event name
   * @param {Object} [attributes] - Event attributes
   * @returns {Span} this
   */
  addEvent(name, attributes = {}) {
    this.events.push({
      name,
      timestamp: Date.now(),
      attributes,
    });
    return this;
  }

  /**
   * Mark this span as errored.
   * @param {Error|string} error
   * @returns {Span} this
   */
  setError(error) {
    this.status = SpanStatus.ERROR;
    this.error = error instanceof Error ? error.message : String(error);
    return this;
  }

  /**
   * End this span, computing duration.
   * @returns {Span} this
   */
  end() {
    if (this.endTime !== null) return this; // already ended
    this.endTime = Date.now();
    this.duration = this.endTime - this.startTime;
    if (this.status === SpanStatus.UNSET) {
      this.status = SpanStatus.OK;
    }
    return this;
  }

  /** @returns {boolean} */
  get ended() {
    return this.endTime !== null;
  }

  /**
   * Serialize for storage/transmission.
   * @returns {Object}
   */
  toJSON() {
    return {
      name: this.name,
      traceId: this.context.traceId,
      spanId: this.context.spanId,
      parentSpanId: this.context.parentSpanId,
      startTime: this.startTime,
      endTime: this.endTime,
      duration: this.duration,
      attributes: { ...this.attributes },
      events: [...this.events],
      status: this.status,
      error: this.error,
    };
  }
}

// ============================================================================
// NO-OP SPAN
// ============================================================================

/**
 * Lightweight no-op span for unsampled traces.
 * All methods are safe to call but do nothing.
 */
export class NoOpSpan {
  constructor(name, context) {
    this.name = name;
    this.context = context;
    this.startTime = Date.now();
    this.endTime = null;
    this.duration = null;
    this.attributes = {};
    this.events = [];
    this.status = SpanStatus.OK;
    this.error = null;
  }

  setAttribute() { return this; }
  addEvent() { return this; }
  setError() { return this; }

  end() {
    this.endTime = Date.now();
    this.duration = this.endTime - this.startTime;
    return this;
  }

  get ended() { return this.endTime !== null; }

  toJSON() {
    return { name: this.name, noop: true };
  }
}
