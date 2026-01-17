/**
 * CYNIC Operator Registry
 *
 * Manages operators who can create PoJ blocks.
 * "Many dogs, one pack" - κυνικός
 *
 * Operators are nodes authorized to:
 * - Sign and propose PoJ blocks
 * - Validate blocks from other operators
 * - Participate in consensus
 *
 * @module @cynic/mcp/operator-registry
 */

'use strict';

import crypto from 'crypto';
import { EventEmitter } from 'events';
import { PHI_INV } from '@cynic/core';

/**
 * Generate operator key pair
 * @returns {{publicKey: string, privateKey: string}} Key pair
 */
export function generateOperatorKeyPair() {
  const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519', {
    publicKeyEncoding: { type: 'spki', format: 'der' },
    privateKeyEncoding: { type: 'pkcs8', format: 'der' },
  });
  return {
    publicKey: publicKey.toString('hex'),
    privateKey: privateKey.toString('hex'),
  };
}

/**
 * Sign data with operator private key
 * @param {string|Buffer} data - Data to sign
 * @param {string} privateKeyHex - Private key in hex
 * @returns {string} Signature in hex
 */
export function signWithOperatorKey(data, privateKeyHex) {
  const privateKey = crypto.createPrivateKey({
    key: Buffer.from(privateKeyHex, 'hex'),
    format: 'der',
    type: 'pkcs8',
  });
  const dataBuffer = typeof data === 'string' ? Buffer.from(data) : data;
  const signature = crypto.sign(null, dataBuffer, privateKey);
  return signature.toString('hex');
}

/**
 * Verify signature with operator public key
 * @param {string|Buffer} data - Original data
 * @param {string} signatureHex - Signature in hex
 * @param {string} publicKeyHex - Public key in hex
 * @returns {boolean} True if valid
 */
export function verifyOperatorSignature(data, signatureHex, publicKeyHex) {
  try {
    const publicKey = crypto.createPublicKey({
      key: Buffer.from(publicKeyHex, 'hex'),
      format: 'der',
      type: 'spki',
    });
    const dataBuffer = typeof data === 'string' ? Buffer.from(data) : data;
    const signature = Buffer.from(signatureHex, 'hex');
    return crypto.verify(null, dataBuffer, publicKey, signature);
  } catch (err) {
    return false;
  }
}

/**
 * Operator Registry
 *
 * Tracks authorized operators and their credentials.
 */
export class OperatorRegistry extends EventEmitter {
  /**
   * @param {Object} [options] - Configuration
   * @param {number} [options.minOperators=1] - Minimum operators for consensus
   * @param {number} [options.maxOperators=100] - Maximum operators allowed
   */
  constructor(options = {}) {
    super();

    this.minOperators = options.minOperators || 1;
    this.maxOperators = options.maxOperators || 100;

    // Operators: Map<publicKey, OperatorInfo>
    this._operators = new Map();

    // Self operator (this node)
    this._selfOperator = null;

    // Stats
    this._stats = {
      operatorsRegistered: 0,
      operatorsRemoved: 0,
      blocksValidated: 0,
      signaturesVerified: 0,
      signaturesFailed: 0,
    };
  }

  /**
   * Initialize self as operator
   * @param {Object} [options] - Operator options
   * @param {string} [options.publicKey] - Public key (generates if not provided)
   * @param {string} [options.privateKey] - Private key (generates if not provided)
   * @param {string} [options.name] - Operator name
   * @param {number} [options.weight=1] - Voting weight
   * @returns {Object} Self operator info
   */
  initializeSelf(options = {}) {
    let publicKey = options.publicKey;
    let privateKey = options.privateKey;

    if (!publicKey || !privateKey) {
      const keyPair = generateOperatorKeyPair();
      publicKey = publicKey || keyPair.publicKey;
      privateKey = privateKey || keyPair.privateKey;
    }

    this._selfOperator = {
      publicKey,
      privateKey,
      name: options.name || `operator_${publicKey.slice(0, 8)}`,
      weight: options.weight || 1,
      isSelf: true,
      registeredAt: Date.now(),
    };

    // Also register in operators map
    this._operators.set(publicKey, {
      publicKey,
      name: this._selfOperator.name,
      weight: this._selfOperator.weight,
      isSelf: true,
      registeredAt: Date.now(),
      blocksProposed: 0,
      lastBlockAt: null,
    });

    this._stats.operatorsRegistered++;
    this.emit('operator:self:initialized', { publicKey: publicKey.slice(0, 16) });

    return {
      publicKey,
      name: this._selfOperator.name,
    };
  }

  /**
   * Get self operator
   * @returns {Object|null} Self operator info (without private key)
   */
  getSelf() {
    if (!this._selfOperator) return null;
    return {
      publicKey: this._selfOperator.publicKey,
      name: this._selfOperator.name,
      weight: this._selfOperator.weight,
    };
  }

  /**
   * Sign data as self operator
   * @param {string|Buffer} data - Data to sign
   * @returns {string} Signature
   */
  sign(data) {
    if (!this._selfOperator) {
      throw new Error('Self operator not initialized');
    }
    return signWithOperatorKey(data, this._selfOperator.privateKey);
  }

