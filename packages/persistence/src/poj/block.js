/**
 * PoJ Block Structure
 *
 * Defines the block structure for the Proof of Judgment chain.
 * Each block contains judgments, links to previous block, and attestations.
 *
 * "The chain remembers what dogs forget" - κυνικός
 *
 * @module @cynic/persistence/poj/block
 */

'use strict';

import crypto from 'crypto';
import { encode, decode } from 'cbor-x';
import { createCID, isValidCID } from '../dag/cid.js';

// φ Constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const SLOT_DURATION_MS = 61.8;
const MAX_JUDGMENTS_PER_BLOCK = 13; // Fibonacci number
const QUORUM_THRESHOLD = PHI_INV; // 61.8%

/**
 * Compute merkle root from array of CIDs
 * @param {string[]} cids - Array of CIDs
 * @returns {string} Merkle root hash (hex)
 */
export function computeMerkleRoot(cids) {
  if (cids.length === 0) {
    return crypto.createHash('sha256').update('empty').digest('hex');
  }

  // Convert CIDs to leaf hashes
  let hashes = cids.map(cid =>
    crypto.createHash('sha256').update(cid).digest()
  );

  // Build tree bottom-up
  while (hashes.length > 1) {
    const nextLevel = [];

    for (let i = 0; i < hashes.length; i += 2) {
      const left = hashes[i];
      const right = hashes[i + 1] || left; // Duplicate last if odd

      const combined = crypto.createHash('sha256')
        .update(Buffer.concat([left, right]))
        .digest();

      nextLevel.push(combined);
    }

    hashes = nextLevel;
  }

  return hashes[0].toString('hex');
}

/**
 * PoJ Block Header
 */
export class PoJBlockHeader {
  constructor({
    slot,
    timestamp,
    prevHash,
    judgmentsRoot,
    stateRoot,
    proposer,
  }) {
    this.slot = slot;
    this.timestamp = timestamp || Date.now();
    this.prevHash = prevHash || null;
    this.judgmentsRoot = judgmentsRoot;
    this.stateRoot = stateRoot || '';
    this.proposer = proposer;
  }

  /**
   * Compute header hash
   * @returns {string} SHA-256 hash (hex)
   */
  hash() {
    const data = JSON.stringify({
      slot: this.slot,
      timestamp: this.timestamp,
      prevHash: this.prevHash,
      judgmentsRoot: this.judgmentsRoot,
      stateRoot: this.stateRoot,
      proposer: this.proposer,
    });

    return crypto.createHash('sha256').update(data).digest('hex');
  }

  toJSON() {
    return {
      slot: this.slot,
      timestamp: this.timestamp,
      prevHash: this.prevHash,
      judgmentsRoot: this.judgmentsRoot,
      stateRoot: this.stateRoot,
      proposer: this.proposer,
    };
  }

  static fromJSON(json) {
    return new PoJBlockHeader(json);
  }
}

/**
 * Attestation - node's signature on a block
 */
export class Attestation {
  constructor({
    nodeId,
    slot,
    blockHash,
    signature,
    timestamp,
  }) {
    this.nodeId = nodeId;
    this.slot = slot;
    this.blockHash = blockHash;
    this.signature = signature;
    this.timestamp = timestamp || Date.now();
  }

  /**
   * Create attestation message to sign
   * @returns {string} Message string
   */
  getMessage() {
    return `attest:${this.slot}:${this.blockHash}`;
  }

  /**
   * Sign the attestation (simplified - real impl would use proper crypto)
   * @param {string} privateKey - Node's private key (simplified)
   * @returns {Attestation} This attestation with signature
   */
  sign(privateKey) {
    const message = this.getMessage();
    this.signature = crypto.createHmac('sha256', privateKey)
      .update(message)
      .digest('hex');
    return this;
  }

  /**
   * Verify the attestation signature
   * @param {string} publicKey - Node's public key (simplified)
   * @returns {boolean} True if valid
   */
  verify(publicKey) {
    if (!this.signature) return false;

    const message = this.getMessage();
    const expected = crypto.createHmac('sha256', publicKey)
      .update(message)
      .digest('hex');

    return this.signature === expected;
  }

