/**
 * CostLedger Tests
 *
 * "Le chien vérifie ses propres comptes" — κυνικός
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'path';
import { mkdirSync, writeFileSync, existsSync, unlinkSync } from 'fs';
import { tmpdir } from 'os';

import {
  CostLedger,
  ModelId,
  BudgetStatus,
  getCostLedger,
  resetCostLedger,
} from '../src/accounting/cost-ledger.js';

// Temp path for isolation (lesson #34: shared singletons clobber tests)
function tempPersistPath() {
  const dir = join(tmpdir(), 'cynic-test-cost-' + Date.now() + '-' + Math.random().toString(36).slice(2, 8));
  mkdirSync(dir, { recursive: true });
  return join(dir, 'ledger-state.json');
}

describe('CostLedger', () => {
  let ledger;
  let persistPath;

  beforeEach(() => {
    resetCostLedger();
    persistPath = tempPersistPath();
    ledger = new CostLedger({ persistPath });
  });

  afterEach(() => {
    if (ledger) ledger.removeAllListeners();
    resetCostLedger();
    try { if (existsSync(persistPath)) unlinkSync(persistPath); } catch { /* noop */ }
  });

  // ===========================================================================
  // CONSTRUCTION
  // ===========================================================================

  describe('construction', () => {
    it('should initialize with default values', () => {
      assert.equal(ledger._currentModel, ModelId.OPUS);
      assert.equal(ledger._sessionBudget, null);
      assert.equal(ledger._session.operations, 0);
      assert.equal(ledger._session.totalTokens, 0);
    });

    it('should accept custom model', () => {
      const l = new CostLedger({ model: ModelId.HAIKU, persistPath: tempPersistPath() });
      assert.equal(l._currentModel, ModelId.HAIKU);
      l.removeAllListeners();
    });

    it('should accept session budget', () => {
      const l = new CostLedger({ sessionBudget: 1_000_000, persistPath: tempPersistPath() });
      assert.equal(l._sessionBudget, 1_000_000);
      l.removeAllListeners();
    });
  });

  // ===========================================================================
  // RECORD
  // ===========================================================================

  describe('record()', () => {
    it('should record an operation with token estimates', () => {
      const result = ledger.record({
        type: 'tool_call',
        inputText: 'Hello world', // 11 chars ≈ 3 tokens
        outputText: 'Goodbye world', // 13 chars ≈ 4 tokens
        source: 'test',
      });

      assert.ok(result);
      assert.equal(result.type, 'tool_call');
      assert.equal(result.model, ModelId.OPUS);
      assert.equal(result.inputTokens, 3); // ceil(11/4)
      assert.equal(result.outputTokens, 4); // ceil(13/4)
      assert.equal(result.totalTokens, 7);
      assert.ok(result.cost.total > 0);
      assert.ok(result.timestamp > 0);
    });

    it('should record with explicit token counts', () => {
      const result = ledger.record({
        type: 'judgment',
        inputTokens: 1000,
        outputTokens: 500,
      });

      assert.equal(result.inputTokens, 1000);
      assert.equal(result.outputTokens, 500);
      assert.equal(result.totalTokens, 1500);
    });

    it('should update session totals', () => {
      ledger.record({ type: 'a', inputTokens: 100, outputTokens: 50 });
      ledger.record({ type: 'b', inputTokens: 200, outputTokens: 100 });

      assert.equal(ledger._session.operations, 2);
      assert.equal(ledger._session.inputTokens, 300);
      assert.equal(ledger._session.outputTokens, 150);
      assert.equal(ledger._session.totalTokens, 450);
    });

    it('should track by model', () => {
      ledger.record({ type: 'a', model: ModelId.OPUS, inputTokens: 100 });
      ledger.record({ type: 'b', model: ModelId.HAIKU, inputTokens: 200 });
      ledger.record({ type: 'c', model: ModelId.OPUS, inputTokens: 300 });

      assert.equal(ledger._session.byModel[ModelId.OPUS].operations, 2);
      assert.equal(ledger._session.byModel[ModelId.OPUS].tokens, 400);
      assert.equal(ledger._session.byModel[ModelId.HAIKU].operations, 1);
      assert.equal(ledger._session.byModel[ModelId.HAIKU].tokens, 200);
    });

    it('should track by operation type', () => {
      ledger.record({ type: 'tool_call', inputTokens: 100 });
      ledger.record({ type: 'tool_call', inputTokens: 200 });
      ledger.record({ type: 'judgment', inputTokens: 150 });

      assert.equal(ledger._session.byOperation.tool_call.operations, 2);
      assert.equal(ledger._session.byOperation.judgment.operations, 1);
    });

    it('should emit cost:recorded event', () => {
      let emitted = null;
      ledger.on('cost:recorded', (r) => { emitted = r; });

      ledger.record({ type: 'test', inputTokens: 10 });

      assert.ok(emitted);
      assert.equal(emitted.type, 'test');
    });

    it('should calculate cost using Opus rates by default', () => {
      const result = ledger.record({
        type: 'test',
        inputTokens: 1_000_000, // 1M tokens
        outputTokens: 0,
      });

      // Opus input: $5 per 1M tokens (updated 2026-02-11)
      assert.ok(Math.abs(result.cost.input - 5) < 0.01);
    });

    it('should calculate cost using Haiku rates', () => {
      const result = ledger.record({
        type: 'test',
        model: ModelId.HAIKU,
        inputTokens: 1_000_000,
        outputTokens: 0,
      });

      // Haiku input: $1.00 per 1M tokens (updated 2026-02-11)
      assert.ok(Math.abs(result.cost.input - 1.0) < 0.01);
    });

    it('should calculate Ollama as free', () => {
      const result = ledger.record({
        type: 'test',
        model: ModelId.OLLAMA,
        inputTokens: 1_000_000,
        outputTokens: 1_000_000,
      });

      assert.equal(result.cost.total, 0);
    });

    it('should handle empty input gracefully', () => {
      const result = ledger.record({ type: 'empty' });

      assert.equal(result.inputTokens, 0);
      assert.equal(result.outputTokens, 0);
      assert.equal(result.totalTokens, 0);
      assert.equal(result.cost.total, 0);
    });
  });

  // ===========================================================================
  // BURN RATE
  // ===========================================================================

  describe('getBurnRate()', () => {
    it('should return zeros when insufficient data', () => {
      const rate = ledger.getBurnRate();
      assert.equal(rate.tokensPerMinute, 0);
      assert.equal(rate.velocity, 0);
      assert.equal(rate.trend, 'stable');
    });

    it('should calculate rate from window', () => {
      // Simulate 10 ops over ~1 minute (faked timestamps in burn window)
      const now = Date.now();
      for (let i = 0; i < 10; i++) {
        ledger._burnWindow.push({
          timestamp: now - (60000 - i * 6000), // spread over 1 minute
          tokens: 100,
          cost: 0.001,
        });
      }

      const rate = ledger.getBurnRate();
      assert.ok(rate.tokensPerMinute > 0);
      assert.ok(rate.samples === 10);
    });

    it('should detect acceleration', () => {
      const now = Date.now();
      // First half: small operations
      for (let i = 0; i < 5; i++) {
        ledger._burnWindow.push({
          timestamp: now - 120000 + i * 12000,
          tokens: 10,
          cost: 0.0001,
        });
      }
      // Second half: large operations
      for (let i = 0; i < 5; i++) {
        ledger._burnWindow.push({
          timestamp: now - 60000 + i * 12000,
          tokens: 100,
          cost: 0.001,
        });
      }

      const rate = ledger.getBurnRate();
      assert.equal(rate.trend, 'accelerating');
    });

    it('should φ-bound velocity to 0.618', () => {
      const now = Date.now();
      // Extremely fast burn: 1M tokens over 1 minute → 1M/min
      // velocity = phiBound(1_000_000 / 10_000) = phiBound(100) = 0.618
      for (let i = 0; i < 10; i++) {
        ledger._burnWindow.push({ timestamp: now - 60000 + i * 6000, tokens: 100_000, cost: 1 });
      }

      const rate = ledger.getBurnRate();
      assert.ok(rate.velocity > 0, `velocity should be > 0, got ${rate.velocity}`);
      assert.ok(rate.velocity <= 0.619, `velocity should be φ-bounded, got ${rate.velocity}`);
    });
  });

  // ===========================================================================
  // BUDGET STATUS
  // ===========================================================================

  describe('getBudgetStatus()', () => {
    it('should return abundant when no budget set', () => {
      const status = ledger.getBudgetStatus();
      assert.equal(status.level, BudgetStatus.ABUNDANT);
      assert.equal(status.budget, null);
    });

    it('should classify consumption levels (φ-derived)', () => {
      ledger.setSessionBudget(100_000);

      // < 38.2% = ABUNDANT
      ledger._session.totalTokens = 30_000;
      assert.equal(ledger.getBudgetStatus().level, BudgetStatus.ABUNDANT);

      // < 61.8% = MODERATE
      ledger._session.totalTokens = 50_000;
      assert.equal(ledger.getBudgetStatus().level, BudgetStatus.MODERATE);

      // < 80% = CAUTIOUS
      ledger._session.totalTokens = 75_000;
      assert.equal(ledger.getBudgetStatus().level, BudgetStatus.CAUTIOUS);

      // < 95% = CRITICAL
      ledger._session.totalTokens = 90_000;
      assert.equal(ledger.getBudgetStatus().level, BudgetStatus.CRITICAL);

      // >= 95% = EXHAUSTED
      ledger._session.totalTokens = 96_000;
      assert.equal(ledger.getBudgetStatus().level, BudgetStatus.EXHAUSTED);
    });

    it('should calculate time-to-limit', () => {
      ledger.setSessionBudget(100_000);
      ledger._session.totalTokens = 50_000;

      // Fake burn window: 1000 tokens/min
      const now = Date.now();
      ledger._burnWindow.push({ timestamp: now - 60000, tokens: 500, cost: 0 });
      ledger._burnWindow.push({ timestamp: now, tokens: 500, cost: 0 });

      const status = ledger.getBudgetStatus();
      assert.ok(status.timeToLimitMinutes !== null);
      assert.ok(status.timeToLimitMinutes > 0);
    });
  });

  // ===========================================================================
  // MODEL RECOMMENDATION
  // ===========================================================================

  describe('recommendModel()', () => {
    it('should recommend Haiku for simple tasks', () => {
      const rec = ledger.recommendModel({ taskType: 'simple' });
      assert.equal(rec.model, ModelId.HAIKU);
    });

    it('should recommend Opus for complex reasoning', () => {
      const rec = ledger.recommendModel({ taskType: 'complex', needsReasoning: true });
      assert.equal(rec.model, ModelId.OPUS);
    });

    it('should downgrade on critical budget', () => {
      ledger.setSessionBudget(100);
      ledger._session.totalTokens = 91; // > 80% = CRITICAL

      const rec = ledger.recommendModel({ taskType: 'complex', needsReasoning: true });
      assert.equal(rec.model, ModelId.SONNET);
      assert.ok(rec.reason.includes('budget critical'));
    });

    it('should force Haiku on exhausted budget', () => {
      ledger.setSessionBudget(100);
      ledger._session.totalTokens = 96; // > 95% = EXHAUSTED

      const rec = ledger.recommendModel({ taskType: 'complex', needsReasoning: true });
      assert.equal(rec.model, ModelId.HAIKU);
    });

    it('should downgrade on high burn velocity', () => {
      // Make velocity > φ⁻¹ (need >10k tokens/min)
      // 20 entries of 1000 tokens over 1 minute = 20000 tokens/min
      const now = Date.now();
      for (let i = 0; i < 20; i++) {
        ledger._burnWindow.push({ timestamp: now - 60000 + i * 3000, tokens: 1000, cost: 0.01 });
      }

      const burnRate = ledger.getBurnRate();
      assert.ok(burnRate.velocity > 0.618, `velocity ${burnRate.velocity} should be > φ⁻¹`);

      const rec = ledger.recommendModel({ taskType: 'complex', needsReasoning: true });
      assert.equal(rec.model, ModelId.SONNET); // Downgraded from Opus
    });

    it('should φ-bound confidence to 0.618', () => {
      const rec = ledger.recommendModel();
      assert.ok(rec.confidence <= 0.618);
    });
  });

  // ===========================================================================
  // COST ESTIMATION
  // ===========================================================================

  describe('estimate()', () => {
    it('should estimate from text', () => {
      const est = ledger.estimate({ inputText: 'Hello world' }); // 11 chars
      assert.equal(est.inputTokens, 3); // ceil(11/4)
      assert.ok(est.estimatedCost >= 0);
    });

    it('should estimate from explicit tokens', () => {
      const est = ledger.estimate({ inputTokens: 5000, outputTokens: 1000 });
      assert.equal(est.totalTokens, 6000);
    });

    it('should show budget impact', () => {
      ledger.setSessionBudget(100_000);
      const est = ledger.estimate({ inputTokens: 10_000 });
      assert.ok(est.budgetImpact !== null);
      assert.ok(Math.abs(est.budgetImpact - 0.1) < 0.001);
    });

    it('should return null budgetImpact when no budget', () => {
      const est = ledger.estimate({ inputTokens: 10_000 });
      assert.equal(est.budgetImpact, null);
    });
  });

  // ===========================================================================
  // SESSION SUMMARY
  // ===========================================================================

  describe('getSessionSummary()', () => {
    it('should return complete session data', () => {
      ledger.record({ type: 'a', inputTokens: 100, outputTokens: 50 });
      ledger.record({ type: 'b', inputTokens: 200, outputTokens: 100 });

      const summary = ledger.getSessionSummary();

      assert.equal(summary.operations, 2);
      assert.equal(summary.tokens.input, 300);
      assert.equal(summary.tokens.output, 150);
      assert.equal(summary.tokens.total, 450);
      assert.ok(summary.cost.total > 0);
      assert.ok(summary.burnRate);
      assert.ok(summary.budget);
      assert.equal(summary.currentModel, ModelId.OPUS);
    });
  });

  // ===========================================================================
  // MODEL MANAGEMENT
  // ===========================================================================

  describe('model management', () => {
    it('should change current model', () => {
      ledger.setModel(ModelId.SONNET);
      assert.equal(ledger._currentModel, ModelId.SONNET);
    });

    it('should ignore invalid model', () => {
      ledger.setModel('invalid_model');
      assert.equal(ledger._currentModel, ModelId.OPUS); // unchanged
    });

    it('should return current model info', () => {
      const info = ledger.getCurrentModel();
      assert.equal(info.id, ModelId.OPUS);
      assert.ok(info.label.includes('Opus'));
      assert.equal(info.inputPer1M, 5); // Updated 2026-02-11
      assert.equal(info.outputPer1M, 25); // Updated 2026-02-11
    });
  });

  // ===========================================================================
  // PERSISTENCE
  // ===========================================================================

  describe('persistence', () => {
    it('should persist lifetime stats to disk', () => {
      ledger.record({ type: 'test', inputTokens: 1000 });
      ledger.persist();

      assert.ok(existsSync(persistPath));
    });

    it('should load persisted state on construction', () => {
      // Write fake state
      const dir = join(tmpdir(), 'cynic-test-persist-' + Date.now());
      mkdirSync(dir, { recursive: true });
      const path = join(dir, 'ledger-state.json');
      writeFileSync(path, JSON.stringify({
        totalOperations: 42,
        totalTokens: 100000,
        totalCostUSD: 5.50,
        sessions: 7,
        firstSeen: 1700000000000,
        lastActivity: 1700000000000,
      }));

      const l = new CostLedger({ persistPath: path });
      assert.equal(l._lifetime.totalOperations, 42);
      assert.equal(l._lifetime.totalTokens, 100000);
      assert.equal(l._lifetime.sessions, 7);
      l.removeAllListeners();
    });

    it('should handle missing persist file gracefully', () => {
      const l = new CostLedger({
        persistPath: join(tmpdir(), 'nonexistent-' + Date.now(), 'state.json'),
      });
      assert.equal(l._lifetime.totalOperations, 0);
      l.removeAllListeners();
    });

    it('should handle corrupt persist file gracefully', () => {
      const dir = join(tmpdir(), 'cynic-test-corrupt-' + Date.now());
      mkdirSync(dir, { recursive: true });
      const path = join(dir, 'ledger-state.json');
      writeFileSync(path, 'not json!!!');

      const l = new CostLedger({ persistPath: path });
      assert.equal(l._lifetime.totalOperations, 0);
      l.removeAllListeners();
    });

    it('should increment sessions on endSession', () => {
      ledger.endSession();
      assert.equal(ledger._lifetime.sessions, 1);
    });
  });

  // ===========================================================================
  // LIFETIME STATS
  // ===========================================================================

  describe('getLifetimeStats()', () => {
    it('should aggregate across records', () => {
      ledger.record({ type: 'a', inputTokens: 100 });
      ledger.record({ type: 'b', inputTokens: 200 });

      const stats = ledger.getLifetimeStats();
      assert.equal(stats.totalOperations, 2);
      assert.equal(stats.totalTokens, 300);
      assert.ok(stats.totalCostUSD > 0);
    });
  });

  // ===========================================================================
  // BUDGET ALERTS
  // ===========================================================================

  describe('budget alerts', () => {
    it('should emit budget:moderate at φ⁻¹ threshold', () => {
      ledger.setSessionBudget(1000);
      let emitted = false;
      ledger.on('budget:moderate', () => { emitted = true; });

      // Push past 61.8%
      ledger.record({ type: 'test', inputTokens: 700 });
      assert.ok(emitted);
    });

    it('should emit budget:critical at 80% threshold', () => {
      ledger.setSessionBudget(1000);
      let emitted = false;
      ledger.on('budget:critical', () => { emitted = true; });

      ledger.record({ type: 'test', inputTokens: 850 });
      assert.ok(emitted);
    });

    it('should emit budget:exhausted at 95% threshold', () => {
      ledger.setSessionBudget(1000);
      let emitted = false;
      ledger.on('budget:exhausted', () => { emitted = true; });

      ledger.record({ type: 'test', inputTokens: 960 });
      assert.ok(emitted);
    });

    it('should not emit alerts without budget', () => {
      let emitted = false;
      ledger.on('budget:moderate', () => { emitted = true; });
      ledger.on('budget:critical', () => { emitted = true; });
      ledger.on('budget:exhausted', () => { emitted = true; });

      ledger.record({ type: 'test', inputTokens: 999999 });
      assert.ok(!emitted);
    });
  });

  // ===========================================================================
  // HISTORY
  // ===========================================================================

  describe('getHistory()', () => {
    it('should return recent operations', () => {
      for (let i = 0; i < 5; i++) {
        ledger.record({ type: `op_${i}`, inputTokens: 10 });
      }

      const history = ledger.getHistory();
      assert.equal(history.length, 5);
    });

    it('should respect limit parameter', () => {
      for (let i = 0; i < 10; i++) {
        ledger.record({ type: `op_${i}`, inputTokens: 10 });
      }

      const history = ledger.getHistory(3);
      assert.equal(history.length, 3);
    });

    it('should trim to maxHistory', () => {
      const smallLedger = new CostLedger({ maxHistory: 5, persistPath: tempPersistPath() });
      for (let i = 0; i < 10; i++) {
        smallLedger.record({ type: `op_${i}`, inputTokens: 10 });
      }

      assert.equal(smallLedger._history.length, 5);
      smallLedger.removeAllListeners();
    });
  });

  // ===========================================================================
  // SESSION RESET
  // ===========================================================================

  describe('resetSession()', () => {
    it('should clear all session state', () => {
      ledger.record({ type: 'test', inputTokens: 1000 });
      ledger.resetSession();

      assert.equal(ledger._session.operations, 0);
      assert.equal(ledger._session.totalTokens, 0);
      assert.equal(ledger._burnWindow.length, 0);
      assert.equal(ledger._history.length, 0);
    });

    it('should emit session:reset event', () => {
      let emitted = false;
      ledger.on('session:reset', () => { emitted = true; });
      ledger.resetSession();
      assert.ok(emitted);
    });
  });

  // ===========================================================================
  // SINGLETON
  // ===========================================================================

  describe('singleton', () => {
    it('should return same instance', () => {
      resetCostLedger();
      const a = getCostLedger({ persistPath: tempPersistPath() });
      const b = getCostLedger();
      assert.equal(a, b);
      resetCostLedger();
    });

    it('should create new instance after reset', () => {
      resetCostLedger();
      const a = getCostLedger({ persistPath: tempPersistPath() });
      resetCostLedger();
      const b = getCostLedger({ persistPath: tempPersistPath() });
      assert.notEqual(a, b);
      resetCostLedger();
    });
  });

  // ===========================================================================
  // TOKEN ESTIMATION
  // ===========================================================================

  describe('token estimation', () => {
    it('should estimate ~4 chars per token', () => {
      assert.equal(ledger._estimateTokens('1234'), 1);
      assert.equal(ledger._estimateTokens('12345'), 2);
      assert.equal(ledger._estimateTokens('12345678'), 2);
    });

    it('should handle null/undefined text', () => {
      assert.equal(ledger._estimateTokens(null), 0);
      assert.equal(ledger._estimateTokens(undefined), 0);
      assert.equal(ledger._estimateTokens(''), 0);
    });

    it('should handle non-string input', () => {
      assert.equal(ledger._estimateTokens(42), 0);
      assert.equal(ledger._estimateTokens({}), 0);
    });
  });

  // ===========================================================================
  // CUSTOM COST RATES
  // ===========================================================================

  describe('setCostRates()', () => {
    it('should override specific model rates', () => {
      ledger.setCostRates({
        [ModelId.OPUS]: { input: 20, output: 100, label: 'Expensive Opus' },
      });

      const result = ledger.record({
        type: 'test',
        model: ModelId.OPUS,
        inputTokens: 1_000_000,
      });

      assert.ok(Math.abs(result.cost.input - 20) < 0.01);
    });
  });
});
