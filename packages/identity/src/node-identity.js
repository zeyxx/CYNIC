/**
 * Node Identity - Complete CYNIC Node Identity
 *
 * "Know thyself, then verify" - κυνικός
 *
 * Combines all identity components:
 * - KeyManager: Cryptographic identity (Ed25519)
 * - EScoreCalculator: Reputation tracking
 * - BurnVerifier: Burn proof verification
 *
 * @module @cynic/identity/node-identity
 */

'use strict';

import { PHI_INV } from '@cynic/core';
import { KeyManager, createKeyManager } from './key-manager.js';
import { EScoreCalculator, createEScoreCalculator, THRESHOLDS } from './e-score.js';

/**
 * Identity status
 */
export const IdentityStatus = {
  UNINITIALIZED: 'UNINITIALIZED',  // No keys loaded
  EPHEMERAL: 'EPHEMERAL',          // Keys in memory only (not persisted)
  PERSISTENT: 'PERSISTENT',        // Keys persisted to file
  VERIFIED: 'VERIFIED',            // Has verified burns
};

/**
 * Node Identity
 *
 * Complete identity for a CYNIC node including:
 * - Cryptographic keys (Ed25519)
 * - Reputation (E-Score)
 * - Burn proofs
 *
 * @example
 * ```javascript
 * const identity = new NodeIdentity({
 *   keyfile: './node-key.json',
 *   datafile: './node-identity.json',
 * });
 *
 * await identity.initialize();
 *
 * console.log(identity.nodeId);       // Unique node ID
 * console.log(identity.eScore);       // Current E-Score
 * console.log(identity.status);       // Identity status
 *
 * // Sign data
 * const sig = identity.sign('hello');
 *
 * // Record activity
 * identity.heartbeat();
 * identity.recordJudgment('jdg_123', true);
 * identity.recordBurn(1000000, 'tx_sig...');
 * ```
 */
export class NodeIdentity {
  /**
   * @param {Object} options - Configuration
   * @param {string} [options.keyfile] - Path to keyfile
   * @param {string} [options.datafile] - Path to identity data file
   * @param {Object} [options.burnVerifier] - BurnVerifier instance
   * @param {number} [options.burnScale] - Burn normalization scale
   * @param {number} [options.minJudgments] - Minimum judgments for quality
   */
  constructor(options = {}) {
    this.keyfile = options.keyfile || null;
    this.datafile = options.datafile || null;
    this.burnVerifier = options.burnVerifier || null;

    // Initialize components
    this.keyManager = createKeyManager({ keyfile: this.keyfile });
    this.eScoreCalc = createEScoreCalculator({
      burnScale: options.burnScale,
      minJudgments: options.minJudgments,
    });

    // Metadata
    this.metadata = {
      createdAt: null,
      lastSeen: null,
      version: '1.0.0',
      tags: [],
    };

    // Status
    this._status = IdentityStatus.UNINITIALIZED;
    this._initialized = false;
  }

  /**
   * Node ID (derived from public key)
   * @type {string|null}
   */
  get nodeId() {
    return this.keyManager.nodeId;
  }

  /**
   * Public key (hex)
   * @type {string|null}
   */
  get publicKey() {
    return this.keyManager.publicKey;
  }

  /**
   * Formatted public key with prefix
   * @type {string|null}
   */
  get formattedKey() {
    return this.keyManager.formattedKey;
  }

  /**
   * Current E-Score
   * @type {number}
   */
  get eScore() {
    return this.eScoreCalc.calculate().score;
  }

  /**
   * E-Score status
   * @type {string}
   */
  get eScoreStatus() {
    return this.eScoreCalc.calculate().status;
  }

  /**
   * Identity status
   * @type {string}
   */
  get status() {
    return this._status;
  }

  /**
   * Check if identity is initialized
   * @type {boolean}
   */
  get isInitialized() {
    return this._initialized;
  }

  /**
   * Check if identity is trusted (E-Score >= 61.8%)
   * @type {boolean}
   */
  get isTrusted() {
    return this.eScore >= THRESHOLDS.TRUSTED;
  }

