/**
 * ValidatorManager - Validator set management
 *
 * Extracted from CYNICNetworkNode monolith (BURN)
 *
 * Manages the validator set: add/remove/penalize/reward validators,
 * track activity, evict lowest performers, calculate voting weight.
 *
 * "The pack decides who hunts" - κυνικός
 *
 * @module @cynic/node/network/validator-manager
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger } from '@cynic/core';
import { calculateVoteWeight } from '@cynic/protocol';

const log = createLogger('ValidatorManager');

/**
 * @typedef {Object} ValidatorInfo
 * @property {string} publicKey
 * @property {number} eScore - Current E-Score (0-100)
 * @property {number} burned - Total burned tokens
 * @property {number} uptime - Uptime ratio (0-1)
 * @property {number} lastSeen - Last heartbeat timestamp
 * @property {number} blocksProposed
 * @property {number} blocksFinalized
 * @property {number} penalties - Accumulated penalties
 * @property {string} status - 'active' | 'inactive' | 'penalized' | 'removed'
 * @property {number} joinedAt
 */

const DEFAULT_CONFIG = {
  minEScore: 20,
  maxValidators: 100,
  inactivityTimeout: 120000,   // 2 minutes
  penaltyDecay: 0.95,
  rewardMultiplier: 1.02,
};

export class ValidatorManager extends EventEmitter {
  /**
   * @param {Object} [options]
   * @param {string} options.selfPublicKey - Our node public key (protected from eviction)
   * @param {Object} [options.config] - Override default config
   */
  constructor(options = {}) {
    super();

    this._selfPublicKey = options.selfPublicKey;
    this._config = { ...DEFAULT_CONFIG, ...options.config };
    this._validators = new Map(); // publicKey -> ValidatorInfo
    this._eScoreProvider = options.eScoreProvider || null;

    this._stats = {
      validatorsAdded: 0,
      validatorsRemoved: 0,
      validatorsPenalized: 0,
    };

    // Injected dependency (set via wire())
    this._syncToConsensus = null;
    this._removeFromConsensus = null;
  }

  /** @param {Function} fn - (publicKey) => number|null */
  set eScoreProvider(fn) { this._eScoreProvider = fn; }

  /**
   * Wire consensus callbacks
   * @param {Object} deps
   * @param {Function} deps.syncToConsensus - Register validator in consensus (validatorInfo)
   * @param {Function} deps.removeFromConsensus - Remove validator from consensus (publicKey)
   */
  wire({ syncToConsensus, removeFromConsensus }) {
    if (syncToConsensus) this._syncToConsensus = syncToConsensus;
    if (removeFromConsensus) this._removeFromConsensus = removeFromConsensus;
  }

  /**
   * Add or update a validator
   * @param {Object} validator
   * @param {string} validator.publicKey
   * @param {number} [validator.eScore=50]
   * @param {number} [validator.burned=0]
   * @returns {boolean} True if added (new), false if updated or rejected
   */
  addValidator(validator) {
    const { publicKey, eScore = 50, burned = 0 } = validator;

    if (!publicKey) {
      log.warn('Cannot add validator without publicKey');
      return false;
    }

    if (eScore < this._config.minEScore) {
      log.info('Validator E-Score below minimum', {
        publicKey: publicKey.slice(0, 16),
        eScore,
        minRequired: this._config.minEScore,
      });
      return false;
    }

    const existing = this._validators.get(publicKey);
    const isNew = !existing;

    if (isNew) {
      if (this._validators.size >= this._config.maxValidators) {
        if (!this._evictLowestValidator(eScore)) {
          log.info('Validator set full, cannot add', { publicKey: publicKey.slice(0, 16) });
          return false;
        }
      }
    }

    const validatorInfo = {
      publicKey,
      eScore,
      burned,
      uptime: existing?.uptime || 1.0,
      lastSeen: Date.now(),
      blocksProposed: existing?.blocksProposed || 0,
      blocksFinalized: existing?.blocksFinalized || 0,
      penalties: existing?.penalties || 0,
      status: 'active',
      joinedAt: existing?.joinedAt || Date.now(),
    };

    this._validators.set(publicKey, validatorInfo);

    this._syncToConsensus?.({
      publicKey,
      eScore,
      burned,
      uptime: validatorInfo.uptime,
    });

    if (isNew) {
      this._stats.validatorsAdded++;

      log.info('Validator added', {
        publicKey: publicKey.slice(0, 16),
        eScore,
        totalValidators: this._validators.size,
      });

      this.emit('validator:added', { publicKey, eScore, burned });
    } else {
      this.emit('validator:updated', { publicKey, eScore, burned });
    }

    return isNew;
  }

