/**
 * MarketWatcher Live Integration Test
 *
 * Tests real $asdfasdfa price fetching from DexScreener/Birdeye/Jupiter.
 *
 * "Truth flows through the market" - κυνικός
 */

'use strict';

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { MarketWatcher } from '../src/perception/market-watcher.js';
import { globalEventBus, EventType } from '@cynic/core';

describe('MarketWatcher - Live Price Feed', () => {
  let watcher;
  const ASDFASDFA_MINT = '9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump';

  before(() => {
    watcher = new MarketWatcher({
      tokenMint: ASDFASDFA_MINT,
      rpcUrl: 'https://api.mainnet-beta.solana.com'
    });
  });

  after(async () => {
    if (watcher) {
      await watcher.stop();
    }
  });

  it('should fetch real $asdfasdfa price from DexScreener', async () => {
    // Fetch price directly (bypasses RPC connection check)
    const price = await watcher._fetchPrice();

    assert.ok(price !== null, 'Price should not be null');
    assert.ok(typeof price === 'number', 'Price should be a number');
    assert.ok(price > 0, 'Price should be positive');

    // Should not be the old mock value (0.00042069)
    assert.ok(
      Math.abs(price - 0.00042069) > 0.0001,
      'Price should not be the hardcoded mock value'
    );

    console.log(`✓ Real $asdfasdfa price fetched: $${price.toFixed(8)}`);
  });

  it('should emit MARKET_PRICE_UPDATED event to globalEventBus', async () => {
    let eventReceived = false;
    let receivedPrice = null;

    const handler = (data) => {
      eventReceived = true;
      receivedPrice = data.price;
    };

    globalEventBus.on(EventType.MARKET_PRICE_UPDATED, handler);

    try {
      const price = await watcher._fetchPrice();

      // Wait for async event emission
      await new Promise(resolve => setTimeout(resolve, 100));

      assert.ok(eventReceived, 'MARKET_PRICE_UPDATED event should be emitted');
      assert.strictEqual(receivedPrice, price, 'Event should contain correct price');

      console.log(`✓ Event emitted with price: $${receivedPrice.toFixed(8)}`);
    } finally {
      globalEventBus.off(EventType.MARKET_PRICE_UPDATED, handler);
    }
  });

  it('should fetch real 24h volume from DexScreener', async () => {
    const volume = await watcher._fetchVolume();

    assert.ok(volume !== null, 'Volume should not be null');
    assert.ok(typeof volume === 'number', 'Volume should be a number');
    assert.ok(volume >= 0, 'Volume should be non-negative');

    // Should not be the old mock value (~150000)
    assert.ok(
      Math.abs(volume - 150000) > 50000 || volume === 0,
      'Volume should not be the hardcoded mock value'
    );

    console.log(`✓ Real 24h volume fetched: $${volume.toFixed(2)}`);
  });

  it('should emit MARKET_VOLUME_UPDATED event to globalEventBus', async () => {
    let eventReceived = false;
    let receivedVolume = null;

    const handler = (data) => {
      eventReceived = true;
      receivedVolume = data.volume24h;
    };

    globalEventBus.on(EventType.MARKET_VOLUME_UPDATED, handler);

    try {
      const volume = await watcher._fetchVolume();

      // Wait for async event emission
      await new Promise(resolve => setTimeout(resolve, 100));

      assert.ok(eventReceived, 'MARKET_VOLUME_UPDATED event should be emitted');
      assert.strictEqual(receivedVolume, volume, 'Event should contain correct volume');

      console.log(`✓ Event emitted with volume: $${receivedVolume.toFixed(2)}`);
    } finally {
      globalEventBus.off(EventType.MARKET_VOLUME_UPDATED, handler);
    }
  });

  it('should handle API failures gracefully', async () => {
    // Create watcher with invalid mint to trigger fallback chain
    const invalidWatcher = new MarketWatcher({
      tokenMint: 'invalid_mint_address_that_does_not_exist_123',
      rpcUrl: 'https://api.mainnet-beta.solana.com'
    });

    const price = await invalidWatcher._fetchPrice();

    // Should return null when all sources fail (not throw)
    assert.strictEqual(price, null, 'Should return null for invalid token');

    console.log('✓ Gracefully handled invalid token (returned null)');
  });
});