  /**
   * Initialize identity
   *
   * Loads or generates keys and restores state from datafile.
   *
   * @returns {Promise<void>}
   */
  async initialize() {
    // Load or generate keys
    const keysLoaded = await this.keyManager.loadOrGenerate();

    // Update status based on key persistence
    this._status = keysLoaded ? IdentityStatus.PERSISTENT : IdentityStatus.EPHEMERAL;

    // Load identity data if available
    if (this.datafile) {
      await this._loadData();
    }

    // Set creation time if new
    if (!this.metadata.createdAt) {
      this.metadata.createdAt = Date.now();
    }

    this.metadata.lastSeen = Date.now();
    this._initialized = true;

    // Save initial state
    if (this.datafile) {
      await this._saveData();
    }
  }

  /**
   * Sign data
   * @param {string|Buffer} data - Data to sign
   * @returns {string} Signature (hex)
   */
  sign(data) {
    this._ensureInitialized();
    return this.keyManager.sign(data);
  }

  /**
   * Sign an object
   * @param {Object} obj - Object to sign
   * @returns {string} Signature (hex)
   */
  signObject(obj) {
    this._ensureInitialized();
    return this.keyManager.signObject(obj);
  }

  /**
   * Verify signature
   * @param {string|Buffer} data - Original data
   * @param {string} signature - Signature to verify
   * @param {string} [publicKey] - Public key (defaults to own)
   * @returns {boolean} True if valid
   */
  verify(data, signature, publicKey) {
    return this.keyManager.verify(data, signature, publicKey);
  }

  /**
   * Record heartbeat (node is alive)
   */
  heartbeat() {
    this.eScoreCalc.heartbeat();
    this.metadata.lastSeen = Date.now();
  }

  /**
   * Mark offline
   */
  markOffline() {
    this.eScoreCalc.markOffline();
  }

  /**
   * Record a judgment
   * @param {string} judgmentId - Judgment ID
   * @param {boolean} matchedConsensus - Whether it matched consensus
   */
  recordJudgment(judgmentId, matchedConsensus) {
    this.eScoreCalc.recordJudgment(judgmentId, matchedConsensus);
  }

  /**
   * Record a burn
   * @param {number} amount - Burn amount
   * @param {string} [txSignature] - Transaction signature
   * @param {boolean} [verify=false] - Verify on-chain
   * @returns {Promise<boolean>} True if recorded (and verified if requested)
   */
  async recordBurn(amount, txSignature = null, verify = false) {
    // Optionally verify the burn on-chain
    if (verify && txSignature && this.burnVerifier) {
      const result = await this.burnVerifier.verify(txSignature, {
        minAmount: amount,
      });

      if (!result.verified) {
        return false;
      }

      // Use verified amount
      amount = result.amount;
    }

    this.eScoreCalc.recordBurn(amount, txSignature);

    // Update status if this is first verified burn
    if (txSignature && this._status !== IdentityStatus.VERIFIED) {
      this._status = IdentityStatus.VERIFIED;
    }

    return true;
  }

  /**
   * Get full E-Score calculation
   * @returns {Object} E-Score result with components
   */
  getEScoreDetails() {
    return this.eScoreCalc.calculate();
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    this._ensureInitialized();

    return {
      nodeId: this.nodeId,
      publicKey: this.publicKey,
      status: this._status,
      metadata: this.metadata,
      eScore: this.getEScoreDetails(),
      stats: this.eScoreCalc.getStats(),
    };
  }

  /**
   * Export public identity (safe to share)
   * @returns {Object}
   */
  exportPublic() {
    this._ensureInitialized();

    return {
      nodeId: this.nodeId,
      publicKey: this.publicKey,
      formattedKey: this.formattedKey,
      eScore: this.eScore,
      eScoreStatus: this.eScoreStatus,
      status: this._status,
      createdAt: this.metadata.createdAt,
      lastSeen: this.metadata.lastSeen,
      tags: this.metadata.tags,
    };
  }