  /**
   * Remove a validator
   * @param {string} publicKey
   * @param {string} [reason='manual']
   * @returns {boolean} True if removed
   */
  removeValidator(publicKey, reason = 'manual') {
    const validator = this._validators.get(publicKey);
    if (!validator) return false;

    validator.status = 'removed';
    this._validators.delete(publicKey);

    this._removeFromConsensus?.(publicKey);

    this._stats.validatorsRemoved++;

    log.info('Validator removed', {
      publicKey: publicKey.slice(0, 16),
      reason,
      remainingValidators: this._validators.size,
    });

    this.emit('validator:removed', { publicKey, reason });
    return true;
  }

  /**
   * Penalize a validator
   * @param {string} publicKey
   * @param {number} penalty - Penalty amount (0-100)
   * @param {string} reason
   * @returns {boolean} True if penalized
   */
  penalizeValidator(publicKey, penalty, reason) {
    const validator = this._validators.get(publicKey);
    if (!validator) return false;

    validator.penalties += penalty;
    validator.eScore = Math.max(0, validator.eScore - penalty);

    if (validator.eScore < this._config.minEScore) {
      validator.status = 'penalized';
      this.removeValidator(publicKey, `penalty_threshold_${reason}`);
      this._stats.validatorsPenalized++;
      return true;
    }

    this._syncToConsensus?.({
      publicKey,
      eScore: validator.eScore,
      burned: validator.burned,
      uptime: validator.uptime,
    });

    this._stats.validatorsPenalized++;

    log.warn('Validator penalized', {
      publicKey: publicKey.slice(0, 16),
      penalty,
      reason,
      newEScore: validator.eScore,
    });

    this.emit('validator:penalized', {
      publicKey,
      penalty,
      reason,
      newEScore: validator.eScore,
    });

    return true;
  }

  /**
   * Reward a validator for good behavior
   * @param {string} publicKey
   * @param {string} action - 'block_proposed' | 'block_finalized'
   */
  rewardValidator(publicKey, action) {
    const validator = this._validators.get(publicKey);
    if (!validator) return;

    switch (action) {
      case 'block_proposed':
        validator.blocksProposed++;
        break;
      case 'block_finalized':
        validator.blocksFinalized++;
        validator.eScore = Math.min(100, validator.eScore * this._config.rewardMultiplier);
        break;
    }

    this.emit('validator:rewarded', { publicKey, action });
  }

  /**
   * Update validator activity (on heartbeat)
   * @param {string} publicKey
   */
  updateValidatorActivity(publicKey) {
    const validator = this._validators.get(publicKey);
    if (!validator) return;

    const wasInactive = validator.status === 'inactive';
    validator.lastSeen = Date.now();
    validator.status = 'active';

    // Re-register with consensus engine if returning from inactive
    if (wasInactive) {
      this._syncToConsensus?.({
        publicKey,
        eScore: validator.eScore,
        burned: validator.burned,
        uptime: validator.uptime,
      });
    }
  }

  /**
   * Check for inactive validators
   */
  checkInactiveValidators() {
    const now = Date.now();
    const timeout = this._config.inactivityTimeout;

    for (const [publicKey, validator] of this._validators) {
      if (validator.status === 'active' && now - validator.lastSeen > timeout) {
        validator.status = 'inactive';
        validator.uptime *= 0.9;

        // Remove from consensus engine weight calculation.
        // Without this, phantom validators (briefly-connected peers) dilute
        // totalWeight and prevent real voters from reaching 61.8% supermajority.
        // Will be re-added on next heartbeat via updateValidatorActivity().
        this._removeFromConsensus?.(publicKey);

        log.info('Validator marked inactive', {
          publicKey: publicKey.slice(0, 16),
          lastSeen: Math.round((now - validator.lastSeen) / 1000) + 's ago',
        });

        this.emit('validator:inactive', { publicKey });

        if (now - validator.lastSeen > timeout * 3) {
          this.penalizeValidator(publicKey, 5, 'extended_inactivity');
        }
      }
    }
  }

  /**
   * Apply penalty decay across all validators
   */
  applyPenaltyDecay() {
    for (const validator of this._validators.values()) {
      if (validator.penalties > 0) {
        validator.penalties *= this._config.penaltyDecay;
        if (validator.penalties < 0.1) {
          validator.penalties = 0;
        }
      }
    }
  }

