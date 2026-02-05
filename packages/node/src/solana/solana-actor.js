/**
 * CYNIC Solana Actor - C2.4 (SOLANA × ACT)
 *
 * Executes Solana transactions based on decisions.
 * Part of the 7×7 Fractal Matrix action layer.
 *
 * "When φ says go, the dog acts" - κυνικός
 *
 * Acts:
 * - Send transactions (signed, confirmed)
 * - Execute program calls
 * - Manage transaction lifecycle
 * - Handle errors and retries
 *
 * @module @cynic/node/solana/solana-actor
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('SolanaActor');

/**
 * Action types
 */
export const SolanaActionType = {
  SEND_TRANSACTION: 'send_transaction',
  CONFIRM_TRANSACTION: 'confirm_transaction',
  SIMULATE_TRANSACTION: 'simulate_transaction',
  GET_ACCOUNT: 'get_account',
  GET_BALANCE: 'get_balance',
  AIRDROP: 'airdrop',
};

/**
 * Action status
 */
export const ActionStatus = {
  PENDING: 'pending',
  EXECUTING: 'executing',
  CONFIRMED: 'confirmed',
  FAILED: 'failed',
  TIMEOUT: 'timeout',
};

/**
 * SolanaActor - Executes Solana actions
 */
export class SolanaActor extends EventEmitter {
  /**
   * Create a new SolanaActor
   *
   * @param {Object} [options] - Configuration
   * @param {Object} [options.connection] - Solana connection
   * @param {number} [options.confirmationTimeout=30000] - Confirmation timeout ms
   */
  constructor(options = {}) {
    super();

    this._connection = options.connection || null;
    this._confirmationTimeout = options.confirmationTimeout || 30000;

    // Action queue
    this._pendingActions = new Map();

    // Stats
    this._stats = {
      actionsTotal: 0,
      byType: {},
      byStatus: {},
      avgExecutionTime: 0,
      lastAction: null,
    };

    // Initialize counters
    for (const type of Object.values(SolanaActionType)) {
      this._stats.byType[type] = 0;
    }
    for (const status of Object.values(ActionStatus)) {
      this._stats.byStatus[status] = 0;
    }
  }

  /**
   * Set Solana connection
   *
   * @param {Object} connection - Solana connection
   */
  setConnection(connection) {
    this._connection = connection;
  }