  /**
   * Create signed identity attestation
   *
   * Proves ownership of this identity.
   *
   * @param {number} [ttl=300000] - Attestation validity in ms (default 5 min)
   * @returns {Object} Signed attestation
   */
  createAttestation(ttl = 300000) {
    this._ensureInitialized();

    const attestation = {
      type: 'CYNIC_IDENTITY_ATTESTATION',
      nodeId: this.nodeId,
      publicKey: this.publicKey,
      eScore: this.eScore,
      timestamp: Date.now(),
      expiresAt: Date.now() + ttl,
      version: this.metadata.version,
    };

    const signature = this.signObject(attestation);

    return {
      ...attestation,
      signature,
    };
  }

  /**
   * Verify an identity attestation
   * @param {Object} attestation - Attestation to verify
   * @returns {boolean} True if valid and not expired
   */
  static verifyAttestation(attestation) {
    if (!attestation || !attestation.signature || !attestation.publicKey) {
      return false;
    }

    // Check expiration
    if (attestation.expiresAt && Date.now() > attestation.expiresAt) {
      return false;
    }

    // Verify signature
    const { signature, ...data } = attestation;
    const km = createKeyManager({ publicKey: attestation.publicKey });

    try {
      return km.verifyObject(data, signature, attestation.publicKey);
    } catch {
      return false;
    }
  }

  /**
   * Add a tag to identity
   * @param {string} tag - Tag to add
   */
  addTag(tag) {
    if (!this.metadata.tags.includes(tag)) {
      this.metadata.tags.push(tag);
    }
  }

  /**
   * Remove a tag from identity
   * @param {string} tag - Tag to remove
   */
  removeTag(tag) {
    const idx = this.metadata.tags.indexOf(tag);
    if (idx !== -1) {
      this.metadata.tags.splice(idx, 1);
    }
  }

  /**
   * Save identity data to file
   * @returns {Promise<void>}
   */
  async save() {
    if (this.datafile) {
      await this._saveData();
    }
  }

  /**
   * Load identity data from file
   * @private
   */
  async _loadData() {
    if (!this.datafile) return;

    try {
      const fs = await import('fs');
      const data = fs.readFileSync(this.datafile, 'utf-8');
      const parsed = JSON.parse(data);

      // Restore E-Score state
      if (parsed.eScoreState) {
        this.eScoreCalc.import(parsed.eScoreState);
      }

      // Restore metadata
      if (parsed.metadata) {
        this.metadata = { ...this.metadata, ...parsed.metadata };
      }

      // Restore status
      if (parsed.status) {
        this._status = parsed.status;
      }
    } catch (err) {
      if (err.code !== 'ENOENT') {
        console.warn(`Failed to load identity data: ${err.message}`);
      }
    }
  }

  /**
   * Save identity data to file
   * @private
   */
  async _saveData() {
    if (!this.datafile) return;

    try {
      const fs = await import('fs');
      const path = await import('path');

      // Ensure directory exists
      const dir = path.dirname(this.datafile);
      if (dir && dir !== '.') {
        fs.mkdirSync(dir, { recursive: true });
      }

      const data = JSON.stringify({
        nodeId: this.nodeId,
        status: this._status,
        metadata: this.metadata,
        eScoreState: this.eScoreCalc.export(),
        savedAt: Date.now(),
      }, null, 2);

      fs.writeFileSync(this.datafile, data);
    } catch (err) {
      console.warn(`Failed to save identity data: ${err.message}`);
    }
  }

  /**
   * Ensure identity is initialized
   * @private
   */
  _ensureInitialized() {
    if (!this._initialized) {
      throw new Error('NodeIdentity not initialized. Call initialize() first.');
    }
  }
}

/**
 * Create a NodeIdentity instance
 * @param {Object} [options] - Configuration
 * @returns {NodeIdentity}
 */
export function createNodeIdentity(options = {}) {
  return new NodeIdentity(options);
}

export default {
  NodeIdentity,
  createNodeIdentity,
  IdentityStatus,
};