  /**
   * Register an external operator
   * @param {Object} operator - Operator info
   * @param {string} operator.publicKey - Public key
   * @param {string} [operator.name] - Operator name
   * @param {number} [operator.weight=1] - Voting weight
   * @returns {boolean} True if registered
   */
  registerOperator(operator) {
    if (!operator.publicKey) {
      throw new Error('Operator public key required');
    }

    if (this._operators.size >= this.maxOperators) {
      throw new Error(`Maximum operators (${this.maxOperators}) reached`);
    }

    if (this._operators.has(operator.publicKey)) {
      // Update existing operator
      const existing = this._operators.get(operator.publicKey);
      existing.name = operator.name || existing.name;
      existing.weight = operator.weight ?? existing.weight;
      existing.updatedAt = Date.now();
      return false;
    }

    this._operators.set(operator.publicKey, {
      publicKey: operator.publicKey,
      name: operator.name || `operator_${operator.publicKey.slice(0, 8)}`,
      weight: operator.weight || 1,
      isSelf: false,
      registeredAt: Date.now(),
      blocksProposed: 0,
      lastBlockAt: null,
    });

    this._stats.operatorsRegistered++;
    this.emit('operator:registered', { publicKey: operator.publicKey.slice(0, 16) });

    return true;
  }

  /**
   * Remove an operator
   * @param {string} publicKey - Operator public key
   * @returns {boolean} True if removed
   */
  removeOperator(publicKey) {
    const operator = this._operators.get(publicKey);
    if (!operator) return false;

    if (operator.isSelf) {
      throw new Error('Cannot remove self operator');
    }

    this._operators.delete(publicKey);
    this._stats.operatorsRemoved++;
    this.emit('operator:removed', { publicKey: publicKey.slice(0, 16) });

    return true;
  }

  /**
   * Check if public key is a registered operator
   * @param {string} publicKey - Public key to check
   * @returns {boolean} True if registered
   */
  isOperator(publicKey) {
    return this._operators.has(publicKey);
  }

  /**
   * Get operator info
   * @param {string} publicKey - Operator public key
   * @returns {Object|null} Operator info or null
   */
  getOperator(publicKey) {
    return this._operators.get(publicKey) || null;
  }

  /**
   * Get all operators
   * @returns {Object[]} Array of operator info
   */
  getAllOperators() {
    return Array.from(this._operators.values());
  }

  /**
   * Verify a signature from any registered operator
   * @param {string|Buffer} data - Original data
   * @param {string} signature - Signature to verify
   * @param {string} publicKey - Operator public key
   * @returns {boolean} True if valid and from registered operator
   */
  verifySignature(data, signature, publicKey) {
    // Check if operator is registered
    if (!this._operators.has(publicKey)) {
      this._stats.signaturesFailed++;
      return false;
    }

    const valid = verifyOperatorSignature(data, signature, publicKey);

    if (valid) {
      this._stats.signaturesVerified++;
    } else {
      this._stats.signaturesFailed++;
    }

    return valid;
  }

  /**
   * Sign a PoJ block
   * @param {Object} block - Block to sign
   * @returns {Object} Block with signature
   */
  signBlock(block) {
    if (!this._selfOperator) {
      throw new Error('Self operator not initialized');
    }

    // Create signing data (deterministic representation)
    const signingData = JSON.stringify({
      slot: block.slot,
      prev_hash: block.prev_hash,
      judgments_root: block.judgments_root,
      timestamp: block.timestamp,
    });

    const signature = this.sign(signingData);

    return {
      ...block,
      operator: this._selfOperator.publicKey,
      operator_name: this._selfOperator.name,
      signature,
    };
  }

  /**
   * Verify a PoJ block signature
   * @param {Object} block - Block to verify
   * @returns {{valid: boolean, error?: string}} Verification result
   */
  verifyBlock(block) {
    if (!block.operator) {
      return { valid: false, error: 'Block has no operator' };
    }

    if (!block.signature) {
      return { valid: false, error: 'Block has no signature' };
    }

    if (!this._operators.has(block.operator)) {
      return { valid: false, error: `Unknown operator: ${block.operator.slice(0, 16)}...` };
    }

    // Recreate signing data
    const signingData = JSON.stringify({
      slot: block.slot,
      prev_hash: block.prev_hash,
      judgments_root: block.judgments_root,
      timestamp: block.timestamp,
    });

    const valid = verifyOperatorSignature(signingData, block.signature, block.operator);

    if (valid) {
      this._stats.blocksValidated++;
      // Update operator stats
      const operator = this._operators.get(block.operator);
      if (operator) {
        operator.blocksProposed++;
        operator.lastBlockAt = Date.now();
      }
    }

    return valid
      ? { valid: true }
      : { valid: false, error: 'Invalid block signature' };
  }

  /**
   * Get total voting weight of all operators
   * @returns {number} Total weight
   */
  getTotalWeight() {
    let total = 0;
    for (const operator of this._operators.values()) {
      total += operator.weight || 1;
    }
    return total || 1;
  }

  /**
   * Check if we have enough operators for consensus
   * @returns {boolean} True if quorum met
   */
  hasQuorum() {
    return this._operators.size >= this.minOperators;
  }

  /**
   * Get registry statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      operatorCount: this._operators.size,
      totalWeight: this.getTotalWeight(),
      hasQuorum: this.hasQuorum(),
      minOperators: this.minOperators,
      selfInitialized: !!this._selfOperator,
      maxConfidence: PHI_INV,
    };
  }

  /**
   * Export operator list (public keys only)
   * @returns {Object[]} Exportable operator list
   */
  exportOperators() {
    return Array.from(this._operators.values()).map(op => ({
      publicKey: op.publicKey,
      name: op.name,
      weight: op.weight,
      registeredAt: op.registeredAt,
    }));
  }

  /**
   * Import operators from list
   * @param {Object[]} operators - Operator list to import
   * @returns {{imported: number, skipped: number}} Import result
   */
  importOperators(operators) {
    let imported = 0;
    let skipped = 0;

    for (const op of operators) {
      try {
        if (this.registerOperator(op)) {
          imported++;
        } else {
          skipped++;
        }
      } catch (err) {
        skipped++;
      }
    }

    return { imported, skipped };
  }
}

export default OperatorRegistry;