  /**
   * Execute an action
   *
   * @param {string} type - Action type
   * @param {Object} params - Action parameters
   * @returns {Promise<Object>} Action result
   */
  async execute(type, params) {
    const actionId = `action_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    const startTime = Date.now();

    const action = {
      id: actionId,
      type,
      params,
      status: ActionStatus.PENDING,
      startTime,
      result: null,
      error: null,
    };

    this._pendingActions.set(actionId, action);
    this._stats.actionsTotal++;
    this._stats.byType[type] = (this._stats.byType[type] || 0) + 1;

    // Emit start event
    this.emit('action_start', { actionId, type });

    try {
      action.status = ActionStatus.EXECUTING;

      // Execute based on type (mock implementation for now)
      let result;
      switch (type) {
        case SolanaActionType.SEND_TRANSACTION:
          result = await this._sendTransaction(params);
          break;
        case SolanaActionType.CONFIRM_TRANSACTION:
          result = await this._confirmTransaction(params);
          break;
        case SolanaActionType.SIMULATE_TRANSACTION:
          result = await this._simulateTransaction(params);
          break;
        case SolanaActionType.GET_ACCOUNT:
          result = await this._getAccount(params);
          break;
        case SolanaActionType.GET_BALANCE:
          result = await this._getBalance(params);
          break;
        case SolanaActionType.AIRDROP:
          result = await this._airdrop(params);
          break;
        default:
          throw new Error(`Unknown action type: ${type}`);
      }

      action.status = ActionStatus.CONFIRMED;
      action.result = result;

      this._stats.byStatus[ActionStatus.CONFIRMED]++;

    } catch (error) {
      action.status = ActionStatus.FAILED;
      action.error = error.message;

      this._stats.byStatus[ActionStatus.FAILED]++;

      log.error('Action failed', { actionId, type, error: error.message });
    }

    // Calculate execution time
    action.executionTime = Date.now() - startTime;

    // Update avg execution time
    const n = this._stats.actionsTotal;
    this._stats.avgExecutionTime = ((n - 1) * this._stats.avgExecutionTime + action.executionTime) / n;
    this._stats.lastAction = Date.now();

    // Cleanup
    this._pendingActions.delete(actionId);

    // Build result
    const result = {
      actionId,
      type,
      cell: 'C2.4',
      dimension: 'SOLANA',
      analysis: 'ACT',
      status: action.status,
      result: action.result,
      error: action.error,
      executionTime: action.executionTime,
      timestamp: Date.now(),
    };

    // Emit events
    this.emit('action_complete', result);
    globalEventBus.emit('solana:action', result);

    return result;
  }

  /**
   * Send transaction (stub - needs real connection)
   * @private
   */
  async _sendTransaction(params) {
    if (!this._connection) {
      return { signature: null, simulated: true, message: 'No connection - simulated' };
    }

    const { transaction, signers } = params;
    const signature = await this._connection.sendTransaction(transaction, signers);
    return { signature };
  }

  /**
   * Confirm transaction (stub)
   * @private
   */
  async _confirmTransaction(params) {
    if (!this._connection) {
      return { confirmed: true, simulated: true };
    }

    const { signature, commitment = 'confirmed' } = params;
    const result = await this._connection.confirmTransaction(signature, commitment);
    return { confirmed: !result.value?.err, result };
  }

  /**
   * Simulate transaction (stub)
   * @private
   */
  async _simulateTransaction(params) {
    if (!this._connection) {
      return { success: true, simulated: true, logs: [] };
    }

    const { transaction } = params;
    const result = await this._connection.simulateTransaction(transaction);
    return {
      success: !result.value?.err,
      logs: result.value?.logs || [],
      unitsConsumed: result.value?.unitsConsumed,
    };
  }

  /**
   * Get account info (stub)
   * @private
   */
  async _getAccount(params) {
    if (!this._connection) {
      return { exists: false, simulated: true };
    }

    const { pubkey } = params;
    const account = await this._connection.getAccountInfo(pubkey);
    return {
      exists: !!account,
      lamports: account?.lamports,
      owner: account?.owner?.toString(),
      dataLength: account?.data?.length,
    };
  }

  /**
   * Get balance (stub)
   * @private
   */
  async _getBalance(params) {
    if (!this._connection) {
      return { lamports: 0, simulated: true };
    }

    const { pubkey } = params;
    const lamports = await this._connection.getBalance(pubkey);
    return { lamports, sol: lamports / 1e9 };
  }

  /**
   * Request airdrop (stub)
   * @private
   */
  async _airdrop(params) {
    if (!this._connection) {
      return { signature: null, simulated: true };
    }

    const { pubkey, amount } = params;
    const signature = await this._connection.requestAirdrop(pubkey, amount);
    return { signature };
  }

  /**
   * Get pending actions count
   *
   * @returns {number}
   */
  getPendingCount() {
    return this._pendingActions.size;
  }

  /**
   * Get statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this._stats,
      pendingCount: this._pendingActions.size,
    };
  }

  /**
   * Get health assessment
   *
   * @returns {Object}
   */
  getHealth() {
    const hasConnection = !!this._connection;
    const pendingCount = this._pendingActions.size;
    const failureRate = this._stats.actionsTotal > 0
      ? this._stats.byStatus[ActionStatus.FAILED] / this._stats.actionsTotal
      : 0;

    let status = 'healthy';
    let score = PHI_INV;

    if (!hasConnection) {
      status = 'disconnected';
      score = PHI_INV_2;
    } else if (failureRate > 0.5) {
      status = 'degraded';
      score = PHI_INV_2;
    } else if (pendingCount > 10) {
      status = 'congested';
      score = PHI_INV_2;
    }

    return {
      status,
      score,
      hasConnection,
      pendingCount,
      failureRate,
      actionsTotal: this._stats.actionsTotal,
      avgExecutionTimeMs: this._stats.avgExecutionTime,
    };
  }

  /**
   * Clear state
   */
  clear() {
    this._pendingActions.clear();
    this._stats = {
      actionsTotal: 0,
      byType: {},
      byStatus: {},
      avgExecutionTime: 0,
      lastAction: null,
    };
    for (const type of Object.values(SolanaActionType)) {
      this._stats.byType[type] = 0;
    }
    for (const status of Object.values(ActionStatus)) {
      this._stats.byStatus[status] = 0;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaActor singleton
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {SolanaActor}
 */
export function getSolanaActor(options = {}) {
  if (!_instance) {
    _instance = new SolanaActor(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaActor() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaActor,
  SolanaActionType,
  ActionStatus,
  getSolanaActor,
  resetSolanaActor,
};
