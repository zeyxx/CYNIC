/**
 * E-Score Provider Bridge
 *
 * PHASE 2: DECENTRALIZE
 *
 * Bridges globalEventBus events into an EScore7DCalculator instance,
 * providing a real-time dynamic eScoreProvider for ValidatorManager.
 *
 * Feeds: BURN, BUILD, JUDGE, RUN (from events)
 * Stubs: SOCIAL, GRAPH, HOLD (Phase 3)
 *
 * "The pack knows its own strength" - κυνικός
 *
 * @module @cynic/node/network/escore-provider
 */

'use strict';

import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { createEScore7DCalculator } from '@cynic/identity';

const log = createLogger('EScoreProvider');

/**
 * Create a live E-Score provider backed by EScore7DCalculator.
 *
 * Subscribes to globalEventBus events and feeds them into the calculator.
 * Returns a provider function suitable for ValidatorManager.eScoreProvider.
 *
 * @param {Object} options
 * @param {string} options.selfPublicKey - This node's public key
 * @returns {{ provider: (publicKey: string) => number|null, calculator: EScore7DCalculator, destroy: () => void }}
 */
export function createEScoreProvider({ selfPublicKey }) {
  const calc = createEScore7DCalculator();

  // --- Event subscriptions ---

  const subscriptions = [];

  // JUDGE: judgment:created → recordJudgment
  subscriptions.push(
    globalEventBus.subscribe(EventType.JUDGMENT_CREATED, (event) => {
      const { consensus, matchedConsensus } = event.payload || {};
      calc.recordJudgment(matchedConsensus ?? consensus ?? false);
    })
  );

  // RUN: block:finalized → recordBlock + heartbeat
  subscriptions.push(
    globalEventBus.subscribe(EventType.BLOCK_FINALIZED, () => {
      calc.recordBlock();
      calc.heartbeat();
    })
  );

  // RUN: block:proposed → heartbeat (we're alive and producing)
  subscriptions.push(
    globalEventBus.subscribe(EventType.BLOCK_PROPOSED, () => {
      calc.heartbeat();
    })
  );

  // RUN: metrics:reported → heartbeat
  subscriptions.push(
    globalEventBus.subscribe(EventType.METRICS_REPORTED, () => {
      calc.heartbeat();
    })
  );

  // BUILD: tool:completed → track build activity (commits, PRs from hooks)
  subscriptions.push(
    globalEventBus.subscribe(EventType.TOOL_COMPLETED, (event) => {
      const { tool, result } = event.payload || {};
      if (tool === 'Bash' && result?.includes?.('commit')) {
        calc.recordCommit();
      }
    })
  );

  // BURN: Listen for burn events if they arrive via DOG_EVENT
  subscriptions.push(
    globalEventBus.subscribe(EventType.DOG_EVENT, (event) => {
      const { dog, action, data } = event.payload || {};
      if (action === 'burn' && data?.amount) {
        calc.recordBurn(data.amount, data.txSignature);
      }
      // BUILD: track architect/deployer activity
      if (dog === 'Architect' || dog === 'Deployer') {
        if (action === 'deploy' || action === 'build') {
          calc.recordCommit();
        }
      }
    })
  );

  // SOCIAL, GRAPH, HOLD: Stubbed as 0 (Phase 3)
  // calc.recordSocialContent(), calc.updateGraphPosition(), calc.recordHoldings()
  // will be wired when on-chain social and trust graph are implemented.

  log.info('EScore provider created', {
    selfPublicKey: selfPublicKey.slice(0, 16),
    subscribedEvents: subscriptions.length,
  });

  // Remote score cache: publicKey -> { score, updatedAt }
  const _remoteScores = new Map();
  const REMOTE_SCORE_TTL = 120_000; // 2 min (> 2 heartbeats at 8s interval)

  /**
   * Provider function for ValidatorManager.
   * Returns score for the local node via calculator;
   * returns cached score for remote validators (fed by heartbeats).
   *
   * @param {string} publicKey
   * @returns {number|null}
   */
  function provider(publicKey) {
    if (publicKey === selfPublicKey) {
      const result = calc.calculate();
      return result.score;
    }
    // Remote: return cached score from heartbeat
    const cached = _remoteScores.get(publicKey);
    if (!cached || Date.now() - cached.updatedAt > REMOTE_SCORE_TTL) {
      _remoteScores.delete(publicKey);
      return null;
    }
    return cached.score;
  }

  /**
   * Update a remote validator's E-Score (called from heartbeat handler)
   *
   * @param {string} publicKey - Remote validator's public key
   * @param {number} score - E-Score reported in heartbeat
   */
  function updateRemoteScore(publicKey, score, dimensions) {
    _remoteScores.set(publicKey, { score, dimensions: dimensions || null, updatedAt: Date.now() });
  }

  /**
   * Get remote validator's score with 7D breakdown (if available)
   *
   * @param {string} publicKey - Remote validator's public key
   * @returns {{ score: number, dimensions: Object|null }|null}
   */
  function getRemoteBreakdown(publicKey) {
    const cached = _remoteScores.get(publicKey);
    if (!cached || Date.now() - cached.updatedAt > REMOTE_SCORE_TTL) {
      _remoteScores.delete(publicKey);
      return null;
    }
    return { score: cached.score, dimensions: cached.dimensions };
  }

  /**
   * Clean up event subscriptions
   */
  function destroy() {
    for (const unsub of subscriptions) {
      if (typeof unsub === 'function') unsub();
      else if (unsub?.unsubscribe) unsub.unsubscribe();
    }
    subscriptions.length = 0;
    _remoteScores.clear();
    log.info('EScore provider destroyed');
  }

  return { provider, calculator: calc, destroy, updateRemoteScore, getRemoteBreakdown };
}
