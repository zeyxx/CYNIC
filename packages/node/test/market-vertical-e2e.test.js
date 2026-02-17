/**
 * Market Vertical Stack E2E Test
 *
 * Tests FULL verticality: PERCEIVE → JUDGE → DECIDE → ACT
 * C3.1 → C3.2 → C3.3 → C3.4
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';

// Import Market components
import { MarketWatcher } from '../src/perception/market-watcher.js';
import { getMarketJudge, resetMarketJudge } from '../src/market/market-judge.js';
import { getMarketDecider, resetMarketDecider } from '../src/market/market-decider.js';
import { getMarketActor, resetMarketActor } from '../src/market/market-actor.js';

describe('Market Vertical Stack E2E', () => {
  let watcher;
  let judge;
  let decider;
  let actor;

  before(async () => {
    // Initialize components
    watcher = new MarketWatcher({ mockMode: true, pollInterval: 100 });
    judge = getMarketJudge();
    decider = getMarketDecider();
    actor = getMarketActor();

    // Start watcher
    await watcher.start();
  });

  after(async () => {
    await watcher.stop();
    resetMarketJudge();
    resetMarketDecider();
    resetMarketActor();
  });

  it('C3.1 (PERCEIVE): MarketWatcher fetches price', async () => {
    // Wait for first poll
    await new Promise(resolve => setTimeout(resolve, 150));

    const state = watcher.getState();
    assert.ok(state.lastPrice !== null, 'Should have fetched a price');
    assert.ok(state.lastPrice > 0, 'Price should be positive');
  });

  it('C3.2 (JUDGE): MarketJudge scores price movement', async () => {
    const marketData = {
      price: 0.00042,
      priceChange24h: -0.15, // -15% drop
      volume: 1500,
      timestamp: Date.now(),
    };

    const judgment = await judge.judge(marketData);

    assert.ok(judgment, 'Should return judgment');
    assert.ok(judgment.score !== undefined, 'Should have score');
    assert.ok(judgment.verdict !== undefined, 'Should have verdict');
    assert.ok(judgment.confidence <= 0.618, 'Confidence should be φ-bounded');
  });

  it('C3.3 (DECIDE): MarketDecider makes trading decision', async () => {
    const marketData = {
      price: 0.00042,
      priceChange24h: -0.25, // -25% drop (buy signal territory)
      volume: 2000,
      liquidity: 15000,
      sentiment: 'positive',
    };

    const decision = await decider.decide(marketData);

    assert.ok(decision, 'Should return decision');
    assert.ok(decision.decision, 'Should have decision type');
    assert.ok(decision.confidence <= 0.618, 'Confidence should be φ-bounded');
    assert.ok(decision.reason, 'Should have reason');
  });

  it('C3.4 (ACT): MarketActor executes action', async () => {
    const decision = {
      decision: 'buy_signal',
      confidence: 0.55,
      severity: 'high',
      reason: 'Test buy signal',
      factors: {
        price: 0.0004,
        priceChange24h: -0.25,
        volume: 2000,
        liquidity: 15000,
        sentiment: 'positive',
      },
    };

    const action = await actor.act(decision);

    assert.ok(action, 'Should return action result');
    assert.strictEqual(action.type, 'log_buy', 'Should be buy action');
    assert.strictEqual(action.status, 'delivered', 'Should be delivered');
    assert.ok(action.trade, 'Should have trade details');
    assert.ok(action.trade.hypothetical === true, 'Should be hypothetical');
  });

  it('FULL VERTICAL: PERCEIVE → JUDGE → DECIDE → ACT', async () => {
    // C3.1: Get market data from watcher
    await new Promise(resolve => setTimeout(resolve, 150));
    const state = watcher.getState();
    assert.ok(state.lastPrice, 'C3.1: Price perceived');

    // C3.2: Judge the market data
    const marketData = {
      price: state.lastPrice,
      priceChange24h: -0.30, // Strong drop
      volume: 3000,
      liquidity: 20000,
      sentiment: 'positive',
    };
    const judgment = await judge.judge(marketData);
    assert.ok(judgment.score !== undefined, 'C3.2: Market judged');

    // C3.3: Make decision
    const decision = await decider.decide(marketData);
    assert.ok(decision.decision, 'C3.3: Decision made');

    // C3.4: Execute action
    const action = await actor.act(decision);
    assert.ok(action.status === 'delivered', 'C3.4: Action executed');

    // Verify end-to-end data flow
    assert.ok(
      state.lastPrice === marketData.price,
      'Data flows from PERCEIVE to DECIDE'
    );
    assert.ok(
      decision.factors.price === action.trade?.price,
      'Data flows from DECIDE to ACT'
    );
  });

  it('Portfolio tracking: hypothetical P&L calculation', async () => {
    // Get initial portfolio
    const initialPortfolio = actor.getPortfolio();
    const initialSol = initialPortfolio.sol;

    // Execute buy
    const buyDecision = {
      decision: 'buy_signal',
      confidence: 0.5,
      severity: 'high',
      reason: 'Test',
      factors: { price: 0.001 },
    };
    await actor.act(buyDecision);

    // Verify SOL decreased, tokens increased
    const afterBuy = actor.getPortfolio();
    assert.ok(afterBuy.sol < initialSol, 'SOL should decrease after buy');
    assert.ok(afterBuy.tokens > 0, 'Tokens should increase after buy');

    // Execute sell at higher price (profit)
    const sellDecision = {
      decision: 'sell_signal',
      confidence: 0.5,
      severity: 'high',
      reason: 'Test',
      factors: { price: 0.0015 }, // 50% profit
    };
    await actor.act(sellDecision);

    // Verify profit recorded
    const afterSell = actor.getPortfolio();
    const pnl = actor.getHypotheticalPnL();
    assert.ok(pnl > 0, 'Should have positive P&L from profitable trade');
  });

  it('Decision history and cooldowns work', async () => {
    const marketData = {
      price: 0.0004,
      priceChange24h: -0.65, // Crash
      volume: 5000,
      liquidity: 10000,
      sentiment: 'negative',
    };

    // First alert
    const decision1 = await decider.decide(marketData);
    assert.strictEqual(decision1.decision, 'alert', 'Should trigger alert');

    // Immediate second alert (should be on cooldown)
    const decision2 = await decider.decide(marketData);
    assert.notStrictEqual(decision2.decision, 'alert', 'Should be on cooldown');

    // Verify stats
    const stats = decider.getStats();
    assert.ok(stats.decisionHistorySize >= 2, 'Should track decision history');
  });
});
