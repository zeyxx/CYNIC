/**
 * Cross-Node Trace Propagation
 *
 * Injects/extracts trace context from P2P messages for distributed tracing.
 * Wire into TransportComponent message handling.
 *
 * @module @cynic/node/tracing/trace-propagation
 */

'use strict';

import { TraceContext } from '@cynic/core';

const TRACE_KEY = '_traceContext';

/**
 * Cross-node trace context propagation.
 */
export const TracePropagation = {
  /**
   * Inject trace context into an outgoing P2P message.
   * Adds _traceContext field with serialized context.
   *
   * @param {Object} message - The P2P message to send
   * @param {import('@cynic/core').Span} span - Current span
   * @returns {Object} The message with injected context
   */
  inject(message, span) {
    if (!message || !span?.context) return message;
    message[TRACE_KEY] = span.context.toJSON();
    return message;
  },

  /**
   * Extract trace context from a received P2P message.
   * Creates a child context for the local processing.
   *
   * @param {Object} message - The received P2P message
   * @returns {TraceContext|null} Child context, or null if no context present
   */
  extract(message) {
    if (!message || !message[TRACE_KEY]) return null;
    const parentContext = TraceContext.fromJSON(message[TRACE_KEY]);
    if (!parentContext) return null;
    return parentContext.child();
  },

  /**
   * Check if a message carries trace context.
   * @param {Object} message
   * @returns {boolean}
   */
  hasContext(message) {
    return !!(message && message[TRACE_KEY]);
  },

  /** The key used for trace context in messages */
  TRACE_KEY,
};
