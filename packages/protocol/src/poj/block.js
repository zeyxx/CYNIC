/**
 * Proof of Judgment - Block Management
 *
 * Block creation, validation, and chain management
 *
 * @module @cynic/protocol/poj/block
 */

'use strict';

import { SLOT_MS, BLOCK_MS, THRESHOLDS } from '@cynic/core';
import { sha256Prefixed, merkleRoot, hashObject } from '../crypto/hash.js';
import { signBlock, verifyBlock, formatPublicKey } from '../crypto/signature.js';

/**
 * Block types
 */
export const BlockType = {
  JUDGMENT: 'JUDGMENT',
  KNOWLEDGE: 'KNOWLEDGE',
  GOVERNANCE: 'GOVERNANCE',
};

/**
 * Calculate slot number from timestamp
 * @param {number} [timestamp=Date.now()] - Unix timestamp ms
 * @param {number} [genesisTime=0] - Genesis timestamp
 * @returns {number} Slot number
 */
export function calculateSlot(timestamp = Date.now(), genesisTime = 0) {
  return Math.floor((timestamp - genesisTime) / SLOT_MS);
}

/**
 * Get timestamp for slot
 * @param {number} slot - Slot number
 * @param {number} [genesisTime=0] - Genesis timestamp
 * @returns {number} Unix timestamp ms
 */
export function slotToTimestamp(slot, genesisTime = 0) {
  return genesisTime + slot * SLOT_MS;
}

/**
 * Create genesis block
 * @param {string} operatorPublicKey - Operator public key (hex)
 * @param {string} operatorPrivateKey - Operator private key (hex)
 * @param {number} [genesisTime=Date.now()] - Genesis timestamp
 * @returns {Object} Genesis block
 */
export function createGenesisBlock(operatorPublicKey, operatorPrivateKey, genesisTime = Date.now()) {
  const block = {
    slot: 0,
    prev_hash: sha256Prefixed('CYNIC_GENESIS_φ'),
    timestamp: genesisTime,
    type: BlockType.JUDGMENT,
    judgments: [],
    operator: formatPublicKey(operatorPublicKey),
    judgments_root: sha256Prefixed('empty'),
    state_root: sha256Prefixed('genesis'),
  };

  block.operator_sig = signBlock(block, operatorPrivateKey);
  return block;
}

/**
 * Create judgment block
 * @param {Object} params - Block parameters
 * @param {number} params.slot - Slot number
 * @param {string} params.prevHash - Previous block hash
 * @param {Object[]} params.judgments - Array of judgments
 * @param {string} params.operatorPublicKey - Operator public key
 * @param {string} params.operatorPrivateKey - Operator private key
 * @param {string} [params.stateRoot] - Current state root
 * @returns {Object} Judgment block
 */
export function createJudgmentBlock({
  slot,
  prevHash,
  judgments,
  operatorPublicKey,
  operatorPrivateKey,
  stateRoot = null,
  timestamp = Date.now(),
}) {
  // Hash each judgment
  const judgmentHashes = judgments.map((j) => hashObject(j));
  const judgmentsRoot = sha256Prefixed(merkleRoot(judgmentHashes));

  const block = {
    slot,
    prev_hash: prevHash,
    timestamp,
    type: BlockType.JUDGMENT,
    judgments,
    operator: formatPublicKey(operatorPublicKey),
    judgments_root: judgmentsRoot,
    state_root: stateRoot || sha256Prefixed(`state:${slot}`),
  };

  block.operator_sig = signBlock(block, operatorPrivateKey);
  return block;
}

/**
 * Create knowledge block
 * @param {Object} params - Block parameters
 * @param {number} params.slot - Slot number
 * @param {string} params.prevHash - Previous block hash
 * @param {Object[]} params.patterns - Array of patterns
 * @param {Object[]} params.learnings - Array of learnings
 * @param {string} params.operatorPublicKey - Operator public key
 * @param {string} params.operatorPrivateKey - Operator private key
 * @returns {Object} Knowledge block
 */
