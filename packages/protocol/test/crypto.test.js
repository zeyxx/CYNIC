/**
 * Crypto Utilities Tests
 */

import { describe, it } from 'node:test';
import assert from 'node:assert';

import {
  sha256,
  sha256Prefixed,
  chainHash,
  phiSaltedHash,
  randomHex,
  merkleRoot,
  hashObject,
  generateKeypair,
  signData,
  verifySignature,
  signBlock,
  verifyBlock,
  formatPublicKey,
} from '../src/index.js';

describe('Hash Utilities', () => {
  it('should compute SHA-256 hash', () => {
    const hash = sha256('hello');
    assert.strictEqual(hash.length, 64); // 256 bits = 64 hex chars
    assert.strictEqual(
      hash,
      '2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824'
    );
  });

  it('should compute prefixed SHA-256 hash', () => {
    const hash = sha256Prefixed('hello');
    assert.ok(hash.startsWith('sha256:'));
    assert.strictEqual(hash.length, 64 + 7); // sha256: prefix
  });

  it('should chain hashes', () => {
    const hash1 = sha256('a');
    const hash2 = chainHash(hash1, 'b');
    assert.notStrictEqual(hash1, hash2);
    assert.strictEqual(hash2.length, 64);
  });

  it('should generate φ-salted hash', () => {
    const hash1 = phiSaltedHash('secret');
    const hash2 = phiSaltedHash('secret');
    assert.strictEqual(hash1, hash2); // Deterministic

    const hash3 = phiSaltedHash('secret', 'extra-salt');
    assert.notStrictEqual(hash1, hash3);
  });

  it('should generate random hex', () => {
    const hex1 = randomHex(32);
    const hex2 = randomHex(32);
    assert.strictEqual(hex1.length, 64);
    assert.notStrictEqual(hex1, hex2);
  });

  it('should compute merkle root', () => {
    const hashes = ['a', 'b', 'c', 'd'].map(sha256);
    const root = merkleRoot(hashes);
    assert.strictEqual(root.length, 64);

    // Same input = same root
    const root2 = merkleRoot(hashes);
    assert.strictEqual(root, root2);
  });

  it('should handle empty merkle root', () => {
    const root = merkleRoot([]);
    assert.strictEqual(root, sha256('empty'));
  });

  it('should handle single item merkle root', () => {
    const hash = sha256('single');
    const root = merkleRoot([hash]);
    assert.strictEqual(root, hash);
  });

  it('should hash objects deterministically', () => {
    const obj = { b: 2, a: 1 };
    const hash1 = hashObject(obj);
    const hash2 = hashObject({ a: 1, b: 2 });
    assert.strictEqual(hash1, hash2); // Order independent
  });
});

describe('Signature Utilities', () => {
  it('should generate Ed25519 keypair', () => {
    const { publicKey, privateKey } = generateKeypair();
    assert.ok(publicKey.length > 0);
    assert.ok(privateKey.length > 0);
  });

  it('should sign and verify data', () => {
    const { publicKey, privateKey } = generateKeypair();
    const data = 'test message';

    const signature = signData(data, privateKey);
    assert.ok(signature.length > 0);

    const valid = verifySignature(data, signature, publicKey);
    assert.strictEqual(valid, true);
  });

  it('should reject invalid signatures', () => {
    const keypair1 = generateKeypair();
    const keypair2 = generateKeypair();

    const signature = signData('test', keypair1.privateKey);
    const valid = verifySignature('test', signature, keypair2.publicKey);
    assert.strictEqual(valid, false);
  });

  it('should reject tampered data', () => {
    const { publicKey, privateKey } = generateKeypair();
    const signature = signData('original', privateKey);
    const valid = verifySignature('tampered', signature, publicKey);
    assert.strictEqual(valid, false);
  });

  it('should format public key with prefix', () => {
    const { publicKey } = generateKeypair();
    const formatted = formatPublicKey(publicKey);
    assert.ok(formatted.startsWith('ed25519:'));
  });

  it('should sign and verify blocks', () => {
    const { publicKey, privateKey } = generateKeypair();

    const block = {
      slot: 1,
      prev_hash: 'sha256:abc',
      timestamp: Date.now(),
      operator: formatPublicKey(publicKey),
    };

    block.operator_sig = signBlock(block, privateKey);
    assert.ok(block.operator_sig.length > 0);

    const valid = verifyBlock(block);
    assert.strictEqual(valid, true);
  });
});
