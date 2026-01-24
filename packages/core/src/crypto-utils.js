/**
 * CYNIC Cryptographic Utilities
 *
 * Secure random generation for IDs, nonces, and tokens.
 * Uses crypto.randomBytes() instead of Math.random().
 *
 * @module @cynic/core/crypto-utils
 */

'use strict';

import { randomBytes } from 'crypto';

/**
 * Generate cryptographically secure random number in [0, 1)
 * @returns {number} Secure random value
 */
export function secureRandom() {
  const bytes = randomBytes(4);
  const value = bytes.readUInt32BE(0);
  return value / 0x100000000; // Divide by 2^32
}

/**
 * Generate cryptographically secure random bytes as hex string
 * @param {number} [length=16] - Number of bytes
 * @returns {string} Hex-encoded random bytes
 */
export function secureRandomHex(length = 16) {
  return randomBytes(length).toString('hex');
}

/**
 * Generate cryptographically secure random bytes as base36 string
 * @param {number} [length=8] - Approximate output length
 * @returns {string} Base36-encoded random string
 */
export function secureRandomBase36(length = 8) {
  // Each byte gives ~1.5 base36 chars, so we need length/1.5 bytes
  const bytes = randomBytes(Math.ceil(length * 0.75));
  // Convert to BigInt then to base36
  let num = BigInt('0x' + bytes.toString('hex'));
  let result = '';
  while (num > 0n && result.length < length) {
    result = '0123456789abcdefghijklmnopqrstuvwxyz'[Number(num % 36n)] + result;
    num = num / 36n;
  }
  return result.padStart(length, '0').slice(-length);
}

/**
 * Generate a secure unique ID with prefix
 * @param {string} [prefix='id'] - ID prefix
 * @param {number} [randomLength=8] - Length of random suffix
 * @returns {string} Unique ID like "prefix_timestamp_random"
 */
export function secureId(prefix = 'id', randomLength = 8) {
  const timestamp = Date.now().toString(36);
  const random = secureRandomBase36(randomLength);
  return `${prefix}_${timestamp}_${random}`;
}

/**
 * Generate a cryptographically secure nonce
 * @param {number} [length=16] - Nonce length in bytes
 * @returns {string} Hex-encoded nonce
 */
export function secureNonce(length = 16) {
  return randomBytes(length).toString('hex');
}

/**
 * Generate a secure token for sessions/locks
 * @returns {string} Secure token
 */
export function secureToken() {
  return `${Date.now()}-${secureRandomHex(16)}`;
}