export function createKnowledgeBlock({
  slot,
  prevHash,
  patterns,
  learnings,
  operatorPublicKey,
  operatorPrivateKey,
  timestamp = Date.now(),
}) {
  const patternHashes = patterns.map((p) => hashObject(p));
  const learningHashes = learnings.map((l) => hashObject(l));

  const block = {
    slot,
    prev_hash: prevHash,
    timestamp,
    type: BlockType.KNOWLEDGE,
    patterns,
    learnings,
    operator: formatPublicKey(operatorPublicKey),
    patterns_root: sha256Prefixed(merkleRoot(patternHashes)),
    learnings_root: sha256Prefixed(merkleRoot(learningHashes)),
  };

  block.operator_sig = signBlock(block, operatorPrivateKey);
  return block;
}

/**
 * Create governance block
 * @param {Object} params - Block parameters
 * @param {number} params.slot - Slot number
 * @param {string} params.prevHash - Previous block hash
 * @param {Object} params.proposal - Governance proposal
 * @param {Object[]} params.votes - Array of votes
 * @param {string} params.operatorPublicKey - Operator public key
 * @param {string} params.operatorPrivateKey - Operator private key
 * @returns {Object} Governance block
 */
export function createGovernanceBlock({
  slot,
  prevHash,
  proposal,
  votes,
  operatorPublicKey,
  operatorPrivateKey,
  timestamp = Date.now(),
}) {
  // Calculate vote result
  const totalWeight = votes.reduce((sum, v) => sum + (v.e_score || 1), 0);
  const approveWeight = votes
    .filter((v) => v.vote === 'APPROVE')
    .reduce((sum, v) => sum + (v.e_score || 1), 0);
  const ratio = totalWeight > 0 ? approveWeight / totalWeight : 0;

  const block = {
    slot,
    prev_hash: prevHash,
    timestamp,
    type: BlockType.GOVERNANCE,
    proposal,
    votes,
    result: {
      total_weight: totalWeight,
      approve_weight: approveWeight,
      ratio,
      status: ratio >= THRESHOLDS.MAX_CONFIDENCE ? 'PASSED' : 'REJECTED',
    },
    operator: formatPublicKey(operatorPublicKey),
  };

  block.operator_sig = signBlock(block, operatorPrivateKey);
  return block;
}

/**
 * Calculate block hash
 * @param {Object} block - Block to hash
 * @returns {string} Block hash (prefixed)
 */
export function hashBlock(block) {
  return sha256Prefixed(hashObject(block));
}

/**
 * Validate block structure
 * @param {Object} block - Block to validate
 * @returns {{ valid: boolean, errors: string[] }} Validation result
 */
export function validateBlockStructure(block) {
  const errors = [];

  // Required fields
  const required = ['slot', 'prev_hash', 'timestamp', 'operator', 'operator_sig'];
  for (const field of required) {
    if (block[field] === undefined) {
      errors.push(`Missing required field: ${field}`);
    }
  }

  // Slot must be non-negative
  if (typeof block.slot !== 'number' || block.slot < 0) {
    errors.push('Invalid slot: must be non-negative integer');
  }

  // Timestamp must be valid
  if (typeof block.timestamp !== 'number' || block.timestamp < 0) {
    errors.push('Invalid timestamp');
  }

  // Verify signature
  if (errors.length === 0 && !verifyBlock(block)) {
    errors.push('Invalid block signature');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Validate block in chain context
 * @param {Object} block - Block to validate
 * @param {Object} prevBlock - Previous block
 * @returns {{ valid: boolean, errors: string[] }} Validation result
 */
export function validateBlockChain(block, prevBlock) {
  const structureResult = validateBlockStructure(block);
  if (!structureResult.valid) {
    return structureResult;
  }

  const errors = [];

  // Slot must be greater than previous
  if (block.slot <= prevBlock.slot) {
    errors.push(`Slot ${block.slot} must be greater than previous ${prevBlock.slot}`);
  }

  // Previous hash must match
  const expectedPrevHash = hashBlock(prevBlock);
  if (block.prev_hash !== expectedPrevHash) {
    errors.push(`Invalid prev_hash: expected ${expectedPrevHash}`);
  }

  // Timestamp must be greater
  if (block.timestamp <= prevBlock.timestamp) {
    errors.push('Timestamp must be greater than previous block');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

export default {
  BlockType,
  calculateSlot,
  slotToTimestamp,
  createGenesisBlock,
  createJudgmentBlock,
  createKnowledgeBlock,
  createGovernanceBlock,
  hashBlock,
  validateBlockStructure,
  validateBlockChain,
};
