/**
 * Market Actor Configuration (C3.4 - MARKET × ACT)
 *
 * Executes trading actions based on market decisions.
 * Phase 1: Hypothetical trades only (NO real blockchain transactions)
 *
 * Action types: LOG_BUY, LOG_SELL, LOG_ACCUMULATE, LOG_DISTRIBUTE, ALERT_PRICE, HOLD
 *
 * @module @cynic/node/cycle/configs/market-actor
 */

'use strict';

import { globalEventBus } from '@cynic/core';
import { createLogger } from '@cynic/core';
import { phiBound } from '@cynic/core/axioms/phi-utils.js';

const log = createLogger('MarketActor');

/**
 * Action types (6 types, Fibonacci-aligned)
 */
export const MarketActionType = {
  LOG_BUY: 'log_buy',
  LOG_SELL: 'log_sell',
  LOG_ACCUMULATE: 'log_accumulate',
  LOG_DISTRIBUTE: 'log_distribute',
  ALERT_PRICE: 'alert_price',
  HOLD: 'hold',
};

/**
 * Action status
 */
export const MarketActionStatus = {
  DELIVERED: 'delivered',
  HOLD: 'hold',
  ERROR: 'error',
};

/**
 * Trade history size (F11 = 89)
 */
const TRADE_HISTORY_SIZE = 89;

/**
 * Initial portfolio (hypothetical)
 */
const INITIAL_PORTFOLIO = {
  tokens: 0,
  sol: 1.0, // Start with 1 SOL
  totalValue: 1.0,
  costBasis: 0,
  realizedPnL: 0,
};

/**
 * Market Actor Config
 *
 * 65% base logic (portfolio tracking, trade logging, P&L)
 * 35% delegated logic (core action execution)
 */
