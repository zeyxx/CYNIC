/**
 * Market Module — R3 (MARKET row of 7×7 matrix)
 *
 * Exports all market components:
 * - C3.1 (PERCEIVE): MarketWatcher
 * - C3.2 (JUDGE): MarketJudge
 * - C3.3 (DECIDE): (future) MarketDecider
 * - C3.4 (ACT): (future) MarketActor
 * - C3.5 (LEARN): MarketLearner
 * - C3.6 (ACCOUNT): MarketAccountant
 * - C3.7 (EMERGE): MarketEmergence
 *
 * "The market is a teacher, not an oracle" - κυνικός
 *
 * @module @cynic/node/market
 */

'use strict';

// C3.1: Perception
export {
  MarketWatcher,
  getMarketWatcher,
  resetMarketWatcher,
  ASDFASDFA_MINT,
  MarketEventType,
} from './market-watcher.js';

// C3.2: Judgment
export {
  MarketJudge,
  getMarketJudge,
  resetMarketJudge,
  MarketVerdict,
} from './market-judge.js';

// C3.5: Learning
export {
  MarketLearner,
  getMarketLearner,
  resetMarketLearner,
} from './market-learner.js';

// C3.6: Accounting
export {
  MarketAccountant,
  getMarketAccountant,
  resetMarketAccountant,
} from './market-accountant.js';

// C3.7: Emergence
export {
  MarketEmergence,
  getMarketEmergence,
  resetMarketEmergence,
  PatternType,
} from './market-emergence.js';

/**
 * Wire market components together
 *
 * Connects MarketWatcher → Judge → Learner → Accountant → Emergence
 *
 * @param {Object} options
 * @param {import('./market-watcher.js').MarketWatcher} options.watcher
 * @param {import('./market-judge.js').MarketJudge} options.judge
 * @param {import('./market-learner.js').MarketLearner} options.learner
 * @param {import('./market-accountant.js').MarketAccountant} options.accountant
 * @param {import('./market-emergence.js').MarketEmergence} options.emergence
 */
export function wireMarketPipeline(options = {}) {
  const {
    watcher = getMarketWatcher(),
    judge = getMarketJudge(),
    learner = getMarketLearner(),
    accountant = getMarketAccountant(),
    emergence = getMarketEmergence(),
  } = options;

  // Wire: MarketWatcher.PRICE_UPDATE → Judge → Learner → Accountant → Emergence
  watcher.on('perception:market:price', (event) => {
    // C3.2: Judge the price move
    const judgment = judge.judgePriceMove(event);

    // C3.5: Learn from the price
    learner.learnFromPrice(event);

    // C3.6: Update portfolio value
    accountant.updateValue(event.price);

    // C3.7: Detect emergent patterns
    emergence.processEvent({ ...event, judgment });
  });

  // Wire: MarketWatcher.LIQUIDITY_CHANGE → Judge
  watcher.on('perception:market:liquidity', (event) => {
    judge.judgeLiquidityChange(event);
  });

  // Wire: MarketWatcher.VOLUME_SPIKE → Judge → Emergence
  watcher.on('perception:market:volume_spike', (event) => {
    const judgment = judge.judgeVolumeSpike(event);
    emergence.processEvent({ ...event, judgment });
  });

  return {
    watcher,
    judge,
    learner,
    accountant,
    emergence,
  };
}
