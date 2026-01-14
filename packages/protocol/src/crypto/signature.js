/**
 * Ed25519 Signature Utilities
 *
 * Cryptographic signing for block verification
 *
 * @module @cynic/protocol/crypto/signature
 */

'use strict';

import { generateKeyPairSync, sign, verify, createPrivateKey, createPublicKey } from 'crypto';
import { sha256 } from './hash.js';

/**
 * Generate Ed25519 keypair
 * @returns {{ publicKey: string, privateKey: string }} Keypair (hex encoded)
 */
export function generateKeypair() {
  const { publicKey, privateKey } = generateKeyPairSync('ed25519', {
    publicKeyEncoding: { type: 'spki', format: 'der' },
    privateKeyEncoding: { type: 'pkcs8', format: 'der' },
  });

  return {
    publicKey: publicKey.toString('hex'),
    privateKey: privateKey.toString('hex'),
  };
}

/**
 * Sign data with Ed25519 private key
 * @param {string|Buffer} data - Data to sign
 * @param {string} privateKeyHex - Private key (hex encoded DER)
 * @returns {string} Signature (hex encoded)
 */
export function signData(data, privateKeyHex) {
  const privateKey = createPrivateKey({
    key: Buffer.from(privateKeyHex, 'hex'),
    format: 'der',
    type: 'pkcs8',
  });

  const dataBuffer = typeof data === 'string' ? Buffer.from(data) : data;
  const signature = sign(null, dataBuffer, privateKey);
  return signature.toString('hex');
}

/**
 * Verify Ed25519 signature
 * @param {string|Buffer} data - Original data
 * @param {string} signatureHex - Signature (hex encoded)
 * @param {string} publicKeyHex - Public key (hex encoded DER)
 * @returns {boolean} True if valid
 */
export function verifySignature(data, signatureHex, publicKeyHex) {
  try {
    const publicKey = createPublicKey({
      key: Buffer.from(publicKeyHex, 'hex'),
      format: 'der',
      type: 'spki',
    });

    const dataBuffer = typeof data === 'string' ? Buffer.from(data) : data;
    const signatureBuffer = Buffer.from(signatureHex, 'hex');

    return verify(null, dataBuffer, publicKey, signatureBuffer);
  } catch {
    return false;
  }
}

/**
 * Sign a block
 * @param {Object} block - Block to sign
 * @param {string} privateKeyHex - Private key
 * @returns {string} Block signature
 */
export function signBlock(block, privateKeyHex) {
  // Create signable content (exclude signature field)
  const { operator_sig, ...signableContent } = block;
  const contentHash = sha256(JSON.stringify(signableContent, Object.keys(signableContent).sort()));
  return signData(contentHash, privateKeyHex);
}

/**
 * Verify block signature
 * @param {Object} block - Block with signature
 * @returns {boolean} True if valid
 */
export function verifyBlock(block) {
  const { operator_sig, ...signableContent } = block;

  if (!operator_sig || !block.operator) {
    return false;
  }

  // Extract public key from ed25519:pubkey format
  const publicKeyHex = block.operator.startsWith('ed25519:')
    ? block.operator.slice(8)
    : block.operator;

  const contentHash = sha256(JSON.stringify(signableContent, Object.keys(signableContent).sort()));
  return verifySignature(contentHash, operator_sig, publicKeyHex);
}

/**
 * Format public key with prefix
 * @param {string} publicKeyHex - Public key hex
 * @returns {string} Prefixed public key
 */
export function formatPublicKey(publicKeyHex) {
  return `ed25519:${publicKeyHex}`;
}

export default {
  generateKeypair,
  signData,
  verifySignature,
  signBlock,
  verifyBlock,
  formatPublicKey,
};
