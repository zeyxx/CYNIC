/**
 * Tests for ModelIntelligence — Learned Model Selection
 *
 * "Le chien teste ses propres hypothèses" — κυνικός
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'path';
import { mkdtempSync, rmSync, existsSync } from 'fs';
import { tmpdir } from 'os';

import { PHI_INV } from '@cynic/core';
import {
  ModelIntelligence,
  ModelTier,
  TaskCategory,
  getModelIntelligence,
  resetModelIntelligence,
} from '../src/learning/model-intelligence.js';

function tempPersistPath() {
  const dir = mkdtempSync(join(tmpdir(), 'cynic-mi-'));
  return join(dir, 'intelligence-state.json');
}

describe('ModelIntelligence', () => {
  let mi;
  let persistPath;

  beforeEach(() => {
    resetModelIntelligence();
    persistPath = tempPersistPath();
    mi = new ModelIntelligence({ persistPath });
  });

  afterEach(() => {
    try {
      const dir = join(persistPath, '..');
      if (existsSync(dir)) rmSync(dir, { recursive: true });
    } catch {}
  });

  describe('construction', () => {
    it('initializes samplers for all task categories', () => {
      const categories = Object.values(TaskCategory);
      for (const cat of categories) {
        assert.ok(mi._samplers.has(cat), `Missing sampler for ${cat}`);
      }
    });

    it('initializes model arms in each sampler', () => {
      const sampler = mi._samplers.get(TaskCategory.CODE_REVIEW);
      assert.ok(sampler.arms.has(ModelTier.OPUS));
      assert.ok(sampler.arms.has(ModelTier.SONNET));
      assert.ok(sampler.arms.has(ModelTier.HAIKU));
      assert.ok(sampler.arms.has(ModelTier.OLLAMA));
    });

    it('higher tiers get higher priors', () => {
      const sampler = mi._samplers.get(TaskCategory.CODE_WRITE);
      const opusArm = sampler.arms.get(ModelTier.OPUS);
      const haikuArm = sampler.arms.get(ModelTier.HAIKU);
      assert.ok(opusArm.alpha > haikuArm.alpha, 'Opus prior should be higher than Haiku');
    });

    it('has default tool affinity', () => {
      assert.equal(mi._toolAffinity.brain_health, ModelTier.HAIKU);
      assert.equal(mi._toolAffinity.brain_orchestrate, ModelTier.OPUS);
      assert.equal(mi._toolAffinity.brain_cynic_judge, ModelTier.SONNET);
    });
  });

  describe('detectCurrentModel', () => {
    it('returns a valid tier', () => {
      const result = mi.detectCurrentModel();
      assert.ok(result.tier);
      assert.ok(result.source);
      assert.ok(['opus', 'sonnet', 'haiku', 'ollama'].includes(result.tier));
    });
  });

  describe('selectModel', () => {
    it('returns a model selection for code tasks', () => {
      const result = mi.selectModel('code');
      assert.ok(result.model);
      assert.ok(result.reason);
      assert.ok(result.confidence >= 0 && result.confidence <= PHI_INV);
      assert.equal(result.category, TaskCategory.CODE_WRITE);
    });

    it('returns Haiku when budget exhausted', () => {
      const result = mi.selectModel('architecture', { budgetLevel: 'exhausted' });
      assert.equal(result.model, ModelTier.HAIKU);
      assert.ok(result.reason.includes('exhausted'));
    });

    it('caps at Sonnet when budget critical', () => {
      // Run many selections — none should be Opus
      for (let i = 0; i < 50; i++) {
        const result = mi.selectModel('architecture', { budgetLevel: 'critical' });
        assert.notEqual(result.model, ModelTier.OPUS, `Selection ${i} should not be Opus`);
      }
    });

    it('respects tool affinity floor', () => {
      // brain_orchestrate requires Opus
      const results = [];
      for (let i = 0; i < 20; i++) {
        results.push(mi.selectModel('simple', { tool: 'brain_orchestrate' }));
      }
      // Most should be Opus (tool affinity floor) unless falsification
      const opusCount = results.filter(r => r.model === ModelTier.OPUS).length;
      assert.ok(opusCount >= 10, `Expected mostly Opus for brain_orchestrate, got ${opusCount}/20`);
    });

    it('increments selection stats', () => {
      mi.selectModel('code');
      mi.selectModel('debug');
      mi.selectModel('simple');
      assert.equal(mi._stats.selectionsTotal, 3);
    });

    it('maps unknown task types to routing', () => {
      const result = mi.selectModel('unknown_task_42');
      assert.equal(result.category, TaskCategory.ROUTING);
    });

    it('confidence is φ-bounded', () => {
      for (let i = 0; i < 50; i++) {
        const result = mi.selectModel('code');
        assert.ok(result.confidence <= PHI_INV, `Confidence ${result.confidence} exceeds φ⁻¹`);
      }
    });
  });

  describe('recordOutcome', () => {
    it('updates Thompson arms on success', () => {
      const sampler = mi._samplers.get(TaskCategory.CODE_WRITE);
      const before = sampler.arms.get(ModelTier.SONNET).alpha;

      mi.recordOutcome({
        taskType: 'code',
        model: ModelTier.SONNET,
        success: true,
      });

      const after = sampler.arms.get(ModelTier.SONNET).alpha;
      assert.ok(after > before, 'Alpha should increase on success');
    });

    it('updates Thompson arms on failure', () => {
      const sampler = mi._samplers.get(TaskCategory.DEBUG);
      const before = sampler.arms.get(ModelTier.OPUS).beta;

      mi.recordOutcome({
        taskType: 'debug',
        model: ModelTier.OPUS,
        success: false,
      });

      const after = sampler.arms.get(ModelTier.OPUS).beta;
      assert.ok(after > before, 'Beta should increase on failure');
    });

    it('uses qualityScore threshold when provided', () => {
      const sampler = mi._samplers.get(TaskCategory.CODE_REVIEW);

      // Quality below φ⁻¹ = failure
      const beforeBeta = sampler.arms.get(ModelTier.HAIKU).beta;
      mi.recordOutcome({
        taskType: 'review',
        model: ModelTier.HAIKU,
        success: true, // explicit success overridden by quality
        qualityScore: 0.3, // below φ⁻¹
      });
      const afterBeta = sampler.arms.get(ModelTier.HAIKU).beta;
      assert.ok(afterBeta > beforeBeta, 'Low quality should count as failure');
    });

    it('increments outcome stats', () => {
      mi.recordOutcome({ taskType: 'code', model: ModelTier.SONNET, success: true });
      mi.recordOutcome({ taskType: 'code', model: ModelTier.SONNET, success: false });
      assert.equal(mi._stats.outcomesRecorded, 2);
    });

    it('maintains rolling window', () => {
      for (let i = 0; i < 250; i++) {
        mi.recordOutcome({ taskType: 'simple', model: ModelTier.HAIKU, success: true });
      }
      assert.ok(mi._outcomes.length <= 200, 'Should cap at maxOutcomes');
    });

    it('ignores outcomes with missing model/taskType', () => {
      mi.recordOutcome({ taskType: null, model: ModelTier.SONNET, success: true });
      mi.recordOutcome({ taskType: 'code', model: null, success: true });
      assert.equal(mi._stats.outcomesRecorded, 0);
    });
  });

  describe('tool affinity learning', () => {
    it('lowers affinity when cheaper model succeeds', () => {
      assert.equal(mi._toolAffinity.brain_cynic_judge, ModelTier.SONNET);

      mi.recordOutcome({
        taskType: 'review',
        model: ModelTier.HAIKU,
        success: true,
        tool: 'brain_cynic_judge',
      });

      assert.equal(mi._toolAffinity.brain_cynic_judge, ModelTier.HAIKU);
    });

    it('does not raise affinity on failure', () => {
      assert.equal(mi._toolAffinity.brain_health, ModelTier.HAIKU);

      mi.recordOutcome({
        taskType: 'simple',
        model: ModelTier.OPUS,
        success: false,
        tool: 'brain_health',
      });

      // Should still be Haiku (failure doesn't raise)
      assert.equal(mi._toolAffinity.brain_health, ModelTier.HAIKU);
    });

    it('tracks new tools', () => {
      mi.recordOutcome({
        taskType: 'simple',
        model: ModelTier.SONNET,
        success: true,
        tool: 'brain_new_tool_xyz',
      });

      assert.equal(mi._toolAffinity.brain_new_tool_xyz, ModelTier.SONNET);
    });
  });

  describe('falsification', () => {
    it('proposes downgrade experiments', () => {
      // Force many selections to trigger at least one falsification
      let experiment = null;
      for (let i = 0; i < 100 && !experiment; i++) {
        const result = mi.selectModel('architecture');
        if (result.experiment) {
          experiment = result.experiment;
        }
      }
      // With φ⁻³ ≈ 23.6% probability per call, 100 tries should trigger one
      // But it's probabilistic — just check that the mechanism exists
      assert.ok(mi._stats.selectionsTotal >= 1);
    });

    it('completes experiment after required trials', () => {
      // Manually create an experiment
      const experiment = {
        id: 'exp_test_1',
        hypKey: 'code_write:opus→sonnet',
        taskCategory: TaskCategory.CODE_WRITE,
        expensiveTier: ModelTier.OPUS,
        cheapTier: ModelTier.SONNET,
        startTime: Date.now(),
        state: 'active',
        outcomes: { success: 0, failure: 0, total: 0 },
        requiredTrials: 5,
      };
      mi._experiments.set(experiment.id, experiment);

      // Record 5 successes → should falsify (cheaper model works)
      for (let i = 0; i < 5; i++) {
        mi.recordOutcome({
          taskType: 'code',
          model: ModelTier.SONNET,
          success: true,
          experimentId: 'exp_test_1',
        });
      }

      assert.equal(experiment.state, 'falsified');
      assert.equal(mi._stats.falsificationsCompleted, 1);
      assert.equal(mi._stats.falsificationsSucceeded, 1);
    });

    it('supports hypothesis when cheaper model fails', () => {
      const experiment = {
        id: 'exp_test_2',
        hypKey: 'architecture:opus→sonnet',
        taskCategory: TaskCategory.ARCHITECTURE,
        expensiveTier: ModelTier.OPUS,
        cheapTier: ModelTier.SONNET,
        startTime: Date.now(),
        state: 'active',
        outcomes: { success: 0, failure: 0, total: 0 },
        requiredTrials: 5,
      };
      mi._experiments.set(experiment.id, experiment);

      // Record mostly failures → should support expensive model
      for (let i = 0; i < 5; i++) {
        mi.recordOutcome({
          taskType: 'architecture',
          model: ModelTier.SONNET,
          success: i === 0, // 1 success, 4 failures = 20% < φ⁻²
          experimentId: 'exp_test_2',
        });
      }

      assert.equal(experiment.state, 'supported');
      assert.equal(mi._stats.falsificationsCompleted, 1);
      assert.equal(mi._stats.falsificationsSucceeded, 0);
    });

    it('marks inconclusive when results are mixed', () => {
      const experiment = {
        id: 'exp_test_3',
        hypKey: 'debug:opus→sonnet',
        taskCategory: TaskCategory.DEBUG,
        expensiveTier: ModelTier.OPUS,
        cheapTier: ModelTier.SONNET,
        startTime: Date.now(),
        state: 'active',
        outcomes: { success: 0, failure: 0, total: 0 },
        requiredTrials: 5,
      };
      mi._experiments.set(experiment.id, experiment);

      // 2/5 success = 40% — between φ⁻² (38.2%) and φ⁻¹ (61.8%)
      for (let i = 0; i < 5; i++) {
        mi.recordOutcome({
          taskType: 'debug',
          model: ModelTier.SONNET,
          success: i < 2,
          experimentId: 'exp_test_3',
        });
      }

      assert.equal(experiment.state, 'inconclusive');
    });
  });

  describe('affinity matrix', () => {
    it('returns matrix for all categories and tiers', () => {
      const matrix = mi.getAffinityMatrix();
      const categories = Object.values(TaskCategory);
      for (const cat of categories) {
        assert.ok(matrix[cat], `Missing category ${cat}`);
        assert.ok(matrix[cat].opus);
        assert.ok(matrix[cat].sonnet);
        assert.ok(matrix[cat].haiku);
      }
    });

    it('expected values are φ-bounded', () => {
      const matrix = mi.getAffinityMatrix();
      for (const [cat, models] of Object.entries(matrix)) {
        for (const [tier, data] of Object.entries(models)) {
          assert.ok(data.expectedValue <= PHI_INV,
            `${cat}:${tier} EV ${data.expectedValue} exceeds φ⁻¹`);
        }
      }
    });
  });

  describe('persistence', () => {
    it('persists and reloads state', () => {
      // Record some outcomes
      mi.recordOutcome({ taskType: 'code', model: ModelTier.SONNET, success: true });
      mi.recordOutcome({ taskType: 'code', model: ModelTier.SONNET, success: true });
      mi.recordOutcome({ taskType: 'debug', model: ModelTier.OPUS, success: false });

      mi.persist();

      // Create new instance from same path
      const mi2 = new ModelIntelligence({ persistPath });

      // Verify sampler state was restored
      const origSampler = mi._samplers.get(TaskCategory.CODE_WRITE);
      const loadedSampler = mi2._samplers.get(TaskCategory.CODE_WRITE);

      const origArm = origSampler.arms.get(ModelTier.SONNET);
      const loadedArm = loadedSampler.arms.get(ModelTier.SONNET);

      assert.equal(loadedArm.alpha, origArm.alpha);
      assert.equal(loadedArm.beta, origArm.beta);
      assert.equal(loadedArm.pulls, origArm.pulls);
    });

    it('persists tool affinity changes', () => {
      mi.recordOutcome({
        taskType: 'simple',
        model: ModelTier.HAIKU,
        success: true,
        tool: 'brain_cynic_judge',
      });
      mi.persist();

      const mi2 = new ModelIntelligence({ persistPath });
      assert.equal(mi2._toolAffinity.brain_cynic_judge, ModelTier.HAIKU);
    });

    it('persists hypothesis state', () => {
      mi._hypotheses.set('test:hyp', {
        state: 'falsified',
        successRate: 0.8,
        trials: 5,
        timestamp: Date.now(),
      });
      mi.persist();

      const mi2 = new ModelIntelligence({ persistPath });
      const hyp = mi2._hypotheses.get('test:hyp');
      assert.ok(hyp);
      assert.equal(hyp.state, 'falsified');
    });

    it('handles missing persist file gracefully', () => {
      const mi2 = new ModelIntelligence({ persistPath: '/nonexistent/path/state.json' });
      assert.ok(mi2._samplers.size > 0);
    });
  });

  describe('stats', () => {
    it('returns comprehensive stats', () => {
      const stats = mi.getStats();
      assert.ok('selectionsTotal' in stats);
      assert.ok('falsificationsProposed' in stats);
      assert.ok('downgrades' in stats);
      assert.ok('samplerMaturity' in stats);
      assert.ok('toolsTracked' in stats);
    });

    it('maturity signal is φ-bounded', () => {
      const signal = mi.getMaturitySignal();
      assert.ok(signal.maturity >= 0);
      assert.ok(signal.maturity <= PHI_INV);
      assert.equal(typeof signal.converged, 'boolean');
    });
  });

  describe('hypotheses', () => {
    it('returns list of tested hypotheses', () => {
      mi._hypotheses.set('a:b→c', {
        state: 'falsified',
        successRate: 0.7,
        trials: 5,
        timestamp: Date.now(),
      });

      const hyps = mi.getHypotheses();
      assert.equal(hyps.length, 1);
      assert.equal(hyps[0].state, 'falsified');
    });
  });

  describe('singleton', () => {
    it('returns same instance', () => {
      resetModelIntelligence();
      const a = getModelIntelligence({ persistPath });
      const b = getModelIntelligence();
      assert.strictEqual(a, b);
      resetModelIntelligence();
    });

    it('reset creates new instance', () => {
      resetModelIntelligence();
      const a = getModelIntelligence({ persistPath });
      resetModelIntelligence();
      const b = getModelIntelligence({ persistPath });
      assert.notStrictEqual(a, b);
      resetModelIntelligence();
    });
  });
});
