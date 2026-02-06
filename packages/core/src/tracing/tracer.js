/**
 * Tracer
 *
 * Creates and manages spans within traces. Uses φ-aligned sampling
 * to control trace volume (max sampling rate: 61.8%).
 *
 * @module @cynic/core/tracing/tracer
 */

'use strict';

import { TraceContext, Span, NoOpSpan, SpanStatus } from './trace-context.js';
import { PHI_INV } from '../axioms/constants.js';

// ============================================================================
// φ-ALIGNED SAMPLER
// ============================================================================

/**
 * Create a φ-aligned sampler. Rate is capped at PHI_INV (61.8%).
 *
 * @param {number} [rate=0.1] - Sampling rate (0.0 to 1.0, capped at PHI_INV)
 * @returns {Function} (traceContext?) => boolean
 */
export function createPhiSampler(rate = 0.1) {
  const effectiveRate = Math.min(rate, PHI_INV);
  return (parentContext) => {
    // Inherit parent sampling decision if present
    if (parentContext && parentContext.sampled !== undefined) {
      return parentContext.sampled;
    }
    return Math.random() < effectiveRate;
  };
}

// ============================================================================
// TRACER
// ============================================================================

/**
 * Tracer manages span lifecycle within a service.
 * Creates root and child spans, tracks active spans, and optionally
 * stores completed spans.
 */
export class Tracer {
  /**
   * @param {Object} [options]
   * @param {string} [options.serviceName='cynic'] - Service name for spans
   * @param {Function} [options.sampler] - Sampling function, default 10%
   * @param {Object} [options.storage] - Optional storage backend with storeSpan(span)
   */
  constructor(options = {}) {
    this.serviceName = options.serviceName || 'cynic';
    this._sampler = options.sampler || createPhiSampler(0.1);
    this._storage = options.storage || null;
    this._activeSpans = new Map(); // spanId -> Span
    this._stats = {
      spansCreated: 0,
      spansEnded: 0,
      spansDropped: 0,
    };
  }

  /**
   * Start a new root span (no parent).
   *
   * @param {string} name - Span name
   * @param {Object} [attributes] - Initial attributes
   * @returns {Span|NoOpSpan}
   */
  startSpan(name, attributes = {}) {
    const context = new TraceContext();
    const sampled = this._sampler(null);
    context.sampled = sampled;

    if (!sampled) {
      this._stats.spansDropped++;
      return new NoOpSpan(name, context);
    }

    const span = new Span(name, context, {
      'service.name': this.serviceName,
      ...attributes,
    });

    this._activeSpans.set(context.spanId, span);
    this._stats.spansCreated++;
    return span;
  }

  /**
   * Start a child span from a parent span or context.
   *
   * @param {string} name - Span name
   * @param {Span|TraceContext} parent - Parent span or context
   * @param {Object} [attributes] - Initial attributes
   * @returns {Span|NoOpSpan}
   */
  startChildSpan(name, parent, attributes = {}) {
    const parentContext = parent instanceof Span ? parent.context : parent;

    // Inherit sampling decision
    if (!parentContext.sampled) {
      this._stats.spansDropped++;
      return new NoOpSpan(name, parentContext.child());
    }

    const childContext = parentContext.child();
    const span = new Span(name, childContext, {
      'service.name': this.serviceName,
      ...attributes,
    });

    this._activeSpans.set(childContext.spanId, span);
    this._stats.spansCreated++;
    return span;
  }

  /**
   * End a span and optionally store it.
   *
   * @param {Span|NoOpSpan} span
   */
  endSpan(span) {
    if (!span || span.ended) return;

    span.end();
    this._activeSpans.delete(span.context?.spanId);
    this._stats.spansEnded++;

    // Store if backend available and span is sampled
    if (this._storage && !(span instanceof NoOpSpan)) {
      try {
        this._storage.storeSpan(span);
      } catch (_) {
        // Non-critical: don't let storage errors affect tracing
      }
    }
  }

  /**
   * Get an active span by ID.
   * @param {string} spanId
   * @returns {Span|undefined}
   */
  getActiveSpan(spanId) {
    return this._activeSpans.get(spanId);
  }

  /** @returns {number} Count of active (un-ended) spans */
  get activeSpanCount() {
    return this._activeSpans.size;
  }

  /** @returns {Object} Tracer statistics */
  get stats() {
    return {
      ...this._stats,
      activeSpans: this._activeSpans.size,
    };
  }
}
