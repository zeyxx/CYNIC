/**
 * Deep tests for Distributed Tracing
 *
 * Tests:
 * - TraceContext: creation, child(), baggage propagation, serialization
 * - Span: creation, duration, events, status, error, ended flag
 * - NoOpSpan: lightweight no-op behavior
 * - Tracer: root/child spans, sampling, NoOp for unsampled, storage
 * - createPhiSampler: rate capping at PHI_INV, parent inheritance
 * - Event bus middleware: injection, extraction, auto-spans
 * - Dog tracing: execution wrap, timing, error handling
 * - Cross-node propagation: inject/extract, child context
 * - φ-alignment: sampling cap, retention constants
 *
 * Uses node:test (NOT vitest) for CI compatibility.
 * "φ mesure la latence" - kynikos
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import { TraceContext, Span, NoOpSpan, SpanStatus } from '../src/tracing/trace-context.js';
import { Tracer, createPhiSampler } from '../src/tracing/tracer.js';
import { createTracingMiddleware } from '../src/tracing/event-bus-middleware.js';
import { PHI_INV } from '../src/axioms/constants.js';

// ============================================================================
// HELPERS
// ============================================================================

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

const createMockStorage = () => ({
  spans: [],
  storeSpan(span) { this.spans.push(span); },
});

const createMockEvent = (type = 'test:event', opts = {}) => ({
  id: opts.id || 'evt-1',
  type,
  payload: opts.payload || {},
  source: opts.source || 'test',
  timestamp: Date.now(),
  correlationId: opts.correlationId || null,
  metadata: opts.metadata || {},
});

// ============================================================================
// TRACE CONTEXT
// ============================================================================

describe('TraceContext - Deep Tests', () => {
  describe('Creation', () => {
    it('should generate unique traceId and spanId', () => {
      const ctx = new TraceContext();
      assert.ok(ctx.traceId);
      assert.ok(ctx.spanId);
      assert.strictEqual(ctx.parentSpanId, null);
    });

    it('should accept custom traceId and spanId', () => {
      const ctx = new TraceContext({ traceId: 'trace-1', spanId: 'span-1' });
      assert.strictEqual(ctx.traceId, 'trace-1');
      assert.strictEqual(ctx.spanId, 'span-1');
    });

    it('should default sampled to true', () => {
      const ctx = new TraceContext();
      assert.strictEqual(ctx.sampled, true);
    });

    it('should respect sampled=false', () => {
      const ctx = new TraceContext({ sampled: false });
      assert.strictEqual(ctx.sampled, false);
    });

    it('should freeze baggage', () => {
      const ctx = new TraceContext({ baggage: { key: 'val' } });
      assert.strictEqual(ctx.baggage.key, 'val');
      assert.throws(() => { ctx.baggage.newKey = 'fail'; }, TypeError);
    });

    it('should default to empty frozen baggage', () => {
      const ctx = new TraceContext();
      assert.deepStrictEqual(ctx.baggage, {});
      assert.throws(() => { ctx.baggage.x = 1; }, TypeError);
    });

    it('should generate different IDs for different contexts', () => {
      const a = new TraceContext();
      const b = new TraceContext();
      assert.notStrictEqual(a.traceId, b.traceId);
      assert.notStrictEqual(a.spanId, b.spanId);
    });
  });

  describe('child()', () => {
    it('should preserve traceId', () => {
      const parent = new TraceContext({ traceId: 'trace-abc' });
      const child = parent.child();
      assert.strictEqual(child.traceId, 'trace-abc');
    });

    it('should set parentSpanId to parent spanId', () => {
      const parent = new TraceContext();
      const child = parent.child();
      assert.strictEqual(child.parentSpanId, parent.spanId);
    });

    it('should generate new spanId', () => {
      const parent = new TraceContext();
      const child = parent.child();
      assert.notStrictEqual(child.spanId, parent.spanId);
    });

    it('should inherit baggage', () => {
      const parent = new TraceContext({ baggage: { region: 'us-east' } });
      const child = parent.child();
      assert.strictEqual(child.baggage.region, 'us-east');
    });

    it('should merge extra baggage', () => {
      const parent = new TraceContext({ baggage: { a: 1 } });
      const child = parent.child({ b: 2 });
      assert.strictEqual(child.baggage.a, 1);
      assert.strictEqual(child.baggage.b, 2);
    });

    it('should inherit sampling decision', () => {
      const sampled = new TraceContext({ sampled: true });
      assert.strictEqual(sampled.child().sampled, true);

      const unsampled = new TraceContext({ sampled: false });
      assert.strictEqual(unsampled.child().sampled, false);
    });

    it('should support multi-level nesting', () => {
      const root = new TraceContext({ traceId: 'deep-trace' });
      const child1 = root.child();
      const child2 = child1.child();
      const child3 = child2.child();

      assert.strictEqual(child3.traceId, 'deep-trace');
      assert.strictEqual(child3.parentSpanId, child2.spanId);
    });
  });

  describe('Serialization', () => {
    it('should serialize to JSON', () => {
      const ctx = new TraceContext({ traceId: 't1', spanId: 's1', baggage: { k: 'v' } });
      const json = ctx.toJSON();
      assert.strictEqual(json.traceId, 't1');
      assert.strictEqual(json.spanId, 's1');
      assert.strictEqual(json.baggage.k, 'v');
      assert.strictEqual(json.sampled, true);
    });

    it('should reconstruct from JSON', () => {
      const original = new TraceContext({ traceId: 'rt', spanId: 'rs', baggage: { x: 1 } });
      const restored = TraceContext.fromJSON(original.toJSON());
      assert.strictEqual(restored.traceId, 'rt');
      assert.strictEqual(restored.spanId, 'rs');
      assert.strictEqual(restored.baggage.x, 1);
    });

    it('should return null for invalid JSON', () => {
      assert.strictEqual(TraceContext.fromJSON(null), null);
      assert.strictEqual(TraceContext.fromJSON({}), null);
      assert.strictEqual(TraceContext.fromJSON(undefined), null);
    });

    it('should survive JSON.stringify roundtrip', () => {
      const ctx = new TraceContext({ baggage: { deep: 'value' } });
      const roundtripped = TraceContext.fromJSON(JSON.parse(JSON.stringify(ctx.toJSON())));
      assert.strictEqual(roundtripped.traceId, ctx.traceId);
      assert.strictEqual(roundtripped.baggage.deep, 'value');
    });
  });
});

// ============================================================================
// SPAN
// ============================================================================

describe('Span - Deep Tests', () => {
  let ctx;

  beforeEach(() => {
    ctx = new TraceContext({ traceId: 'span-test-trace' });
  });

  describe('Creation', () => {
    it('should store name and context', () => {
      const span = new Span('test-op', ctx);
      assert.strictEqual(span.name, 'test-op');
      assert.strictEqual(span.context, ctx);
    });

    it('should record startTime', () => {
      const before = Date.now();
      const span = new Span('op', ctx);
      assert.ok(span.startTime >= before);
    });

    it('should start with UNSET status', () => {
      const span = new Span('op', ctx);
      assert.strictEqual(span.status, SpanStatus.UNSET);
    });

    it('should start with null endTime and duration', () => {
      const span = new Span('op', ctx);
      assert.strictEqual(span.endTime, null);
      assert.strictEqual(span.duration, null);
    });

    it('should accept initial attributes', () => {
      const span = new Span('op', ctx, { key: 'val' });
      assert.strictEqual(span.attributes.key, 'val');
    });

    it('should start with empty events array', () => {
      const span = new Span('op', ctx);
      assert.deepStrictEqual(span.events, []);
    });

    it('should start with null error', () => {
      const span = new Span('op', ctx);
      assert.strictEqual(span.error, null);
    });
  });

  describe('setAttribute', () => {
    it('should set attribute and return this', () => {
      const span = new Span('op', ctx);
      const result = span.setAttribute('key', 'value');
      assert.strictEqual(result, span);
      assert.strictEqual(span.attributes.key, 'value');
    });

    it('should overwrite existing attributes', () => {
      const span = new Span('op', ctx, { x: 1 });
      span.setAttribute('x', 2);
      assert.strictEqual(span.attributes.x, 2);
    });
  });

  describe('addEvent', () => {
    it('should add timestamped event', () => {
      const span = new Span('op', ctx);
      span.addEvent('checkpoint', { step: 1 });
      assert.strictEqual(span.events.length, 1);
      assert.strictEqual(span.events[0].name, 'checkpoint');
      assert.ok(span.events[0].timestamp);
      assert.strictEqual(span.events[0].attributes.step, 1);
    });

    it('should support multiple events', () => {
      const span = new Span('op', ctx);
      span.addEvent('start').addEvent('middle').addEvent('end');
      assert.strictEqual(span.events.length, 3);
    });
  });

  describe('setError', () => {
    it('should set status to ERROR and store message from Error', () => {
      const span = new Span('op', ctx);
      span.setError(new Error('boom'));
      assert.strictEqual(span.status, SpanStatus.ERROR);
      assert.strictEqual(span.error, 'boom');
    });

    it('should convert string error', () => {
      const span = new Span('op', ctx);
      span.setError('string-error');
      assert.strictEqual(span.error, 'string-error');
      assert.strictEqual(span.status, SpanStatus.ERROR);
    });
  });

  describe('end()', () => {
    it('should compute duration', async () => {
      const span = new Span('op', ctx);
      await wait(10);
      span.end();
      assert.ok(span.duration >= 0);
      assert.ok(span.endTime >= span.startTime);
    });

    it('should set status to OK if UNSET', () => {
      const span = new Span('op', ctx);
      span.end();
      assert.strictEqual(span.status, SpanStatus.OK);
    });

    it('should preserve ERROR status', () => {
      const span = new Span('op', ctx);
      span.setError('fail');
      span.end();
      assert.strictEqual(span.status, SpanStatus.ERROR);
    });

    it('should be idempotent', () => {
      const span = new Span('op', ctx);
      span.end();
      const firstEnd = span.endTime;
      span.end();
      assert.strictEqual(span.endTime, firstEnd);
    });

    it('should set ended flag', () => {
      const span = new Span('op', ctx);
      assert.strictEqual(span.ended, false);
      span.end();
      assert.strictEqual(span.ended, true);
    });
  });

  describe('toJSON', () => {
    it('should serialize all fields', () => {
      const span = new Span('op', ctx, { svc: 'test' });
      span.addEvent('evt');
      span.end();
      const json = span.toJSON();
      assert.strictEqual(json.name, 'op');
      assert.strictEqual(json.traceId, ctx.traceId);
      assert.strictEqual(json.spanId, ctx.spanId);
      assert.strictEqual(json.attributes.svc, 'test');
      assert.strictEqual(json.events.length, 1);
      assert.strictEqual(json.status, SpanStatus.OK);
      assert.ok(json.duration !== null);
    });
  });
});

// ============================================================================
// NO-OP SPAN
// ============================================================================

describe('NoOpSpan - Deep Tests', () => {
  it('should have same interface as Span', () => {
    const noop = new NoOpSpan('noop', new TraceContext());
    assert.strictEqual(typeof noop.setAttribute, 'function');
    assert.strictEqual(typeof noop.addEvent, 'function');
    assert.strictEqual(typeof noop.setError, 'function');
    assert.strictEqual(typeof noop.end, 'function');
  });

  it('should return this from all setters (chainable)', () => {
    const noop = new NoOpSpan('noop', new TraceContext());
    assert.strictEqual(noop.setAttribute('k', 'v'), noop);
    assert.strictEqual(noop.addEvent('e'), noop);
    assert.strictEqual(noop.setError('err'), noop);
  });

  it('should not store attributes', () => {
    const noop = new NoOpSpan('noop', new TraceContext());
    noop.setAttribute('key', 'val');
    assert.strictEqual(noop.attributes.key, undefined);
  });

  it('should not store events', () => {
    const noop = new NoOpSpan('noop', new TraceContext());
    noop.addEvent('evt');
    assert.strictEqual(noop.events.length, 0);
  });

  it('should compute duration on end', () => {
    const noop = new NoOpSpan('noop', new TraceContext());
    noop.end();
    assert.ok(noop.duration !== null);
    assert.strictEqual(noop.ended, true);
  });

  it('should serialize with noop flag', () => {
    const noop = new NoOpSpan('noop', new TraceContext());
    const json = noop.toJSON();
    assert.strictEqual(json.noop, true);
  });
});

// ============================================================================
// createPhiSampler
// ============================================================================

describe('createPhiSampler - Deep Tests', () => {
  it('should cap rate at PHI_INV (61.8%)', () => {
    const sampler = createPhiSampler(1.0);
    // Run many samples: should not always return true
    let trueCount = 0;
    const N = 10000;
    for (let i = 0; i < N; i++) {
      if (sampler(null)) trueCount++;
    }
    // With 61.8% rate, expect roughly 6180 ± 300
    assert.ok(trueCount < N, 'Should not sample everything at rate 1.0');
    assert.ok(trueCount > N * 0.5, `Expected >50% sampled, got ${trueCount / N * 100}%`);
    assert.ok(trueCount < N * 0.72, `Expected <72% sampled, got ${trueCount / N * 100}%`);
  });

  it('should respect low sampling rate', () => {
    const sampler = createPhiSampler(0.01);
    let trueCount = 0;
    for (let i = 0; i < 10000; i++) {
      if (sampler(null)) trueCount++;
    }
    assert.ok(trueCount < 500, `Expected <5% sampled, got ${trueCount}`);
  });

  it('should inherit parent sampling decision (true)', () => {
    const sampler = createPhiSampler(0.0); // 0% rate
    const parent = new TraceContext({ sampled: true });
    assert.strictEqual(sampler(parent), true);
  });

  it('should inherit parent sampling decision (false)', () => {
    const sampler = createPhiSampler(1.0); // 100% rate (capped to 61.8%)
    const parent = new TraceContext({ sampled: false });
    assert.strictEqual(sampler(parent), false);
  });

  it('should default to 10% rate', () => {
    const sampler = createPhiSampler();
    let trueCount = 0;
    for (let i = 0; i < 10000; i++) {
      if (sampler(null)) trueCount++;
    }
    // 10% ± margin
    assert.ok(trueCount > 500 && trueCount < 1500, `Expected ~10%, got ${trueCount / 100}%`);
  });
});

// ============================================================================
// TRACER
// ============================================================================

describe('Tracer - Deep Tests', () => {
  let tracer;
  let storage;

  beforeEach(() => {
    storage = createMockStorage();
    tracer = new Tracer({
      serviceName: 'test-svc',
      sampler: () => true, // always sample for deterministic tests
      storage,
    });
  });

  describe('startSpan (root)', () => {
    it('should create a Span with service name', () => {
      const span = tracer.startSpan('root-op');
      assert.ok(span instanceof Span);
      assert.strictEqual(span.attributes['service.name'], 'test-svc');
    });

    it('should track active spans', () => {
      const span = tracer.startSpan('op');
      assert.strictEqual(tracer.activeSpanCount, 1);
      tracer.endSpan(span);
      assert.strictEqual(tracer.activeSpanCount, 0);
    });

    it('should increment stats.spansCreated', () => {
      tracer.startSpan('op1');
      tracer.startSpan('op2');
      assert.strictEqual(tracer.stats.spansCreated, 2);
    });

    it('should set custom attributes', () => {
      const span = tracer.startSpan('op', { custom: 'attr' });
      assert.strictEqual(span.attributes.custom, 'attr');
    });
  });

  describe('startChildSpan', () => {
    it('should create child from parent Span', () => {
      const parent = tracer.startSpan('parent');
      const child = tracer.startChildSpan('child', parent);
      assert.strictEqual(child.context.traceId, parent.context.traceId);
      assert.strictEqual(child.context.parentSpanId, parent.context.spanId);
    });

    it('should create child from TraceContext', () => {
      const parentCtx = new TraceContext({ traceId: 'ctx-trace' });
      const child = tracer.startChildSpan('child', parentCtx);
      assert.strictEqual(child.context.traceId, 'ctx-trace');
      assert.strictEqual(child.context.parentSpanId, parentCtx.spanId);
    });

    it('should return NoOpSpan for unsampled parent', () => {
      const unsampledCtx = new TraceContext({ sampled: false });
      const child = tracer.startChildSpan('child', unsampledCtx);
      assert.ok(child instanceof NoOpSpan);
    });

    it('should increment spansDropped for unsampled', () => {
      const unsampledCtx = new TraceContext({ sampled: false });
      tracer.startChildSpan('child', unsampledCtx);
      assert.strictEqual(tracer.stats.spansDropped, 1);
    });
  });

  describe('endSpan', () => {
    it('should end the span and store it', () => {
      const span = tracer.startSpan('op');
      tracer.endSpan(span);
      assert.strictEqual(span.ended, true);
      assert.strictEqual(storage.spans.length, 1);
    });

    it('should not store NoOpSpan', () => {
      const noop = new NoOpSpan('noop', new TraceContext());
      tracer.endSpan(noop);
      assert.strictEqual(storage.spans.length, 0);
    });

    it('should handle null span gracefully', () => {
      tracer.endSpan(null); // should not throw
    });

    it('should not double-end', () => {
      const span = tracer.startSpan('op');
      tracer.endSpan(span);
      tracer.endSpan(span);
      assert.strictEqual(storage.spans.length, 1);
      assert.strictEqual(tracer.stats.spansEnded, 1);
    });

    it('should increment spansEnded', () => {
      const span = tracer.startSpan('op');
      tracer.endSpan(span);
      assert.strictEqual(tracer.stats.spansEnded, 1);
    });

    it('should tolerate storage errors', () => {
      const badStorage = { storeSpan() { throw new Error('db down'); } };
      const t = new Tracer({ sampler: createPhiSampler(1.0), storage: badStorage });
      const span = t.startSpan('op');
      t.endSpan(span); // should not throw
    });
  });

  describe('Unsampled traces', () => {
    it('should return NoOpSpan when sampler rejects', () => {
      const neverSample = new Tracer({
        sampler: () => false,
      });
      const span = neverSample.startSpan('op');
      assert.ok(span instanceof NoOpSpan);
    });

    it('should not track NoOpSpan as active', () => {
      const neverSample = new Tracer({ sampler: () => false });
      neverSample.startSpan('op');
      assert.strictEqual(neverSample.activeSpanCount, 0);
    });
  });

  describe('getActiveSpan', () => {
    it('should retrieve active span by id', () => {
      const span = tracer.startSpan('op');
      const found = tracer.getActiveSpan(span.context.spanId);
      assert.strictEqual(found, span);
    });

    it('should return undefined after span ends', () => {
      const span = tracer.startSpan('op');
      tracer.endSpan(span);
      assert.strictEqual(tracer.getActiveSpan(span.context.spanId), undefined);
    });
  });

  describe('stats', () => {
    it('should track all counters', () => {
      tracer.startSpan('op1');
      const span2 = tracer.startSpan('op2');
      tracer.endSpan(span2);

      const stats = tracer.stats;
      assert.strictEqual(stats.spansCreated, 2);
      assert.strictEqual(stats.spansEnded, 1);
      assert.strictEqual(stats.activeSpans, 1);
    });
  });
});

// ============================================================================
// EVENT BUS MIDDLEWARE
// ============================================================================

describe('createTracingMiddleware - Deep Tests', () => {
  let tracer;
  let storage;
  let middleware;

  beforeEach(() => {
    storage = createMockStorage();
    tracer = new Tracer({
      serviceName: 'middleware-test',
      sampler: () => true, // always sample for deterministic tests
      storage,
    });
    middleware = createTracingMiddleware(tracer);
  });

  it('should inject traceContext into event metadata', () => {
    const event = createMockEvent('test:op');
    middleware(event);
    assert.ok(event.metadata.traceContext);
    assert.ok(event.metadata.traceContext.traceId);
    assert.ok(event.metadata.traceContext.spanId);
  });

  it('should create root span for events without trace context', () => {
    const event = createMockEvent('test:op');
    middleware(event);
    assert.strictEqual(storage.spans.length, 1);
    assert.strictEqual(storage.spans[0].name, 'event:test:op');
  });

  it('should create child span for events with existing trace context', () => {
    const parentCtx = new TraceContext({ traceId: 'parent-trace' });
    const event = createMockEvent('test:child', {
      metadata: { traceContext: parentCtx.toJSON() },
    });
    middleware(event);

    assert.strictEqual(storage.spans[0].context.traceId, 'parent-trace');
    assert.ok(storage.spans[0].context.parentSpanId);
  });

  it('should set event attributes on span', () => {
    const event = createMockEvent('judgment:created', { source: 'judge', id: 'evt-42' });
    middleware(event);

    const span = storage.spans[0];
    assert.strictEqual(span.attributes['event.type'], 'judgment:created');
    assert.strictEqual(span.attributes['event.source'], 'judge');
    assert.strictEqual(span.attributes['event.id'], 'evt-42');
  });

  it('should include correlationId as attribute', () => {
    const event = createMockEvent('test:op', { correlationId: 'corr-123' });
    middleware(event);
    assert.strictEqual(storage.spans[0].attributes['event.correlationId'], 'corr-123');
  });

  it('should not block events (return undefined)', () => {
    const event = createMockEvent('test:op');
    const result = middleware(event);
    assert.strictEqual(result, undefined);
  });

  it('should handle null event gracefully', () => {
    middleware(null); // should not throw
  });

  it('should handle event without metadata', () => {
    const event = { type: 'bare:event', id: 'e1', source: 'test' };
    middleware(event);
    assert.ok(event.metadata.traceContext);
  });

  it('should auto-end spans', () => {
    const event = createMockEvent('test:op');
    middleware(event);
    assert.strictEqual(storage.spans[0].ended, true);
    assert.strictEqual(tracer.activeSpanCount, 0);
  });
});

// ============================================================================
// DOG TRACING
// ============================================================================

describe('Dog Tracing - Deep Tests', async () => {
  // Dynamic import since it's in packages/node
  const { traceDogExecution, traceAllDogs } = await import(
    '../../node/src/tracing/dog-tracing.js'
  );

  let tracer;
  let storage;

  beforeEach(() => {
    storage = createMockStorage();
    tracer = new Tracer({
      serviceName: 'dog-test',
      sampler: () => true, // always sample for deterministic tests
      storage,
    });
  });

  const createMockDog = (name = 'Guardian', opts = {}) => ({
    name,
    sefirah: opts.sefirah || 'Gevurah',
    trigger: opts.trigger || 'PRE_TOOL_USE',
    _traced: false,
    async process(event, context) {
      return { response: 'ALLOW', confidence: 0.55 };
    },
  });

  it('should wrap dog.process with tracing', async () => {
    const dog = createMockDog();
    traceDogExecution(dog, tracer);
    assert.strictEqual(dog._traced, true);

    const result = await dog.process({ type: 'test' });
    assert.strictEqual(result.response, 'ALLOW');
    assert.strictEqual(storage.spans.length, 1);
    assert.ok(storage.spans[0].name.includes('Guardian'));
  });

  it('should capture dog name and sefirah', async () => {
    const dog = createMockDog('Oracle', { sefirah: 'Tiferet' });
    traceDogExecution(dog, tracer);
    await dog.process({ type: 'test' });

    const span = storage.spans[0];
    assert.strictEqual(span.attributes['dog.name'], 'Oracle');
    assert.strictEqual(span.attributes['dog.sefirah'], 'Tiferet');
  });

  it('should capture response and confidence', async () => {
    const dog = createMockDog();
    traceDogExecution(dog, tracer);
    await dog.process({ type: 'test' });

    const span = storage.spans[0];
    assert.strictEqual(span.attributes['dog.response'], 'ALLOW');
    assert.strictEqual(span.attributes['dog.confidence'], 0.55);
  });

  it('should handle errors in process()', async () => {
    const dog = createMockDog();
    dog.process = async () => { throw new Error('dog error'); };
    traceDogExecution(dog, tracer);

    await assert.rejects(() => dog.process({ type: 'test' }), /dog error/);
    assert.strictEqual(storage.spans.length, 1);
    assert.strictEqual(storage.spans[0].status, SpanStatus.ERROR);
    assert.strictEqual(storage.spans[0].error, 'dog error');
  });

  it('should not double-wrap', () => {
    const dog = createMockDog();
    traceDogExecution(dog, tracer);
    const firstProcess = dog.process;
    traceDogExecution(dog, tracer);
    assert.strictEqual(dog.process, firstProcess);
  });

  it('should handle null dog gracefully', () => {
    const result = traceDogExecution(null, tracer);
    assert.strictEqual(result, null);
  });

  it('should trace all dogs in a map', async () => {
    const dogs = new Map([
      ['Guardian', createMockDog('Guardian')],
      ['Analyst', createMockDog('Analyst', { sefirah: 'Binah' })],
    ]);
    traceAllDogs(dogs, tracer);

    assert.strictEqual(dogs.get('Guardian')._traced, true);
    assert.strictEqual(dogs.get('Analyst')._traced, true);
  });
});

// ============================================================================
// CROSS-NODE PROPAGATION
// ============================================================================

describe('TracePropagation - Deep Tests', async () => {
  const { TracePropagation } = await import(
    '../../node/src/tracing/trace-propagation.js'
  );

  it('should inject trace context into message', () => {
    const span = new Span('send', new TraceContext({ traceId: 'prop-trace' }));
    const msg = { type: 'HEARTBEAT' };
    TracePropagation.inject(msg, span);
    assert.ok(msg._traceContext);
    assert.strictEqual(msg._traceContext.traceId, 'prop-trace');
  });

  it('should extract child context from message', () => {
    const parentCtx = new TraceContext({ traceId: 'extract-trace' });
    const msg = { _traceContext: parentCtx.toJSON() };

    const childCtx = TracePropagation.extract(msg);
    assert.ok(childCtx);
    assert.strictEqual(childCtx.traceId, 'extract-trace');
    assert.strictEqual(childCtx.parentSpanId, parentCtx.spanId);
  });

  it('should return null for message without context', () => {
    assert.strictEqual(TracePropagation.extract({}), null);
    assert.strictEqual(TracePropagation.extract(null), null);
  });

  it('should check hasContext', () => {
    const withCtx = { _traceContext: { traceId: 'x' } };
    const without = { type: 'HEARTBEAT' };
    assert.strictEqual(TracePropagation.hasContext(withCtx), true);
    assert.strictEqual(TracePropagation.hasContext(without), false);
    assert.strictEqual(TracePropagation.hasContext(null), false);
  });

  it('should handle inject with null span gracefully', () => {
    const msg = { type: 'TEST' };
    const result = TracePropagation.inject(msg, null);
    assert.strictEqual(result, msg);
    assert.strictEqual(msg._traceContext, undefined);
  });

  it('should roundtrip inject → extract', () => {
    const originalCtx = new TraceContext({ traceId: 'roundtrip', baggage: { env: 'test' } });
    const span = new Span('op', originalCtx);
    const msg = {};
    TracePropagation.inject(msg, span);

    const childCtx = TracePropagation.extract(msg);
    assert.strictEqual(childCtx.traceId, 'roundtrip');
    assert.strictEqual(childCtx.parentSpanId, originalCtx.spanId);
    assert.notStrictEqual(childCtx.spanId, originalCtx.spanId);
  });
});

// ============================================================================
// φ-ALIGNMENT
// ============================================================================

describe('φ-Alignment - Deep Tests', () => {
  it('should never sample above PHI_INV', () => {
    const sampler = createPhiSampler(0.99);
    let maxRate = 0;
    // Multiple batches to check consistency
    for (let batch = 0; batch < 5; batch++) {
      let count = 0;
      const N = 5000;
      for (let i = 0; i < N; i++) {
        if (sampler(null)) count++;
      }
      const rate = count / N;
      if (rate > maxRate) maxRate = rate;
    }
    assert.ok(maxRate <= PHI_INV + 0.05, `Max observed rate ${maxRate} should be near ${PHI_INV}`);
  });

  it('should use PHI_INV as the ceiling constant', () => {
    assert.ok(Math.abs(PHI_INV - 0.618033988749895) < 1e-10);
  });

  it('Tracer default service name should be cynic', () => {
    const t = new Tracer();
    assert.strictEqual(t.serviceName, 'cynic');
  });

  it('Span status values should include UNSET, OK, ERROR', () => {
    assert.strictEqual(SpanStatus.UNSET, 'unset');
    assert.strictEqual(SpanStatus.OK, 'ok');
    assert.strictEqual(SpanStatus.ERROR, 'error');
  });
});
