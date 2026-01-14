/**
 * Cryptographic Hash Utilities
 *
 * SHA-256 chaining for Proof of Judgment
 *
 * @module @cynic/protocol/crypto/hash
 */

'use strict';

import { createHash, randomBytes } from 'crypto';
import { PHI_INV } from '@cynic/core';

/**
 * Compute SHA-256 hash of data
 * @param {string|Buffer} data - Data to hash
 * @returns {string} Hex-encoded hash
 */
export function sha256(data) {
  return createHash('sha256').update(data).digest('hex');
}

/**
 * Compute SHA-256 hash with prefix
 * @param {string|Buffer} data - Data to hash
 * @returns {string} Prefixed hash (sha256:...)
 */
export function sha256Prefixed(data) {
  return `sha256:${sha256(data)}`;
}

/**
 * Chain two hashes together (for PoJ chain)
 * @param {string} prevHash - Previous hash in chain
 * @param {string} data - New data to chain
 * @returns {string} New chain hash
 */
export function chainHash(prevHash, data) {
  return sha256(`${prevHash}:${data}`);
}

/**
 * Generate φ-salted hash for privacy
 * Uses PHI_INV (61.8%) as salt factor
 * @param {string} data - Data to hash (e.g., PII)
 * @param {string} [salt] - Optional additional salt
 * @returns {string} φ-salted hash
 */
export function phiSaltedHash(data, salt = '') {
  const phiSalt = Math.floor(PHI_INV * 1e10).toString(36);
  return sha256(`${phiSalt}:${salt}:${data}`);
}

/**
 * Generate random bytes
 * @param {number} [size=32] - Number of bytes
 * @returns {string} Hex-encoded random bytes
 */
export function randomHex(size = 32) {
  return randomBytes(size).toString('hex');
}

/**
 * Compute Merkle root of array of hashes
 * @param {string[]} hashes - Array of hashes
 * @returns {string} Merkle root
 */
export function merkleRoot(hashes) {
  if (hashes.length === 0) {
    return sha256('empty');
  }

  if (hashes.length === 1) {
    return hashes[0];
  }

  const nextLevel = [];
  for (let i = 0; i < hashes.length; i += 2) {
    const left = hashes[i];
    const right = hashes[i + 1] || left; // Duplicate if odd
    nextLevel.push(sha256(`${left}:${right}`));
  }

  return merkleRoot(nextLevel);
}

/**
 * Hash an object (deterministic JSON serialization)
 * @param {Object} obj - Object to hash
 * @returns {string} Hash of object
 */
export function hashObject(obj) {
  const sorted = JSON.stringify(obj, Object.keys(obj).sort());
  return sha256(sorted);
}

export default {
  sha256,
  sha256Prefixed,
  chainHash,
  phiSaltedHash,
  randomHex,
  merkleRoot,
  hashObject,
};
