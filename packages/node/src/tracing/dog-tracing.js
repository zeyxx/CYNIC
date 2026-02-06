/**
 * Dog Execution Tracing
 *
 * Non-invasive wrapping of Dog.process() with trace spans.
 * Captures execution time, dog name, sefirah, and error status.
 *
 * Usage:
 *   traceDogExecution(guardianDog, tracer);
 *   // guardianDog.process() now auto-creates spans
 *
 * @module @cynic/node/tracing/dog-tracing
 */

'use strict';

/**
 * Wrap a Dog's process() method with tracing spans.
 * Non-invasive: wraps existing method, no signature changes.
 *
 * @param {import('../agents/base.js').BaseAgent} dog - The dog to trace
 * @param {import('@cynic/core').Tracer} tracer - The tracer instance
 * @returns {import('../agents/base.js').BaseAgent} The same dog (mutated)
 */
export function traceDogExecution(dog, tracer) {
  if (!dog || !tracer) return dog;
  if (dog._traced) return dog; // already wrapped

  const originalProcess = dog.process.bind(dog);

  dog.process = async function tracedProcess(event, context = {}) {
    // Extract parent trace context from event metadata if available
    const parentContext = event?.metadata?.traceContext || context?.traceContext;

    let span;
    if (parentContext) {
      const { TraceContext } = await import('@cynic/core');
      const parent = TraceContext.fromJSON(parentContext);
      span = parent
        ? tracer.startChildSpan(`dog:${dog.name}:process`, parent, {
            'dog.name': dog.name,
            'dog.sefirah': dog.sefirah || 'unknown',
            'dog.trigger': dog.trigger || 'unknown',
          })
        : tracer.startSpan(`dog:${dog.name}:process`, {
            'dog.name': dog.name,
            'dog.sefirah': dog.sefirah || 'unknown',
          });
    } else {
      span = tracer.startSpan(`dog:${dog.name}:process`, {
        'dog.name': dog.name,
        'dog.sefirah': dog.sefirah || 'unknown',
      });
    }

    try {
      const result = await originalProcess(event, context);

      if (result) {
        span.setAttribute('dog.response', result.response || 'unknown');
        if (result.blocked) span.setAttribute('dog.blocked', true);
        if (result.confidence !== undefined) {
          span.setAttribute('dog.confidence', result.confidence);
        }
      }

      tracer.endSpan(span);
      return result;
    } catch (error) {
      span.setError(error);
      tracer.endSpan(span);
      throw error;
    }
  };

  dog._traced = true;
  return dog;
}

/**
 * Wrap all dogs in a CollectivePack with tracing.
 *
 * @param {Map|Object} dogs - Map of dog instances or pack.agents
 * @param {import('@cynic/core').Tracer} tracer
 */
export function traceAllDogs(dogs, tracer) {
  if (!dogs || !tracer) return;

  const iterable = dogs instanceof Map ? dogs.values() : Object.values(dogs);
  for (const dog of iterable) {
    if (dog && typeof dog.process === 'function') {
      traceDogExecution(dog, tracer);
    }
  }
}