  toJSON() {
    return {
      nodeId: this.nodeId,
      slot: this.slot,
      blockHash: this.blockHash,
      signature: this.signature,
      timestamp: this.timestamp,
    };
  }

  static fromJSON(json) {
    return new Attestation(json);
  }
}

/**
 * Judgment Reference - minimal judgment data for block inclusion
 */
export class JudgmentRef {
  constructor({
    id,
    cid,
    qScore,
    verdict,
    timestamp,
  }) {
    this.id = id;
    this.qScore = qScore;
    this.verdict = verdict;
    this.timestamp = timestamp || Date.now();
    // Auto-generate CID if not provided
    this.cid = cid || createCID(JSON.stringify({
      id: this.id,
      qScore: this.qScore,
      verdict: this.verdict,
      timestamp: this.timestamp,
    }));
  }

  toJSON() {
    return {
      id: this.id,
      cid: this.cid,
      qScore: this.qScore,
      verdict: this.verdict,
      timestamp: this.timestamp,
    };
  }

  static fromJSON(json) {
    return new JudgmentRef(json);
  }
}

/**
 * PoJ Block - complete block in the chain
 */
export class PoJBlock {
  constructor({
    header,
    judgments = [],
    attestations = [],
    finalized = false,
  }) {
    this.header = header instanceof PoJBlockHeader
      ? header
      : new PoJBlockHeader(header);

    this.judgments = judgments.map(j =>
      j instanceof JudgmentRef ? j : new JudgmentRef(j)
    );

    this.attestations = attestations.map(a =>
      a instanceof Attestation ? a : new Attestation(a)
    );

    this.finalized = finalized;

    // Computed properties
    this._hash = null;
    this._cid = null;
  }

  /**
   * Get block slot
   */
  get slot() {
    return this.header.slot;
  }

  /**
   * Get block timestamp
   */
  get timestamp() {
    return this.header.timestamp;
  }

  /**
   * Get previous block hash
   */
  get prevHash() {
    return this.header.prevHash;
  }

  /**
   * Get proposer node ID
   */
  get proposer() {
    return this.header.proposer;
  }

  /**
   * Get judgment count
   */
  get judgmentCount() {
    return this.judgments.length;
  }

  /**
   * Compute block hash
   * @returns {string} Block hash (hex)
   */
  get hash() {
    if (!this._hash) {
      this._hash = this.header.hash();
    }
    return this._hash;
  }

  /**
   * Get block CID (for DAG storage)
   * @returns {string} CID
   */
  get cid() {
    if (!this._cid) {
      this._cid = createCID(this.encode());
    }
    return this._cid;
  }

  /**
   * Check if block has reached quorum
   * @param {number} totalNodes - Total active nodes
   * @returns {boolean} True if quorum reached
   */
  hasQuorum(totalNodes) {
    const required = Math.ceil(totalNodes * QUORUM_THRESHOLD);
    return this.attestations.length >= required;
  }

  /**
   * Get quorum percentage
   * @param {number} totalNodes - Total active nodes
   * @returns {number} Percentage of attestations
   */
  getQuorumPercentage(totalNodes) {
    return totalNodes > 0
      ? (this.attestations.length / totalNodes) * 100
      : 0;
  }

  /**
   * Add an attestation
   * @param {Attestation} attestation - Attestation to add
   * @returns {boolean} True if added (not duplicate)
   */
  addAttestation(attestation) {
    // Check for duplicate
    const exists = this.attestations.some(a => a.nodeId === attestation.nodeId);
    if (exists) return false;

    // Verify attestation is for this block
    if (attestation.blockHash !== this.hash) return false;

    this.attestations.push(attestation);
    return true;
  }

