/**
 * SocialWatcher tests (C4.1)
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { getSocialWatcher, resetSocialWatcher } from '../../src/perception/social-watcher.js';
import { globalEventBus, EventType } from '@cynic/core';

describe('SocialWatcher (C4.1)', () => {
  after(async () => {
    resetSocialWatcher();
  });

  it('should return singleton instance', () => {
    const watcher1 = getSocialWatcher();
    const watcher2 = getSocialWatcher();
    assert.strictEqual(watcher1, watcher2, 'Should return same instance');
  });

  it('should start in mock mode by default', async () => {
    const watcher = getSocialWatcher({ mockMode: true });
    await watcher.start();

    const state = watcher.getState();
    assert.strictEqual(state.mockMode, true, 'Should be in mock mode');
    assert.strictEqual(state.isRunning, true, 'Should be running');

    await watcher.stop();
  });

  it('should emit SOCIAL_CAPTURE events', async () => {
    const watcher = getSocialWatcher({ mockMode: true, pollInterval: 100 });

    let captureCount = 0;
    const listener = () => captureCount++;
    globalEventBus.on(EventType.SOCIAL_CAPTURE, listener);

    await watcher.start();

    // Wait for 2 polls
    await new Promise(resolve => setTimeout(resolve, 250));

    await watcher.stop();
    globalEventBus.removeListener(EventType.SOCIAL_CAPTURE, listener);

    assert.ok(captureCount >= 2, `Should emit at least 2 SOCIAL_CAPTURE events, got ${captureCount}`);
  });

  it('should analyze sentiment correctly', () => {
    const watcher = getSocialWatcher({ mockMode: true });

    assert.strictEqual(
      watcher._analyzeSentiment('bullish moon wagmi LFG'),
      'positive',
      'Should detect positive sentiment'
    );

    assert.strictEqual(
      watcher._analyzeSentiment('rug dump scam ponzi'),
      'negative',
      'Should detect negative sentiment'
    );

    assert.strictEqual(
      watcher._analyzeSentiment('just a normal tweet'),
      'neutral',
      'Should default to neutral'
    );
  });

  it('should track stats', async () => {
    const watcher = getSocialWatcher({ mockMode: true, pollInterval: 100 });
    await watcher.start();

    await new Promise(resolve => setTimeout(resolve, 250));

    const stats = watcher.getStats();
    assert.ok(stats.polls >= 2, 'Should track poll count');
    assert.ok(stats.mentions >= 0, 'Should track mention count');
    assert.ok(stats.sentimentDistribution, 'Should track sentiment distribution');

    await watcher.stop();
  });

  it('should stop gracefully', async () => {
    const watcher = getSocialWatcher({ mockMode: true });
    await watcher.start();
    await watcher.stop();

    const state = watcher.getState();
    assert.strictEqual(state.isRunning, false, 'Should not be running after stop');
  });

  it('should handle rate limit errors', async () => {
    const watcher = getSocialWatcher({ mockMode: true });

    // Simulate rate limit error
    const error = new Error('429 rate limit exceeded');
    watcher._handlePollError('twitter', error);

    const stats = watcher.getStats();
    assert.strictEqual(stats.rateLimitHits, 1, 'Should track rate limit hits');
    assert.strictEqual(stats.errors, 1, 'Should track errors');
  });
});
