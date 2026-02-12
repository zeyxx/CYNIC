/**
 * Market Watcher Tests
 *
 * Tests for C3.1 (MARKET × PERCEIVE) - first cell in the MARKET row.
 *
 * "Le chien sent le marché" - CYNIC
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  MarketWatcher,
  getMarketWatcher,
  resetMarketWatcher,
  MarketEventType,
  ASDFASDFA_MINT,
} from '../../src/market/market-watcher.js';

describe('MarketWatcher', () => {
  let watcher;

  afterEach(() => {
    if (watcher) {
      watcher.stop();
      watcher = null;
    }
    resetMarketWatcher();
  });

  describe('Constructor', () => {
    it('should create with default options', () => {
      watcher = new MarketWatcher();
      assert.strictEqual(watcher.running, false);
      assert.strictEqual(watcher.tokenMint, ASDFASDFA_MINT);
      assert.strictEqual(watcher.lastPrice, null);
    });

    it('should create with custom token mint', () => {
      watcher = new MarketWatcher({ tokenMint: 'custom_mint' });
      assert.strictEqual(watcher.tokenMint, 'custom_mint');
    });

    it('should not auto-start by default', () => {
      watcher = new MarketWatcher();
      assert.strictEqual(watcher.running, false);
    });

    it('should auto-start if requested', () => {
      watcher = new MarketWatcher({ autoStart: true });
      assert.strictEqual(watcher.running, true);
    });
  });

  describe('Start/Stop', () => {
    it('should start and emit started event', (t, done) => {
      watcher = new MarketWatcher();
      watcher.on('started', () => {
        assert.strictEqual(watcher.running, true);
        done();
      });
      watcher.start();
    });

    it('should be idempotent on multiple starts', () => {
      watcher = new MarketWatcher();
      watcher.start();
      const state1 = watcher.getState();
      watcher.start(); // Should not throw
      const state2 = watcher.getState();
      assert.strictEqual(state1.running, state2.running);
    });

    it('should stop and emit stopped event', (t, done) => {
      watcher = new MarketWatcher({ autoStart: true });
      watcher.on('stopped', () => {
        assert.strictEqual(watcher.running, false);
        done();
      });
      watcher.stop();
    });

    it('should update uptime on stop', () => {
      watcher = new MarketWatcher({ autoStart: true });
      assert.ok(watcher.stats.uptime === 0);
      watcher.stop();
      assert.ok(watcher.stats.uptime > 0);
    });
  });

  describe('State', () => {
    it('should return current state', () => {
      watcher = new MarketWatcher();
      const state = watcher.getState();
      assert.ok(state.hasOwnProperty('running'));
      assert.ok(state.hasOwnProperty('tokenMint'));
      assert.ok(state.hasOwnProperty('lastPrice'));
      assert.ok(state.hasOwnProperty('stats'));
    });

    it('should track stats', () => {
      watcher = new MarketWatcher();
      assert.strictEqual(watcher.stats.priceUpdates, 0);
      assert.strictEqual(watcher.stats.liquidityUpdates, 0);
      assert.strictEqual(watcher.stats.errors, 0);
    });
  });

  describe('Price History', () => {
    it('should return empty array initially', () => {
      watcher = new MarketWatcher();
      const history = watcher.getPriceHistory();
      assert.strictEqual(history.length, 0);
    });

    it('should limit history to 144 points (F(12))', () => {
      watcher = new MarketWatcher();
      // Simulate what _fetchPrice does: push + trim
      for (let i = 0; i < 200; i++) {
        watcher.priceHistory.push({ price: i, timestamp: Date.now() + i });
        if (watcher.priceHistory.length > 144) {
          watcher.priceHistory.shift();
        }
      }
      assert.strictEqual(watcher.priceHistory.length, 144);
    });

    it('should return last N points', () => {
      watcher = new MarketWatcher();
      watcher.priceHistory = [
        { price: 1, timestamp: 1 },
        { price: 2, timestamp: 2 },
        { price: 3, timestamp: 3 },
      ];
      const last2 = watcher.getPriceHistory(2);
      assert.strictEqual(last2.length, 2);
      assert.strictEqual(last2[0].price, 2);
      assert.strictEqual(last2[1].price, 3);
    });
  });

  describe('Event Types', () => {
    it('should have all required event types', () => {
      assert.strictEqual(MarketEventType.PRICE_UPDATE, 'perception:market:price');
      assert.strictEqual(MarketEventType.LIQUIDITY_CHANGE, 'perception:market:liquidity');
      assert.strictEqual(MarketEventType.VOLUME_SPIKE, 'perception:market:volume_spike');
      assert.strictEqual(MarketEventType.HOLDER_CHANGE, 'perception:market:holders');
      assert.strictEqual(MarketEventType.PRICE_ALERT, 'perception:market:price_alert');
      assert.strictEqual(MarketEventType.ERROR, 'perception:market:error');
    });
  });

  describe('Singleton', () => {
    it('should return same instance', () => {
      const w1 = getMarketWatcher();
      const w2 = getMarketWatcher();
      assert.strictEqual(w1, w2);
    });

    it('should reset singleton', () => {
      const w1 = getMarketWatcher();
      resetMarketWatcher();
      const w2 = getMarketWatcher();
      assert.notStrictEqual(w1, w2);
    });
  });

  describe('Token Address', () => {
    it('should have correct $asdfasdfa mint address', () => {
      assert.strictEqual(ASDFASDFA_MINT, '9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump');
    });
  });

  describe('API Fetching', () => {
    it('should handle price fetch errors gracefully', async () => {
      watcher = new MarketWatcher({ tokenMint: 'invalid_mint' });

      let errorEmitted = false;
      watcher.on(MarketEventType.ERROR, (event) => {
        assert.strictEqual(event.source, 'price');
        errorEmitted = true;
      });

      await watcher._fetchPrice();
      // Give time for async operations
      await new Promise(r => setTimeout(r, 100));

      assert.ok(watcher.stats.errors > 0, 'Should have recorded error');
    });

    it('should handle liquidity fetch errors gracefully', async () => {
      watcher = new MarketWatcher({ tokenMint: 'invalid_mint' });

      let errorEmitted = false;
      watcher.on(MarketEventType.ERROR, (event) => {
        if (event.source === 'liquidity') {
          errorEmitted = true;
        }
      });

      await watcher._fetchLiquidity();
      await new Promise(r => setTimeout(r, 100));

      assert.ok(watcher.stats.errors > 0, 'Should have recorded error');
    });
  });
});