  /**
   * Validate block structure
   * @returns {{ valid: boolean, errors: string[] }} Validation result
   */
  validate() {
    const errors = [];

    // Validate header
    if (this.header.slot < 0) {
      errors.push('Invalid slot number');
    }

    if (!this.header.proposer) {
      errors.push('Missing proposer');
    }

    // Validate judgments
    if (this.judgments.length > MAX_JUDGMENTS_PER_BLOCK) {
      errors.push(`Too many judgments (max ${MAX_JUDGMENTS_PER_BLOCK})`);
    }

    // Validate merkle root
    const computedRoot = computeMerkleRoot(this.judgments.map(j => j.cid));
    if (this.header.judgmentsRoot !== computedRoot) {
      errors.push('Judgments merkle root mismatch');
    }

    // Validate attestation signatures would go here
    // (simplified for now)

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * Encode block to CBOR
   * @returns {Buffer} Encoded bytes
   */
  encode() {
    return Buffer.from(encode({
      header: this.header.toJSON(),
      judgments: this.judgments.map(j => j.toJSON()),
      attestations: this.attestations.map(a => a.toJSON()),
      finalized: this.finalized,
    }));
  }

  /**
   * Decode block from CBOR
   * @param {Buffer} bytes - Encoded bytes
   * @returns {PoJBlock} Decoded block
   */
  static decode(bytes) {
    const data = decode(bytes);
    return new PoJBlock({
      header: PoJBlockHeader.fromJSON(data.header),
      judgments: data.judgments.map(j => JudgmentRef.fromJSON(j)),
      attestations: data.attestations.map(a => Attestation.fromJSON(a)),
      finalized: data.finalized,
    });
  }

  /**
   * Convert to JSON
   * @returns {Object} JSON representation
   */
  toJSON() {
    return {
      hash: this.hash,
      cid: this.cid,
      header: this.header.toJSON(),
      judgments: this.judgments.map(j => j.toJSON()),
      attestations: this.attestations.map(a => a.toJSON()),
      finalized: this.finalized,
      judgmentCount: this.judgmentCount,
    };
  }

  /**
   * Create from JSON
   * @param {Object} json - JSON representation
   * @returns {PoJBlock} Block instance
   */
  static fromJSON(json) {
    return new PoJBlock({
      header: json.header,
      judgments: json.judgments,
      attestations: json.attestations,
      finalized: json.finalized,
    });
  }
}

/**
 * Create a genesis block
 * @param {string} proposer - Genesis proposer node ID
 * @returns {PoJBlock} Genesis block
 */
export function createGenesisBlock(proposer) {
  const header = new PoJBlockHeader({
    slot: 0,
    timestamp: Date.now(),
    prevHash: null,
    judgmentsRoot: computeMerkleRoot([]),
    stateRoot: '',
    proposer,
  });

  const block = new PoJBlock({
    header,
    judgments: [],
    attestations: [],
    finalized: true, // Genesis is always finalized
  });

  return block;
}

/**
 * Create a new block
 * @param {Object} options - Block creation options
 * @returns {PoJBlock} New block
 */
export function createBlock({
  slot,
  prevBlock,
  judgments,
  proposer,
  stateRoot = '',
}) {
  const judgmentRefs = judgments.map(j => {
    const id = j.id || j.data?.id;
    const qScore = j.qScore || j.data?.qScore;
    const verdict = j.verdict || j.data?.verdict;
    const timestamp = j.timestamp || Date.now();

    // Auto-generate CID if not provided
    const cid = j.cid || createCID(JSON.stringify({ id, qScore, verdict, timestamp }));

    return new JudgmentRef({ id, cid, qScore, verdict, timestamp });
  });

  const header = new PoJBlockHeader({
    slot,
    timestamp: Date.now(),
    prevHash: prevBlock?.hash || null,
    judgmentsRoot: computeMerkleRoot(judgmentRefs.map(j => j.cid)),
    stateRoot,
    proposer,
  });

  return new PoJBlock({
    header,
    judgments: judgmentRefs,
    attestations: [],
    finalized: false,
  });
}

// Export constants
export const POJ_CONSTANTS = {
  PHI,
  PHI_INV,
  SLOT_DURATION_MS,
  MAX_JUDGMENTS_PER_BLOCK,
  QUORUM_THRESHOLD,
};

export default {
  PoJBlockHeader,
  PoJBlock,
  Attestation,
  JudgmentRef,
  computeMerkleRoot,
  createGenesisBlock,
  createBlock,
  POJ_CONSTANTS,
};
