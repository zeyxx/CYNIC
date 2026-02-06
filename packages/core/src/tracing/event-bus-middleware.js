/**
 * Event Bus Tracing Middleware
 *
 * Auto-creates spans for events flowing through globalEventBus.
 * Injects trace context into event.metadata.traceContext.
 *
 * Usage:
 *   import { globalEventBus } from '@cynic/core';
 *   import { createTracingMiddleware } from '@cynic/core/tracing';
 *   globalEventBus.use(createTracingMiddleware(tracer));
 *
 * @module @cynic/core/tracing/event-bus-middleware
 */

'use strict';

import { TraceContext } from './trace-context.js';

/**
 * Create tracing middleware for the event bus.
 *
 * The middleware:
 * 1. Extracts existing traceContext from event.metadata (if present)
 * 2. Creates a child span from it, or a new root span
 * 3. Injects the span's context back into event.metadata.traceContext
 * 4. Auto-ends the span (event processing is synchronous in the bus)
 *
 * @param {import('./tracer.js').Tracer} tracer
 * @returns {Function} Middleware function for bus.use()
 */
export function createTracingMiddleware(tracer) {
  return (event) => {
    if (!event || !event.type) return;

    // Extract existing context or create root
    const existingContext = event.metadata?.traceContext
      ? TraceContext.fromJSON(event.metadata.traceContext)
      : null;

    let span;
    if (existingContext) {
      span = tracer.startChildSpan(`event:${event.type}`, existingContext, {
        'event.type': event.type,
        'event.id': event.id,
        'event.source': event.source,
      });
    } else {
      span = tracer.startSpan(`event:${event.type}`, {
        'event.type': event.type,
        'event.id': event.id,
        'event.source': event.source,
      });
    }

    // Use correlationId as baggage if present
    if (event.correlationId) {
      span.setAttribute('event.correlationId', event.correlationId);
    }

    // Inject trace context back into event metadata
    if (!event.metadata) event.metadata = {};
    event.metadata.traceContext = span.context.toJSON();

    // End span immediately (bus middleware is synchronous)
    tracer.endSpan(span);

    // Don't block the event
    return undefined;
  };
}