  /**
   * Refresh E-Scores from the provider for all active validators
   * @returns {number} Number of validators updated
   */
  refreshEScores() {
    if (!this._eScoreProvider) return 0;

    let updated = 0;
    for (const [publicKey, validator] of this._validators) {
      if (validator.status !== 'active') continue;

      try {
        const newScore = this._eScoreProvider(publicKey);
        if (newScore != null && newScore !== validator.eScore) {
          const oldScore = validator.eScore;
          validator.eScore = newScore;
          updated++;

          if (validator.eScore < this._config.minEScore) {
            this.removeValidator(publicKey, 'escore_below_minimum');
            continue;
          }

          this._syncToConsensus?.({
            publicKey,
            eScore: validator.eScore,
            burned: validator.burned,
            uptime: validator.uptime,
          });

          this.emit('validator:escore_updated', { publicKey, oldScore, newScore });
        }
      } catch (err) {
        log.warn('eScoreProvider error', { publicKey: publicKey.slice(0, 16), error: err.message });
      }
    }
    return updated;
  }

  /**
   * Evict the lowest E-Score validator to make room
   * @private
   * @param {number} newEScore - Incoming validator's E-Score
   * @returns {boolean} True if evicted someone
   */
  _evictLowestValidator(newEScore) {
    let lowest = null;
    let lowestScore = Infinity;

    for (const [publicKey, validator] of this._validators) {
      if (publicKey === this._selfPublicKey) continue;

      const effectiveScore = validator.eScore * (0.5 + validator.uptime * 0.5);
      if (effectiveScore < lowestScore) {
        lowestScore = effectiveScore;
        lowest = publicKey;
      }
    }

    if (lowest && newEScore > lowestScore) {
      this.removeValidator(lowest, 'evicted_for_higher_escore');
      return true;
    }

    return false;
  }

  /**
   * Get validator info
   * @param {string} publicKey
   * @returns {ValidatorInfo|null}
   */
  getValidator(publicKey) {
    return this._validators.get(publicKey) || null;
  }

  /**
   * Get all validators with optional filter
   * @param {Object} [filter]
   * @param {string} [filter.status]
   * @param {number} [filter.minEScore]
   * @returns {Array<ValidatorInfo>}
   */
  getValidators(filter = {}) {
    let validators = Array.from(this._validators.values());

    if (filter.status) {
      validators = validators.filter(v => v.status === filter.status);
    }
    if (filter.minEScore !== undefined) {
      validators = validators.filter(v => v.eScore >= filter.minEScore);
    }

    validators.sort((a, b) => {
      const scoreA = a.eScore * (0.5 + a.uptime * 0.5);
      const scoreB = b.eScore * (0.5 + b.uptime * 0.5);
      return scoreB - scoreA;
    });

    return validators;
  }

  /**
   * Get validator set summary
   * @returns {Object}
   */
  getValidatorSetStatus() {
    const validators = Array.from(this._validators.values());
    const active = validators.filter(v => v.status === 'active');
    const inactive = validators.filter(v => v.status === 'inactive');

    const totalEScore = active.reduce((sum, v) => sum + v.eScore, 0);
    const avgEScore = active.length > 0 ? totalEScore / active.length : 0;

    return {
      total: validators.length,
      active: active.length,
      inactive: inactive.length,
      maxValidators: this._config.maxValidators,
      minEScore: this._config.minEScore,
      totalEScore,
      avgEScore: Math.round(avgEScore * 10) / 10,
      selfIncluded: this._validators.has(this._selfPublicKey),
      stats: { ...this._stats },
    };
  }

  /**
   * Calculate total voting weight for φ-BFT consensus
   * Uses protocol-aligned formula: eScore × max(log_φ(burned+1), 1) × uptime
   * @returns {number}
   */
  getTotalVotingWeight() {
    let total = 0;
    for (const validator of this._validators.values()) {
      if (validator.status === 'active') {
        total += calculateVoteWeight({
          eScore: validator.eScore,
          burned: validator.burned,
          uptime: validator.uptime,
        });
      }
    }
    return total;
  }

  /**
   * Get voting weight for a specific validator
   * @param {string} publicKey
   * @returns {number} Weight (0 if not found or not active)
   */
  getValidatorWeight(publicKey) {
    const v = this._validators.get(publicKey);
    if (!v || v.status !== 'active') return 0;
    return calculateVoteWeight({ eScore: v.eScore, burned: v.burned, uptime: v.uptime });
  }

  /**
   * Check if we have supermajority (61.8% φ-BFT)
   * @param {number} votingWeight
   * @returns {boolean}
   */
  hasSupermajority(votingWeight) {
    const total = this.getTotalVotingWeight();
    if (total === 0) return false;
    return votingWeight / total >= 0.618;
  }

  /** @returns {number} Validator count */
  get size() {
    return this._validators.size;
  }

  /** @returns {Object} Stats */
  get stats() {
    return { ...this._stats };
  }
}

export default ValidatorManager;
