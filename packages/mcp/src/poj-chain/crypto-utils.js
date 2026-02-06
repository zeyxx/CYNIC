/**
 * PoJ Chain Cryptographic Utilities
 *
 * SHA-256 hashing and Merkle tree computation.
 * "The chain remembers" - κυνικός
 *
 * @module @cynic/mcp/poj-chain/crypto-utils
 */

'use strict';

import crypto from 'crypto';

/**
 * Simple SHA-256 hash
 * @param {string|Object} data - Data to hash
 * @returns {string} Hex-encoded hash
 */
export function sha256(data) {
  return crypto.createHash('sha256')
    .update(typeof data === 'string' ? data : JSON.stringify(data))
    .digest('hex');
}

/**
 * Calculate Merkle root from array of hashes
 * @param {string[]} hashes - Array of hex-encoded hashes
 * @returns {string} Merkle root hash
 */
export function merkleRoot(hashes) {
  if (hashes.length === 0) return sha256('empty');
  if (hashes.length === 1) return hashes[0];

  const pairs = [];
  for (let i = 0; i < hashes.length; i += 2) {
    const left = hashes[i];
    const right = hashes[i + 1] || left; // Duplicate last if odd
    pairs.push(sha256(left + right));
  }
  return merkleRoot(pairs);
}
