/**
 * Dog Pipeline Tests
 *
 * Comprehensive tests for StreamContext, PipelineStage, and DogPipeline classes.
 * Covers stream chaining, stage recording, abort handling, error propagation,
 * template execution, stats tracking, and pipeline formatting.
 *
 * @module @cynic/node/test/dog-pipeline
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';
import {
  DogPipeline,
  StreamContext,
  PipelineStage,
  PipelineTemplates,
  createDogPipeline,
  DogId,
  DOG_CAPABILITIES,
} from '../src/routing/index.js';

// ─────────────────────────────────────────────────────────────────────────────
// StreamContext
// ─────────────────────────────────────────────────────────────────────────────

describe('StreamContext', () => {
  let context;

  beforeEach(() => {
    context = new StreamContext({
      pipelineId: 'test_pipe',
      originalInput: 'test input',
      metadata: { test: true },
    });
  });

  describe('initialization', () => {
    it('should initialize with correct values', () => {
      assert.strictEqual(context.pipelineId, 'test_pipe');
      assert.strictEqual(context.originalInput, 'test input');
      assert.strictEqual(context.currentInput, 'test input');
      assert.strictEqual(context.currentStage, 0);
      assert.deepStrictEqual(context.metadata, { test: true });
      assert.ok(Array.isArray(context.stages));
      assert.ok(Array.isArray(context.outputs));
      assert.ok(Array.isArray(context.errors));
      assert.strictEqual(context.stages.length, 0);
      assert.strictEqual(context.aborted, false);
      assert.strictEqual(context.abortReason, null);
      assert.strictEqual(context.completedAt, null);
      assert.ok(typeof context.startedAt === 'number');
    });

    it('should default metadata to empty object', () => {
      const ctx = new StreamContext({
        pipelineId: 'p1',
        originalInput: 'x',
      });
      assert.deepStrictEqual(ctx.metadata, {});
    });
  });

  describe('recordStage()', () => {
    it('should record a successful stage and advance state', () => {
      const stage = context.recordStage({
        dogId: DogId.SCOUT,
        output: 'scout output',
        latency: 100,
        success: true,
      });

      assert.strictEqual(stage.index, 0);
      assert.strictEqual(stage.dogId, DogId.SCOUT);
      assert.strictEqual(stage.dogName, 'Scout');
      assert.strictEqual(stage.output, 'scout output');
      assert.strictEqual(stage.latency, 100);
      assert.strictEqual(stage.success, true);
      assert.strictEqual(stage.error, null);
      assert.ok(typeof stage.timestamp === 'number');
      assert.strictEqual(context.currentStage, 1);
      assert.strictEqual(context.currentInput, 'scout output');
      assert.strictEqual(context.outputs.length, 1);
      assert.strictEqual(context.outputs[0], 'scout output');
    });

    it('should record a failed stage and track error', () => {
      context.recordStage({
        dogId: DogId.SCOUT,
        output: null,
        latency: 50,
        success: false,
        error: 'Something failed',
      });

      assert.strictEqual(context.errors.length, 1);
      assert.strictEqual(context.errors[0].stage, 0);
      assert.strictEqual(context.errors[0].dogId, DogId.SCOUT);
      assert.strictEqual(context.errors[0].error, 'Something failed');
    });

    it('should chain inputs between consecutive stages', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'output1', latency: 100 });
      context.recordStage({ dogId: DogId.ANALYST, output: 'output2', latency: 150 });

      assert.strictEqual(context.stages.length, 2);
      assert.strictEqual(context.stages[0].output, 'output1');
      assert.strictEqual(context.stages[1].input, 'output1');
      assert.strictEqual(context.stages[1].output, 'output2');
      assert.strictEqual(context.currentInput, 'output2');
      assert.strictEqual(context.currentStage, 2);
    });

    it('should default success to true when not specified', () => {
      const stage = context.recordStage({
        dogId: DogId.SCOUT,
        output: 'data',
        latency: 50,
      });
      assert.strictEqual(stage.success, true);
      assert.strictEqual(context.errors.length, 0);
    });

    it('should increment stage indices sequentially', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'a', latency: 10 });
      context.recordStage({ dogId: DogId.ANALYST, output: 'b', latency: 20 });
      context.recordStage({ dogId: DogId.ARCHITECT, output: 'c', latency: 30 });

      assert.strictEqual(context.stages[0].index, 0);
      assert.strictEqual(context.stages[1].index, 1);
      assert.strictEqual(context.stages[2].index, 2);
    });
  });

  describe('abort()', () => {
    it('should set aborted state with reason', () => {
      context.abort('Manual abort');

      assert.strictEqual(context.aborted, true);
      assert.strictEqual(context.abortReason, 'Manual abort');
      assert.ok(typeof context.completedAt === 'number');
    });

    it('should set completedAt timestamp', () => {
      const before = Date.now();
      context.abort('reason');
      const after = Date.now();

      assert.ok(context.completedAt >= before);
      assert.ok(context.completedAt <= after);
    });
  });

  describe('complete()', () => {
    it('should set completedAt timestamp', () => {
      const before = Date.now();
      context.complete();
      const after = Date.now();

      assert.ok(context.completedAt >= before);
      assert.ok(context.completedAt <= after);
    });
  });

  describe('getFinalOutput()', () => {
    it('should return null when no stages recorded', () => {
      assert.strictEqual(context.getFinalOutput(), null);
    });

    it('should return last stage output', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'first', latency: 10 });
      context.recordStage({ dogId: DogId.ANALYST, output: 'second', latency: 10 });
      context.recordStage({ dogId: DogId.ARCHITECT, output: 'final', latency: 10 });

      assert.strictEqual(context.getFinalOutput(), 'final');
    });

    it('should return complex objects as final output', () => {
      context.recordStage({
        dogId: DogId.SCOUT,
        output: { data: [1, 2, 3], meta: { count: 3 } },
        latency: 10,
      });
      assert.deepStrictEqual(context.getFinalOutput(), { data: [1, 2, 3], meta: { count: 3 } });
    });
  });

  describe('getTotalLatency()', () => {
    it('should return 0 when no stages recorded', () => {
      assert.strictEqual(context.getTotalLatency(), 0);
    });

    it('should sum latencies across all stages', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'a', latency: 100 });
      context.recordStage({ dogId: DogId.ANALYST, output: 'b', latency: 150 });
      context.recordStage({ dogId: DogId.ARCHITECT, output: 'c', latency: 200 });

      assert.strictEqual(context.getTotalLatency(), 450);
    });
  });

  describe('isSuccessful()', () => {
    it('should return true when no errors and not aborted', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'a', latency: 100, success: true });
      context.complete();
      assert.strictEqual(context.isSuccessful(), true);
    });

    it('should return false when there are errors', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: null, latency: 100, success: false, error: 'fail' });
      assert.strictEqual(context.isSuccessful(), false);
    });

    it('should return false when aborted', () => {
      context.abort('reason');
      assert.strictEqual(context.isSuccessful(), false);
    });

    it('should return false when aborted even if no errors', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'ok', latency: 10, success: true });
      context.abort('forced');
      assert.strictEqual(context.isSuccessful(), false);
    });
  });

  describe('getOutputFrom()', () => {
    it('should return output from a specific dog', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'scout-data', latency: 100 });
      context.recordStage({ dogId: DogId.ANALYST, output: 'analyst-data', latency: 100 });

      assert.strictEqual(context.getOutputFrom(DogId.SCOUT), 'scout-data');
      assert.strictEqual(context.getOutputFrom(DogId.ANALYST), 'analyst-data');
    });

    it('should return null for a dog not in the pipeline', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'data', latency: 10 });
      assert.strictEqual(context.getOutputFrom(DogId.GUARDIAN), null);
    });

    it('should return first occurrence if dog appears multiple times', () => {
      context.recordStage({ dogId: DogId.GUARDIAN, output: 'audit-1', latency: 10 });
      context.recordStage({ dogId: DogId.ARCHITECT, output: 'fix', latency: 10 });
      context.recordStage({ dogId: DogId.GUARDIAN, output: 'verify', latency: 10 });

      // find() returns the first match
      assert.strictEqual(context.getOutputFrom(DogId.GUARDIAN), 'audit-1');
    });
  });

  describe('getStage()', () => {
    it('should return stage by index', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'a', latency: 10 });
      const stage = context.getStage(0);
      assert.strictEqual(stage.dogId, DogId.SCOUT);
    });

    it('should return null for out-of-range index', () => {
      assert.strictEqual(context.getStage(99), null);
    });
  });

  describe('toJSON()', () => {
    it('should serialize all fields', () => {
      context.recordStage({ dogId: DogId.SCOUT, output: 'a', latency: 100, success: true });
      context.complete();

      const json = context.toJSON();

      assert.strictEqual(json.pipelineId, 'test_pipe');
      assert.strictEqual(json.originalInput, 'test input');
      assert.strictEqual(json.stageCount, 1);
      assert.ok(Array.isArray(json.stages));
      assert.strictEqual(json.stages[0].dogId, DogId.SCOUT);
      assert.strictEqual(json.stages[0].dogName, 'Scout');
      assert.strictEqual(json.stages[0].success, true);
      assert.strictEqual(json.stages[0].latency, 100);
      assert.strictEqual(json.totalLatency, 100);
      assert.strictEqual(json.successful, true);
      assert.strictEqual(json.aborted, false);
      assert.strictEqual(json.abortReason, null);
      assert.strictEqual(json.errorCount, 0);
      assert.ok(typeof json.startedAt === 'number');
      assert.ok(typeof json.completedAt === 'number');
    });

    it('should truncate long string inputs to 200 chars', () => {
      const longInput = 'x'.repeat(500);
      const ctx = new StreamContext({
        pipelineId: 'long',
        originalInput: longInput,
      });
      const json = ctx.toJSON();
      assert.strictEqual(json.originalInput.length, 200);
    });

    it('should preserve non-string originalInput as-is', () => {
      const ctx = new StreamContext({
        pipelineId: 'obj',
        originalInput: { complex: true },
      });
      const json = ctx.toJSON();
      assert.deepStrictEqual(json.originalInput, { complex: true });
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// PipelineStage
// ─────────────────────────────────────────────────────────────────────────────

describe('PipelineStage', () => {
  it('should create stage with default values', () => {
    const stage = new PipelineStage({ dogId: DogId.SCOUT });

    assert.strictEqual(stage.dogId, DogId.SCOUT);
    assert.strictEqual(stage.handler, null);
    assert.strictEqual(stage.transform, null);
    assert.strictEqual(stage.validate, null);
    assert.strictEqual(stage.optional, false);
    assert.strictEqual(stage.timeout, 30000);
  });

  it('should accept custom timeout', () => {
    const stage = new PipelineStage({ dogId: DogId.ANALYST, timeout: 5000 });
    assert.strictEqual(stage.timeout, 5000);
  });

  it('should accept optional flag', () => {
    const stage = new PipelineStage({ dogId: DogId.ANALYST, optional: true });
    assert.strictEqual(stage.optional, true);
  });

  describe('transformInput()', () => {
    it('should pass through input when no transform function', () => {
      const stage = new PipelineStage({ dogId: DogId.SCOUT });
      const context = new StreamContext({
        pipelineId: 'test',
        originalInput: 'hello',
      });
      assert.strictEqual(stage.transformInput(context), 'hello');
    });

    it('should apply transform function to input', () => {
      const stage = new PipelineStage({
        dogId: DogId.SCOUT,
        transform: (input) => input.toUpperCase(),
      });
      const context = new StreamContext({
        pipelineId: 'test',
        originalInput: 'hello',
      });
      assert.strictEqual(stage.transformInput(context), 'HELLO');
    });

    it('should pass context as second argument to transform', () => {
      const stage = new PipelineStage({
        dogId: DogId.SCOUT,
        transform: (input, ctx) => `${input}:${ctx.pipelineId}`,
      });
      const context = new StreamContext({
        pipelineId: 'pipe123',
        originalInput: 'data',
      });
      assert.strictEqual(stage.transformInput(context), 'data:pipe123');
    });
  });

  describe('validateOutput()', () => {
    it('should return valid when no validate function', () => {
      const stage = new PipelineStage({ dogId: DogId.SCOUT });
      const context = new StreamContext({ pipelineId: 'test', originalInput: 'x' });
      const result = stage.validateOutput('anything', context);
      assert.deepStrictEqual(result, { valid: true });
    });

    it('should call validate function and return result', () => {
      const stage = new PipelineStage({
        dogId: DogId.SCOUT,
        validate: (output) => ({
          valid: output.length > 5,
          reason: 'Output too short',
        }),
      });
      const context = new StreamContext({ pipelineId: 'test', originalInput: 'x' });

      assert.strictEqual(stage.validateOutput('ab', context).valid, false);
      assert.strictEqual(stage.validateOutput('ab', context).reason, 'Output too short');
      assert.strictEqual(stage.validateOutput('abcdef', context).valid, true);
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// PipelineTemplates
// ─────────────────────────────────────────────────────────────────────────────

describe('PipelineTemplates', () => {
  it('should have all predefined templates', () => {
    assert.ok(PipelineTemplates.EXPLORE_ANALYZE_BUILD);
    assert.ok(PipelineTemplates.RESEARCH_DOCUMENT);
    assert.ok(PipelineTemplates.SECURITY_AUDIT);
    assert.ok(PipelineTemplates.CLEANUP_DEPLOY);
    assert.ok(PipelineTemplates.DEEP_ANALYSIS);
    assert.ok(PipelineTemplates.SYNTHESIS);
  });

  it('should have valid dog IDs in all templates', () => {
    const validDogIds = Object.values(DogId);
    for (const [name, stages] of Object.entries(PipelineTemplates)) {
      for (const stage of stages) {
        assert.ok(
          validDogIds.includes(stage.dogId),
          `Invalid dogId "${stage.dogId}" in template ${name}`
        );
      }
    }
  });

  it('should have descriptions for all template stages', () => {
    for (const [name, stages] of Object.entries(PipelineTemplates)) {
      for (const stage of stages) {
        assert.ok(
          typeof stage.description === 'string' && stage.description.length > 0,
          `Missing description in template ${name} for dog ${stage.dogId}`
        );
      }
    }
  });

  it('should be frozen (immutable)', () => {
    assert.ok(Object.isFrozen(PipelineTemplates));
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// DogPipeline
// ─────────────────────────────────────────────────────────────────────────────

describe('DogPipeline', () => {
  let pipeline;

  beforeEach(() => {
    pipeline = createDogPipeline();
  });

  // ── registerHandler() ────────────────────────────────────────────────────

  describe('registerHandler()', () => {
    it('should register a handler by dog ID', () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `scouted: ${input}`);
      assert.ok(pipeline.handlers.has(DogId.SCOUT));
    });

    it('should emit handler:registered event', (t, done) => {
      pipeline.on('handler:registered', (evt) => {
        assert.strictEqual(evt.dogId, DogId.ANALYST);
        done();
      });
      pipeline.registerHandler(DogId.ANALYST, async () => {});
    });

    it('should overwrite existing handler for same dog', () => {
      const handler1 = async () => 'v1';
      const handler2 = async () => 'v2';
      pipeline.registerHandler(DogId.SCOUT, handler1);
      pipeline.registerHandler(DogId.SCOUT, handler2);
      assert.strictEqual(pipeline.handlers.get(DogId.SCOUT), handler2);
    });
  });

  // ── fromTemplate() ──────────────────────────────────────────────────────

  describe('fromTemplate()', () => {
    it('should create PipelineStage array from template name', () => {
      const stages = pipeline.fromTemplate('EXPLORE_ANALYZE_BUILD');
      assert.strictEqual(stages.length, 3);
      assert.ok(stages[0] instanceof PipelineStage);
      assert.strictEqual(stages[0].dogId, DogId.SCOUT);
      assert.strictEqual(stages[1].dogId, DogId.ANALYST);
      assert.strictEqual(stages[2].dogId, DogId.ARCHITECT);
    });

    it('should throw for unknown template name', () => {
      assert.throws(() => {
        pipeline.fromTemplate('NONEXISTENT');
      }, /Unknown template/);
    });
  });

  // ── chain() ──────────────────────────────────────────────────────────────

  describe('chain()', () => {
    it('should create PipelineStage array from dog IDs', () => {
      const stages = pipeline.chain(DogId.SCOUT, DogId.ANALYST, DogId.ARCHITECT);
      assert.strictEqual(stages.length, 3);
      assert.ok(stages.every(s => s instanceof PipelineStage));
      assert.strictEqual(stages[0].dogId, DogId.SCOUT);
      assert.strictEqual(stages[1].dogId, DogId.ANALYST);
      assert.strictEqual(stages[2].dogId, DogId.ARCHITECT);
    });

    it('should create single-stage pipeline', () => {
      const stages = pipeline.chain(DogId.SCOUT);
      assert.strictEqual(stages.length, 1);
    });
  });

  // ── execute() ────────────────────────────────────────────────────────────

  describe('execute()', () => {
    it('should execute a simple two-stage pipeline', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `scouted: ${input}`);
      pipeline.registerHandler(DogId.ANALYST, async (input) => `analyzed: ${input}`);

      const stages = pipeline.chain(DogId.SCOUT, DogId.ANALYST);
      const context = await pipeline.execute(stages, 'test');

      assert.ok(context.isSuccessful());
      assert.strictEqual(context.stages.length, 2);
      assert.strictEqual(context.getFinalOutput(), 'analyzed: scouted: test');
    });

    it('should pass output from one stage to the next', async () => {
      pipeline.registerHandler(DogId.SCOUT, async () => ({ data: 'scout-data' }));
      pipeline.registerHandler(DogId.ANALYST, async (input) => ({
        analysis: `analyzed-${input.data}`,
      }));

      const stages = pipeline.chain(DogId.SCOUT, DogId.ANALYST);
      const context = await pipeline.execute(stages, 'initial');

      assert.deepStrictEqual(context.getFinalOutput(), { analysis: 'analyzed-scout-data' });
    });

    it('should abort when handler is missing for non-optional stage', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      // No ANALYST handler

      const stages = pipeline.chain(DogId.SCOUT, DogId.ANALYST);
      const context = await pipeline.execute(stages, 'test');

      assert.strictEqual(context.aborted, true);
      assert.ok(context.abortReason.includes('No handler'));
    });

    it('should skip optional stages without handlers', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `scouted: ${input}`);
      pipeline.registerHandler(DogId.ARCHITECT, async (input) => `built: ${input}`);

      const stages = [
        new PipelineStage({ dogId: DogId.SCOUT }),
        new PipelineStage({ dogId: DogId.ANALYST, optional: true }),
        new PipelineStage({ dogId: DogId.ARCHITECT }),
      ];
      const context = await pipeline.execute(stages, 'test');

      assert.ok(context.isSuccessful());
      assert.strictEqual(context.stages.length, 2);
      assert.strictEqual(context.getFinalOutput(), 'built: scouted: test');
    });

    it('should abort on stage error when continueOnError is false (default)', async () => {
      pipeline.registerHandler(DogId.SCOUT, async () => {
        throw new Error('Scout failed');
      });

      const stages = pipeline.chain(DogId.SCOUT);
      const context = await pipeline.execute(stages, 'test');

      assert.strictEqual(context.aborted, true);
      assert.ok(context.abortReason.includes('Scout failed'));
    });

    it('should continue on error when continueOnError is true', async () => {
      const cPipeline = createDogPipeline({ continueOnError: true });
      cPipeline.registerHandler(DogId.SCOUT, async () => {
        throw new Error('Scout failed');
      });
      cPipeline.registerHandler(DogId.ANALYST, async (input) => `analyzed: ${input}`);

      const stages = cPipeline.chain(DogId.SCOUT, DogId.ANALYST);
      const context = await cPipeline.execute(stages, 'test');

      assert.strictEqual(context.stages.length, 2);
      assert.strictEqual(context.errors.length, 1);
      assert.strictEqual(context.isSuccessful(), false);
      // Second stage still executed
      assert.strictEqual(context.stages[1].success, true);
    });

    it('should reject pipelines exceeding maxStages', async () => {
      const smallPipeline = createDogPipeline({ maxStages: 2 });
      const stages = smallPipeline.chain(DogId.SCOUT, DogId.ANALYST, DogId.ARCHITECT);

      await assert.rejects(
        () => smallPipeline.execute(stages, 'test'),
        /exceeds maximum stages/
      );
    });

    it('should apply stage transform before handler execution', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `processed: ${input}`);

      const stages = [
        new PipelineStage({
          dogId: DogId.SCOUT,
          transform: (input) => input.toUpperCase(),
        }),
      ];
      const context = await pipeline.execute(stages, 'hello');

      assert.strictEqual(context.getFinalOutput(), 'processed: HELLO');
    });

    it('should abort when stage output fails validation', async () => {
      pipeline.registerHandler(DogId.SCOUT, async () => 'ab');

      const stages = [
        new PipelineStage({
          dogId: DogId.SCOUT,
          validate: (output) => ({
            valid: output.length > 5,
            reason: 'Output too short',
          }),
        }),
      ];
      const context = await pipeline.execute(stages, 'test');

      assert.strictEqual(context.aborted, true);
      assert.ok(context.abortReason.includes('Output too short'));
    });

    it('should use stage-level handler over pipeline-level handler', async () => {
      pipeline.registerHandler(DogId.SCOUT, async () => 'pipeline-handler');

      const stages = [
        new PipelineStage({
          dogId: DogId.SCOUT,
          handler: async () => 'stage-handler',
        }),
      ];
      const context = await pipeline.execute(stages, 'test');

      assert.strictEqual(context.getFinalOutput(), 'stage-handler');
    });

    it('should set completedAt on context after execution', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      const stages = pipeline.chain(DogId.SCOUT);
      const context = await pipeline.execute(stages, 'test');

      assert.ok(typeof context.completedAt === 'number');
      assert.ok(context.completedAt >= context.startedAt);
    });

    it('should pass metadata to context', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input, ctx) => {
        return `meta:${ctx.metadata.key}`;
      });

      const stages = pipeline.chain(DogId.SCOUT);
      const context = await pipeline.execute(stages, 'test', { metadata: { key: 'val' } });

      assert.strictEqual(context.getFinalOutput(), 'meta:val');
    });
  });

  // ── executeTemplate() ────────────────────────────────────────────────────

  describe('executeTemplate()', () => {
    it('should execute a named template pipeline', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `explored: ${input}`);
      pipeline.registerHandler(DogId.ANALYST, async (input) => `analyzed: ${input}`);
      pipeline.registerHandler(DogId.ARCHITECT, async (input) => `designed: ${input}`);

      const context = await pipeline.executeTemplate('EXPLORE_ANALYZE_BUILD', 'task');

      assert.ok(context.isSuccessful());
      assert.strictEqual(context.stages.length, 3);
      assert.strictEqual(context.getFinalOutput(), 'designed: analyzed: explored: task');
    });

    it('should track template name in stats', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      pipeline.registerHandler(DogId.ANALYST, async (input) => input);
      pipeline.registerHandler(DogId.ARCHITECT, async (input) => input);

      await pipeline.executeTemplate('EXPLORE_ANALYZE_BUILD', 'test');
      await pipeline.executeTemplate('EXPLORE_ANALYZE_BUILD', 'test2');

      const stats = pipeline.getStats();
      assert.strictEqual(stats.byTemplate['EXPLORE_ANALYZE_BUILD'].run, 2);
      assert.strictEqual(stats.byTemplate['EXPLORE_ANALYZE_BUILD'].success, 2);
    });

    it('should throw for unknown template', async () => {
      await assert.rejects(
        () => pipeline.executeTemplate('NONEXISTENT', 'test'),
        /Unknown template/
      );
    });
  });

  // ── executeChain() ──────────────────────────────────────────────────────

  describe('executeChain()', () => {
    it('should execute chain shorthand', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `[scout:${input}]`);
      pipeline.registerHandler(DogId.ANALYST, async (input) => `[analyst:${input}]`);

      const context = await pipeline.executeChain('start', DogId.SCOUT, DogId.ANALYST);

      assert.ok(context.isSuccessful());
      assert.strictEqual(context.getFinalOutput(), '[analyst:[scout:start]]');
    });

    it('should work with single dog in chain', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => `done: ${input}`);

      const context = await pipeline.executeChain('go', DogId.SCOUT);
      assert.strictEqual(context.getFinalOutput(), 'done: go');
    });
  });

  // ── abort() ──────────────────────────────────────────────────────────────

  describe('abort()', () => {
    it('should abort an active pipeline by ID', async () => {
      let resolveHandler;
      const waitingHandler = new Promise((r) => { resolveHandler = r; });

      pipeline.registerHandler(DogId.SCOUT, async () => {
        await waitingHandler;
        return 'done';
      });

      const stages = pipeline.chain(DogId.SCOUT);
      const executePromise = pipeline.execute(stages, 'test');

      // Wait for pipeline to become active
      await new Promise((r) => setTimeout(r, 10));
      const active = pipeline.getActivePipelines();
      assert.strictEqual(active.length, 1);

      const aborted = pipeline.abort(active[0].pipelineId, 'Test abort');
      assert.strictEqual(aborted, true);

      resolveHandler();
      const context = await executePromise;
      assert.strictEqual(context.aborted, true);
    });

    it('should return false for non-existent pipeline ID', () => {
      const result = pipeline.abort('non_existent_id');
      assert.strictEqual(result, false);
    });

    it('should emit pipeline:aborted event', async () => {
      let resolveHandler;
      const waitingHandler = new Promise((r) => { resolveHandler = r; });
      const events = [];

      pipeline.on('pipeline:aborted', (e) => events.push(e));
      pipeline.registerHandler(DogId.SCOUT, async () => {
        await waitingHandler;
        return 'done';
      });

      const stages = pipeline.chain(DogId.SCOUT);
      const executePromise = pipeline.execute(stages, 'test');

      await new Promise((r) => setTimeout(r, 10));
      const active = pipeline.getActivePipelines();
      pipeline.abort(active[0].pipelineId, 'forced');
      resolveHandler();
      await executePromise;

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].reason, 'forced');
    });
  });

  // ── getStats() ───────────────────────────────────────────────────────────

  describe('getStats()', () => {
    it('should track pipeline run count and stage count', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      pipeline.registerHandler(DogId.ANALYST, async (input) => input);

      await pipeline.executeChain('test', DogId.SCOUT, DogId.ANALYST);
      await pipeline.executeChain('test', DogId.SCOUT);

      const stats = pipeline.getStats();
      assert.strictEqual(stats.pipelinesRun, 2);
      assert.strictEqual(stats.stagesExecuted, 3);
      assert.strictEqual(stats.successfulPipelines, 2);
    });

    it('should track failed pipelines', async () => {
      pipeline.registerHandler(DogId.SCOUT, async () => {
        throw new Error('fail');
      });

      await pipeline.executeChain('test', DogId.SCOUT);

      const stats = pipeline.getStats();
      assert.strictEqual(stats.abortedPipelines, 1);
    });

    it('should calculate success rate', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);

      await pipeline.executeChain('test', DogId.SCOUT);
      await pipeline.executeChain('test', DogId.SCOUT);

      const stats = pipeline.getStats();
      assert.strictEqual(stats.successRate, 1);
    });

    it('should calculate average latency', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);

      await pipeline.executeChain('test', DogId.SCOUT);

      const stats = pipeline.getStats();
      assert.ok(typeof stats.avgLatency === 'number');
      assert.ok(stats.avgLatency >= 0);
    });

    it('should report handler count', () => {
      pipeline.registerHandler(DogId.SCOUT, async () => {});
      pipeline.registerHandler(DogId.ANALYST, async () => {});

      const stats = pipeline.getStats();
      assert.strictEqual(stats.handlersRegistered, 2);
    });

    it('should report active pipeline count', () => {
      const stats = pipeline.getStats();
      assert.strictEqual(stats.activePipelines, 0);
    });

    it('should calculate avgStagesPerPipeline', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      pipeline.registerHandler(DogId.ANALYST, async (input) => input);

      await pipeline.executeChain('test', DogId.SCOUT, DogId.ANALYST);
      await pipeline.executeChain('test', DogId.SCOUT);

      const stats = pipeline.getStats();
      // 3 stages / 2 pipelines = 1.5
      assert.strictEqual(stats.avgStagesPerPipeline, 1.5);
    });
  });

  // ── resetStats() ─────────────────────────────────────────────────────────

  describe('resetStats()', () => {
    it('should reset all counters', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      await pipeline.executeChain('test', DogId.SCOUT);

      pipeline.resetStats();

      const stats = pipeline.getStats();
      assert.strictEqual(stats.pipelinesRun, 0);
      assert.strictEqual(stats.stagesExecuted, 0);
      assert.strictEqual(stats.successfulPipelines, 0);
      assert.strictEqual(stats.failedPipelines, 0);
      assert.strictEqual(stats.abortedPipelines, 0);
      assert.strictEqual(stats.totalLatency, 0);
      assert.deepStrictEqual(stats.byTemplate, {});
    });
  });

  // ── Events ───────────────────────────────────────────────────────────────

  describe('event emission', () => {
    it('should emit pipeline:start and pipeline:complete', async () => {
      const events = [];
      pipeline.on('pipeline:start', (e) => events.push({ type: 'start', ...e }));
      pipeline.on('pipeline:complete', (e) => events.push({ type: 'complete', ...e }));

      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      await pipeline.executeChain('test', DogId.SCOUT);

      assert.ok(events.some(e => e.type === 'start'));
      assert.ok(events.some(e => e.type === 'complete'));
    });

    it('should emit stage:start and stage:complete for each stage', async () => {
      const stageEvents = [];
      pipeline.on('stage:start', (e) => stageEvents.push(e));
      pipeline.on('stage:complete', (e) => stageEvents.push(e));

      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      pipeline.registerHandler(DogId.ANALYST, async (input) => input);
      await pipeline.executeChain('test', DogId.SCOUT, DogId.ANALYST);

      // 2 stages x 2 events each = 4
      assert.strictEqual(stageEvents.length, 4);
    });

    it('should emit stage:error on handler failure', async () => {
      const errors = [];
      pipeline.on('stage:error', (e) => errors.push(e));

      pipeline.registerHandler(DogId.SCOUT, async () => {
        throw new Error('boom');
      });
      await pipeline.executeChain('test', DogId.SCOUT);

      assert.strictEqual(errors.length, 1);
      assert.ok(errors[0].error.includes('boom'));
    });

    it('should emit stage:skipped for optional stages without handlers', async () => {
      const skipped = [];
      pipeline.on('stage:skipped', (e) => skipped.push(e));

      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      pipeline.registerHandler(DogId.ARCHITECT, async (input) => input);

      const stages = [
        new PipelineStage({ dogId: DogId.SCOUT }),
        new PipelineStage({ dogId: DogId.ANALYST, optional: true }),
        new PipelineStage({ dogId: DogId.ARCHITECT }),
      ];
      await pipeline.execute(stages, 'test');

      assert.strictEqual(skipped.length, 1);
      assert.strictEqual(skipped[0].dogId, DogId.ANALYST);
    });
  });

  // ── Static formatting ───────────────────────────────────────────────────

  describe('formatPipeline()', () => {
    it('should format pipeline as arrow-separated dog names', () => {
      const stages = pipeline.chain(DogId.SCOUT, DogId.ANALYST, DogId.ARCHITECT);
      const formatted = DogPipeline.formatPipeline(stages);

      assert.ok(formatted.includes('Scout'));
      assert.ok(formatted.includes('Analyst'));
      assert.ok(formatted.includes('Architect'));
      assert.ok(formatted.includes('\u2192')); // arrow character
    });

    it('should include emojis from dog capabilities', () => {
      const stages = pipeline.chain(DogId.GUARDIAN);
      const formatted = DogPipeline.formatPipeline(stages);
      assert.ok(formatted.includes(DOG_CAPABILITIES[DogId.GUARDIAN].emoji));
    });
  });

  describe('formatResult()', () => {
    it('should format a successful pipeline result', async () => {
      pipeline.registerHandler(DogId.SCOUT, async (input) => input);
      const context = await pipeline.executeChain('test', DogId.SCOUT);

      const formatted = DogPipeline.formatResult(context);

      assert.ok(formatted.includes('Pipeline:'));
      assert.ok(formatted.includes('Success'));
      assert.ok(formatted.includes('Scout'));
      assert.ok(formatted.includes('Total:'));
    });

    it('should format an aborted pipeline result', async () => {
      pipeline.registerHandler(DogId.SCOUT, async () => { throw new Error('fail'); });
      const context = await pipeline.executeChain('test', DogId.SCOUT);

      const formatted = DogPipeline.formatResult(context);

      assert.ok(formatted.includes('Aborted'));
    });

    it('should show error details for failed stages', async () => {
      const cp = createDogPipeline({ continueOnError: true });
      cp.registerHandler(DogId.SCOUT, async () => { throw new Error('scout error'); });
      cp.registerHandler(DogId.ANALYST, async (input) => input);

      const context = await cp.executeChain('test', DogId.SCOUT, DogId.ANALYST);
      const formatted = DogPipeline.formatResult(context);

      assert.ok(formatted.includes('scout error'));
    });
  });

  // ── getActivePipelines() ─────────────────────────────────────────────────

  describe('getActivePipelines()', () => {
    it('should return empty when no pipelines running', () => {
      const active = pipeline.getActivePipelines();
      assert.strictEqual(active.length, 0);
    });
  });
});