export const marketActorConfig = {
  // ============================================================================
  // BASE LOGIC (65% - portfolio, logging, state management)
  // ============================================================================
  baseLogicFunctions: {
    /**
     * Initialize state
     */
    _initState() {
      this._portfolio = { ...INITIAL_PORTFOLIO };
      this._tradeLog = [];
    },

    /**
     * Get current portfolio
     */
    getPortfolio() {
      return { ...this._portfolio };
    },

    /**
     * Get trade log
     */
    getTradeLog(limit = 10) {
      return this._tradeLog.slice(-limit);
    },

    /**
     * Get hypothetical P&L
     */
    getHypotheticalPnL() {
      return this._portfolio.realizedPnL;
    },

    /**
     * Execute hypothetical buy
     */
    _executeBuy(price, percentage = 0.10) {
      const solToSpend = this._portfolio.sol * percentage;
      if (solToSpend <= 0) {
        log.warn('No SOL available to buy');
        return null;
      }

      const tokensReceived = solToSpend / price;

      // Update portfolio
      this._portfolio.sol -= solToSpend;
      this._portfolio.tokens += tokensReceived;
      this._portfolio.costBasis += solToSpend;

      // Log trade
      const trade = {
        type: 'BUY',
        price,
        amount: tokensReceived,
        cost: solToSpend,
        timestamp: Date.now(),
        hypothetical: true,
      };
      this._logTrade(trade);

      return trade;
    },

    /**
     * Execute hypothetical sell
     */
    _executeSell(price, percentage = 0.20) {
      const tokensToSell = this._portfolio.tokens * percentage;
      if (tokensToSell <= 0) {
        log.warn('No tokens available to sell');
        return null;
      }

      const solReceived = tokensToSell * price;

      // Calculate P&L
      const costBasisSold = (this._portfolio.costBasis * percentage);
      const pnl = solReceived - costBasisSold;

      // Update portfolio
      this._portfolio.tokens -= tokensToSell;
      this._portfolio.sol += solReceived;
      this._portfolio.costBasis -= costBasisSold;
      this._portfolio.realizedPnL += pnl;

      // Log trade
      const trade = {
        type: 'SELL',
        price,
        amount: tokensToSell,
        proceeds: solReceived,
        pnl,
        timestamp: Date.now(),
        hypothetical: true,
      };
      this._logTrade(trade);

      return trade;
    },

    /**
     * Execute hypothetical accumulate (smaller buy)
     */
    _executeAccumulate(price) {
      return this._executeBuy(price, 0.05); // 5% of SOL
    },

    /**
     * Execute hypothetical distribute (smaller sell)
     */
    _executeDistribute(price) {
      return this._executeSell(price, 0.10); // 10% of tokens
    },

    /**
     * Log trade to history
     */
    _logTrade(trade) {
      this._tradeLog.push(trade);

      // Keep only TRADE_HISTORY_SIZE trades
      if (this._tradeLog.length > TRADE_HISTORY_SIZE) {
        this._tradeLog.shift();
      }

      // Emit trade event
      globalEventBus.emit('market:trade_logged', trade);
    },

    /**
     * Get statistics
     */
    getStats() {
      return {
        portfolio: this.getPortfolio(),
        tradeCount: this._tradeLog.length,
        hypotheticalPnL: this.getHypotheticalPnL(),
      };
    },
  },

  // ============================================================================
  // DELEGATED LOGIC (35% - core action execution)
  // ============================================================================
  delegatedLogic: {
    /**
     * Execute action based on market decision
     *
     * @param {Object} decision - Market decision object
     * @param {string} decision.decision - Decision type
     * @param {number} decision.confidence - Confidence level
     * @param {Object} decision.factors - Market factors
     * @param {Object} [context] - Additional context
     * @returns {Object} Action result
     */
    async act(decision, context = {}) {
      const { decision: decisionType, confidence, factors } = decision;
      const { price } = factors || {};

      if (!price || price <= 0) {
        log.error('Invalid price for market action', { decision });
        return this._createResult(
          'hold',
          MarketActionStatus.ERROR,
          'Invalid price data',
          'critical'
        );
      }

      let actionType;
      let trade = null;
      let message = '';

      // Execute based on decision type
      switch (decisionType) {
        case 'buy_signal':
          trade = this._executeBuy(price, 0.10); // 10% of SOL
          if (trade) {
            actionType = MarketActionType.LOG_BUY;
            message = `Hypothetical BUY: ${trade.amount.toFixed(2)} tokens at $${price.toFixed(6)} (cost: ${trade.cost.toFixed(3)} SOL)`;
          } else {
            actionType = MarketActionType.HOLD;
            message = 'Insufficient SOL for BUY';
          }
          break;

        case 'sell_signal':
          trade = this._executeSell(price, 0.20); // 20% of tokens
          if (trade) {
            actionType = MarketActionType.LOG_SELL;
            message = `Hypothetical SELL: ${trade.amount.toFixed(2)} tokens at $${price.toFixed(6)} (proceeds: ${trade.proceeds.toFixed(3)} SOL, P&L: ${trade.pnl > 0 ? '+' : ''}${trade.pnl.toFixed(3)} SOL)`;
          } else {
            actionType = MarketActionType.HOLD;
            message = 'No tokens to SELL';
          }
          break;

        case 'accumulate':
          trade = this._executeAccumulate(price);
          if (trade) {
            actionType = MarketActionType.LOG_ACCUMULATE;
            message = `Hypothetical ACCUMULATE: ${trade.amount.toFixed(2)} tokens at $${price.toFixed(6)}`;
          } else {
            actionType = MarketActionType.HOLD;
            message = 'Insufficient SOL for ACCUMULATE';
          }
          break;

        case 'distribute':
          trade = this._executeDistribute(price);
          if (trade) {
            actionType = MarketActionType.LOG_DISTRIBUTE;
            message = `Hypothetical DISTRIBUTE: ${trade.amount.toFixed(2)} tokens at $${price.toFixed(6)}`;
          } else {
            actionType = MarketActionType.HOLD;
            message = 'No tokens to DISTRIBUTE';
          }
          break;

        case 'alert':
          actionType = MarketActionType.ALERT_PRICE;
          message = `ALERT: ${decision.reason}`;
          break;

        case 'hold':
        case 'wait':
        default:
          actionType = MarketActionType.HOLD;
          message = decision.reason || 'No action taken';
          break;
      }

      // Create result
      const result = this._createResult(
        actionType,
        MarketActionStatus.DELIVERED,
        message,
        decision.severity || 'low',
        confidence,
        trade
      );

      log.debug('Market action executed', {
        type: actionType,
        message,
        portfolio: this.getPortfolio(),
      });

      return result;
    },

    /**
     * Create action result
     */
    _createResult(type, status, message, urgency, confidence = 0.38, trade = null) {
      return {
        type,
        status,
        message,
        urgency,
        confidence: phiBound(confidence),
        trade,
        portfolio: this.getPortfolio(),
        timestamp: Date.now(),
      };
    },
  },

  // ============================================================================
  // METADATA
  // ============================================================================
  actionTypes: MarketActionType,
  ActionStatus: MarketActionStatus,
  singletonName: 'MarketActor',
  domain: 'MARKET',
  cell: 'C3.4',
};

export default marketActorConfig;
