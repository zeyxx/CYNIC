/**
 * OperatorRegistry Tests
 *
 * "Many dogs, one pack" - multi-operator consensus
 *
 * @module @cynic/mcp/test/operator-registry
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import {
  OperatorRegistry,
  generateOperatorKeyPair,
  signWithOperatorKey,
  verifyOperatorSignature,
} from '../src/operator-registry.js';

describe('Operator Key Functions', () => {
  describe('generateOperatorKeyPair', () => {
    it('generates valid key pair', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();

      assert.ok(publicKey);
      assert.ok(privateKey);
      assert.ok(publicKey.length > 0);
      assert.ok(privateKey.length > 0);
    });

    it('generates unique key pairs', () => {
      const keys = new Set();
      for (let i = 0; i < 10; i++) {
        const { publicKey } = generateOperatorKeyPair();
        keys.add(publicKey);
      }
      assert.equal(keys.size, 10);
    });
  });

  describe('signWithOperatorKey', () => {
    it('signs data successfully', () => {
      const { privateKey } = generateOperatorKeyPair();
      const data = 'test data to sign';

      const signature = signWithOperatorKey(data, privateKey);

      assert.ok(signature);
      assert.ok(signature.length > 0);
    });

    it('signs buffer data', () => {
      const { privateKey } = generateOperatorKeyPair();
      const data = Buffer.from('test buffer data');

      const signature = signWithOperatorKey(data, privateKey);

      assert.ok(signature);
    });
  });

  describe('verifyOperatorSignature', () => {
    it('verifies valid signature', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      const data = 'test message';
      const signature = signWithOperatorKey(data, privateKey);

      const valid = verifyOperatorSignature(data, signature, publicKey);

      assert.equal(valid, true);
    });

    it('rejects tampered data', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      const data = 'original message';
      const signature = signWithOperatorKey(data, privateKey);

      const valid = verifyOperatorSignature('tampered message', signature, publicKey);

      assert.equal(valid, false);
    });

    it('rejects wrong public key', () => {
      const keys1 = generateOperatorKeyPair();
      const keys2 = generateOperatorKeyPair();
      const data = 'test message';
      const signature = signWithOperatorKey(data, keys1.privateKey);

      const valid = verifyOperatorSignature(data, signature, keys2.publicKey);

      assert.equal(valid, false);
    });

    it('rejects invalid signature format', () => {
      const { publicKey } = generateOperatorKeyPair();
      const data = 'test message';

      const valid = verifyOperatorSignature(data, 'invalid-signature', publicKey);

      assert.equal(valid, false);
    });
  });
});

describe('OperatorRegistry', () => {
  let registry;

  beforeEach(() => {
    registry = new OperatorRegistry({
      minOperators: 1,
      maxOperators: 10,
    });
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      const r = new OperatorRegistry();
      assert.equal(r.minOperators, 1);
      assert.equal(r.maxOperators, 100);
    });

    it('accepts custom options', () => {
      assert.equal(registry.minOperators, 1);
      assert.equal(registry.maxOperators, 10);
    });
  });

  describe('initializeSelf', () => {
    it('generates keys if not provided', () => {
      const self = registry.initializeSelf();

      assert.ok(self.publicKey);
      assert.ok(self.name);
    });

    it('uses provided keys', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      const self = registry.initializeSelf({
        publicKey,
        privateKey,
        name: 'test-operator',
      });

      assert.equal(self.publicKey, publicKey);
      assert.equal(self.name, 'test-operator');
    });

    it('registers self as operator', () => {
      const self = registry.initializeSelf();

      assert.equal(registry.isOperator(self.publicKey), true);
    });
  });

  describe('getSelf', () => {
    it('returns null before initialization', () => {
      assert.equal(registry.getSelf(), null);
    });

    it('returns self info after initialization', () => {
      registry.initializeSelf({ name: 'my-node' });

      const self = registry.getSelf();

      assert.ok(self);
      assert.equal(self.name, 'my-node');
    });

    it('does not expose private key', () => {
      registry.initializeSelf();

      const self = registry.getSelf();

      assert.ok(!self.privateKey);
    });
  });

  describe('sign', () => {
    it('throws if self not initialized', () => {
      assert.throws(() => {
        registry.sign('test data');
      }, /Self operator not initialized/);
    });

    it('signs data as self', () => {
      registry.initializeSelf();

      const signature = registry.sign('test data');

      assert.ok(signature);
      assert.ok(signature.length > 0);
    });
  });

  describe('registerOperator', () => {
    it('registers new operator', () => {
      const { publicKey } = generateOperatorKeyPair();

      const result = registry.registerOperator({
        publicKey,
        name: 'remote-node',
        weight: 2,
      });

      assert.equal(result, true);
      assert.equal(registry.isOperator(publicKey), true);
    });

    it('returns false for existing operator', () => {
      const { publicKey } = generateOperatorKeyPair();
      registry.registerOperator({ publicKey });

      const result = registry.registerOperator({ publicKey, name: 'updated' });

      assert.equal(result, false);
    });

    it('throws without public key', () => {
      assert.throws(() => {
        registry.registerOperator({ name: 'no-key' });
      }, /public key required/);
    });

    it('throws when max operators reached', () => {
      // Fill up to max
      for (let i = 0; i < 10; i++) {
        const { publicKey } = generateOperatorKeyPair();
        registry.registerOperator({ publicKey });
      }

      const { publicKey } = generateOperatorKeyPair();
      assert.throws(() => {
        registry.registerOperator({ publicKey });
      }, /Maximum operators/);
    });
  });

  describe('removeOperator', () => {
    it('removes registered operator', () => {
      const { publicKey } = generateOperatorKeyPair();
      registry.registerOperator({ publicKey });

      const result = registry.removeOperator(publicKey);

      assert.equal(result, true);
      assert.equal(registry.isOperator(publicKey), false);
    });

    it('returns false for unknown operator', () => {
      const result = registry.removeOperator('unknown-key');
      assert.equal(result, false);
    });

    it('throws when removing self', () => {
      const self = registry.initializeSelf();

      assert.throws(() => {
        registry.removeOperator(self.publicKey);
      }, /Cannot remove self/);
    });
  });

  describe('getOperator', () => {
    it('returns operator info', () => {
      const { publicKey } = generateOperatorKeyPair();
      registry.registerOperator({ publicKey, name: 'test', weight: 3 });

      const operator = registry.getOperator(publicKey);

      assert.equal(operator.publicKey, publicKey);
      assert.equal(operator.name, 'test');
      assert.equal(operator.weight, 3);
    });

    it('returns null for unknown operator', () => {
      const operator = registry.getOperator('unknown');
      assert.equal(operator, null);
    });
  });

  describe('getAllOperators', () => {
    it('returns empty array initially', () => {
      const operators = registry.getAllOperators();
      assert.equal(operators.length, 0);
    });

    it('returns all registered operators', () => {
      const keys1 = generateOperatorKeyPair();
      const keys2 = generateOperatorKeyPair();
      registry.registerOperator({ publicKey: keys1.publicKey });
      registry.registerOperator({ publicKey: keys2.publicKey });

      const operators = registry.getAllOperators();

      assert.equal(operators.length, 2);
    });
  });

  describe('verifySignature', () => {
    it('verifies signature from registered operator', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      registry.registerOperator({ publicKey });

      const data = 'test message';
      const signature = signWithOperatorKey(data, privateKey);

      const valid = registry.verifySignature(data, signature, publicKey);

      assert.equal(valid, true);
    });

    it('rejects signature from unregistered operator', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      // NOT registered

      const data = 'test message';
      const signature = signWithOperatorKey(data, privateKey);

      const valid = registry.verifySignature(data, signature, publicKey);

      assert.equal(valid, false);
    });

    it('updates stats on verification', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      registry.registerOperator({ publicKey });
      const data = 'test';
      const signature = signWithOperatorKey(data, privateKey);

      registry.verifySignature(data, signature, publicKey);

      const stats = registry.getStats();
      assert.equal(stats.signaturesVerified, 1);
    });
  });

  describe('signBlock', () => {
    it('signs block with operator key', () => {
      registry.initializeSelf();

      const block = {
        slot: 1,
        prev_hash: 'abc123',
        judgments_root: 'def456',
        timestamp: Date.now(),
      };

      const signedBlock = registry.signBlock(block);

      assert.ok(signedBlock.operator);
      assert.ok(signedBlock.operator_name);
      assert.ok(signedBlock.signature);
    });

    it('preserves block data', () => {
      registry.initializeSelf();

      const block = {
        slot: 42,
        prev_hash: 'abc',
        judgments_root: 'xyz',
        timestamp: 12345,
        judgments: [{ id: 1 }],
      };

      const signedBlock = registry.signBlock(block);

      assert.equal(signedBlock.slot, 42);
      assert.equal(signedBlock.prev_hash, 'abc');
      assert.deepEqual(signedBlock.judgments, [{ id: 1 }]);
    });
  });

  describe('verifyBlock', () => {
    it('verifies valid signed block', () => {
      registry.initializeSelf();

      const block = {
        slot: 1,
        prev_hash: 'abc',
        judgments_root: 'def',
        timestamp: Date.now(),
      };

      const signedBlock = registry.signBlock(block);
      const result = registry.verifyBlock(signedBlock);

      assert.equal(result.valid, true);
    });

    it('rejects block without operator', () => {
      const block = { slot: 1, prev_hash: 'abc', judgments_root: 'def' };

      const result = registry.verifyBlock(block);

      assert.equal(result.valid, false);
      assert.ok(result.error.includes('no operator'));
    });

    it('rejects block without signature', () => {
      registry.initializeSelf();

      const block = {
        slot: 1,
        prev_hash: 'abc',
        judgments_root: 'def',
        operator: 'some-key',
      };

      const result = registry.verifyBlock(block);

      assert.equal(result.valid, false);
      assert.ok(result.error.includes('no signature'));
    });

    it('rejects block from unknown operator', () => {
      const { publicKey, privateKey } = generateOperatorKeyPair();
      // NOT registered

      const block = {
        slot: 1,
        prev_hash: 'abc',
        judgments_root: 'def',
        timestamp: Date.now(),
        operator: publicKey,
        signature: signWithOperatorKey(
          JSON.stringify({ slot: 1, prev_hash: 'abc', judgments_root: 'def', timestamp: Date.now() }),
          privateKey
        ),
      };

      const result = registry.verifyBlock(block);

      assert.equal(result.valid, false);
      assert.ok(result.error.includes('Unknown operator'));
    });

    it('rejects block with invalid signature', () => {
      registry.initializeSelf();

      const signedBlock = registry.signBlock({
        slot: 1,
        prev_hash: 'abc',
        judgments_root: 'def',
        timestamp: Date.now(),
      });

      // Tamper with signature
      signedBlock.signature = signedBlock.signature.slice(0, -4) + 'xxxx';

      const result = registry.verifyBlock(signedBlock);

      assert.equal(result.valid, false);
      assert.ok(result.error.includes('Invalid'));
    });

    it('updates operator stats on valid block', () => {
      registry.initializeSelf();
      const self = registry.getSelf();

      const signedBlock = registry.signBlock({
        slot: 1,
        prev_hash: 'abc',
        judgments_root: 'def',
        timestamp: Date.now(),
      });

      registry.verifyBlock(signedBlock);

      const operator = registry.getOperator(self.publicKey);
      assert.equal(operator.blocksProposed, 1);
    });
  });

  describe('getTotalWeight', () => {
    it('returns 1 when empty', () => {
      assert.equal(registry.getTotalWeight(), 1);
    });

    it('sums operator weights', () => {
      const k1 = generateOperatorKeyPair();
      const k2 = generateOperatorKeyPair();
      registry.registerOperator({ publicKey: k1.publicKey, weight: 3 });
      registry.registerOperator({ publicKey: k2.publicKey, weight: 5 });

      assert.equal(registry.getTotalWeight(), 8);
    });
  });

  describe('hasQuorum', () => {
    it('returns false when below minimum', () => {
      const r = new OperatorRegistry({ minOperators: 3 });
      assert.equal(r.hasQuorum(), false);
    });

    it('returns true when at minimum', () => {
      const r = new OperatorRegistry({ minOperators: 2 });
      const k1 = generateOperatorKeyPair();
      const k2 = generateOperatorKeyPair();
      r.registerOperator({ publicKey: k1.publicKey });
      r.registerOperator({ publicKey: k2.publicKey });

      assert.equal(r.hasQuorum(), true);
    });
  });

  describe('getStats', () => {
    it('returns comprehensive stats', () => {
      registry.initializeSelf();
      const k = generateOperatorKeyPair();
      registry.registerOperator({ publicKey: k.publicKey });

      const stats = registry.getStats();

      assert.equal(stats.operatorCount, 2);
      assert.equal(stats.selfInitialized, true);
      assert.ok('maxConfidence' in stats);
    });
  });

  describe('exportOperators/importOperators', () => {
    it('exports operator list', () => {
      const k1 = generateOperatorKeyPair();
      const k2 = generateOperatorKeyPair();
      registry.registerOperator({ publicKey: k1.publicKey, name: 'op1' });
      registry.registerOperator({ publicKey: k2.publicKey, name: 'op2' });

      const exported = registry.exportOperators();

      assert.equal(exported.length, 2);
      assert.ok(exported.some(o => o.name === 'op1'));
      assert.ok(exported.some(o => o.name === 'op2'));
    });

    it('imports operator list', () => {
      const k1 = generateOperatorKeyPair();
      const k2 = generateOperatorKeyPair();
      const operators = [
        { publicKey: k1.publicKey, name: 'imported1', weight: 2 },
        { publicKey: k2.publicKey, name: 'imported2', weight: 3 },
      ];

      const result = registry.importOperators(operators);

      assert.equal(result.imported, 2);
      assert.equal(registry.getAllOperators().length, 2);
    });

    it('skips existing operators on import', () => {
      const k1 = generateOperatorKeyPair();
      registry.registerOperator({ publicKey: k1.publicKey, name: 'existing' });

      const result = registry.importOperators([
        { publicKey: k1.publicKey, name: 'updated' },
      ]);

      assert.equal(result.imported, 0);
      assert.equal(result.skipped, 1);
    });
  });
});

describe('OperatorRegistry Events', () => {
  it('emits operator:self:initialized', () => {
    const registry = new OperatorRegistry();
    let emitted = false;

    registry.on('operator:self:initialized', () => {
      emitted = true;
    });

    registry.initializeSelf();

    assert.equal(emitted, true);
  });

  it('emits operator:registered', () => {
    const registry = new OperatorRegistry();
    let emitted = false;

    registry.on('operator:registered', () => {
      emitted = true;
    });

    const { publicKey } = generateOperatorKeyPair();
    registry.registerOperator({ publicKey });

    assert.equal(emitted, true);
  });

  it('emits operator:removed', () => {
    const registry = new OperatorRegistry();
    let emitted = false;
    const { publicKey } = generateOperatorKeyPair();
    registry.registerOperator({ publicKey });

    registry.on('operator:removed', () => {
      emitted = true;
    });

    registry.removeOperator(publicKey);

    assert.equal(emitted, true);
  });
});
