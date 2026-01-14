/**
 * Layer 2: Merkle Knowledge Tree Tests
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  MerkleTree,
  KnowledgeTree,
  createPattern,
  createLearning,
  validatePatternEmergence,
  checkPatternFormation,
  mergePatterns,
  calculatePatternDecay,
  hashObject,
} from '../src/index.js';

import { MIN_PATTERN_SOURCES, PHI_INV } from '@cynic/core';

describe('Merkle Tree', () => {
  it('should build tree from items', () => {
    const items = ['a', 'b', 'c', 'd'];
    const tree = new MerkleTree(items, (x) => hashObject(x));

    assert.strictEqual(tree.size(), 4);
    assert.ok(tree.getRoot());
  });

  it('should compute consistent root', () => {
    const items = ['x', 'y', 'z'];
    const tree1 = new MerkleTree(items, hashObject);
    const tree2 = new MerkleTree(items, hashObject);

    assert.strictEqual(tree1.getRoot(), tree2.getRoot());
  });

  it('should get prefixed root', () => {
    const tree = new MerkleTree(['a'], hashObject);
    const root = tree.getRootPrefixed();
    assert.ok(root.startsWith('sha256:'));
  });

  it('should generate proof of inclusion', () => {
    const items = ['a', 'b', 'c', 'd'];
    const tree = new MerkleTree(items, hashObject);

    const proof = tree.getProof('b');
    assert.ok(proof);
    assert.ok(Array.isArray(proof));
  });

  it('should return null proof for missing item', () => {
    const tree = new MerkleTree(['a', 'b'], hashObject);
    const proof = tree.getProof('missing');
    assert.strictEqual(proof, null);
  });

  it('should verify proof of inclusion', () => {
    const items = ['a', 'b', 'c', 'd'];
    const tree = new MerkleTree(items, hashObject);

    const proof = tree.getProof('c');
    const root = tree.getRoot();

    const valid = MerkleTree.verifyProof('c', proof, root, hashObject);
    assert.strictEqual(valid, true);
  });

  it('should reject invalid proof', () => {
    const items = ['a', 'b', 'c', 'd'];
    const tree = new MerkleTree(items, hashObject);

    const proof = tree.getProof('c');
    const root = tree.getRoot();

    // Verify with wrong item
    const valid = MerkleTree.verifyProof('wrong', proof, root, hashObject);
    assert.strictEqual(valid, false);
  });

  it('should add items to tree', () => {
    const tree = new MerkleTree(['a', 'b'], hashObject);
    const root1 = tree.getRoot();

    tree.add('c');
    const root2 = tree.getRoot();

    assert.strictEqual(tree.size(), 3);
    assert.notStrictEqual(root1, root2);
  });

  it('should get all items', () => {
    const items = ['x', 'y', 'z'];
    const tree = new MerkleTree(items, hashObject);

    const retrieved = tree.getItems();
    assert.deepStrictEqual(retrieved, items);
  });

  it('should handle empty tree', () => {
    const tree = new MerkleTree([], hashObject);
    assert.strictEqual(tree.size(), 0);
    assert.ok(tree.getRoot()); // Should still have a root (empty hash)
  });

  it('should handle single item tree', () => {
    const tree = new MerkleTree(['single'], hashObject);
    assert.strictEqual(tree.size(), 1);

    const proof = tree.getProof('single');
    assert.ok(proof);
  });
});

describe('Knowledge Tree', () => {
  let kTree;

  beforeEach(() => {
    kTree = new KnowledgeTree();
  });

  it('should add patterns by axiom', () => {
    kTree.addPattern({ id: 'pat_1', axiom: 'PHI', content: 'test' });
    kTree.addPattern({ id: 'pat_2', axiom: 'VERIFY', content: 'test2' });

    const phiPatterns = kTree.getPatterns('PHI');
    const verifyPatterns = kTree.getPatterns('VERIFY');

    assert.strictEqual(phiPatterns.length, 1);
    assert.strictEqual(verifyPatterns.length, 1);
  });

  it('should add learnings by axiom', () => {
    kTree.addLearning({ id: 'lrn_1', axiom: 'CULTURE', type: 'insight' });

    const learnings = kTree.getLearnings('CULTURE');
    assert.strictEqual(learnings.length, 1);
  });

  it('should get all patterns', () => {
    kTree.addPattern({ id: 'pat_1', axiom: 'PHI' });
    kTree.addPattern({ id: 'pat_2', axiom: 'BURN' });

    const all = kTree.getAllPatterns();
    assert.strictEqual(all.length, 2);
  });

  it('should get axiom root', () => {
    kTree.addPattern({ id: 'pat_1', axiom: 'VERIFY' });

    const root = kTree.getAxiomRoot('VERIFY');
    assert.ok(root.patterns);
    assert.ok(root.learnings);
  });

  it('should get global root', () => {
    kTree.addPattern({ id: 'pat_1', axiom: 'PHI' });
    const root1 = kTree.getGlobalRoot();

    kTree.addPattern({ id: 'pat_2', axiom: 'BURN' });
    const root2 = kTree.getGlobalRoot();

    assert.notStrictEqual(root1, root2);
  });

  it('should generate pattern proof', () => {
    const pattern = { id: 'pat_1', axiom: 'CULTURE', content: 'test' };
    kTree.addPattern(pattern);

    const proof = kTree.getPatternProof(pattern);
    assert.ok(proof);
    assert.strictEqual(proof.axiom, 'CULTURE');
    assert.strictEqual(proof.type, 'pattern');
    assert.ok(proof.globalRoot);
  });

  it('should get statistics', () => {
    kTree.addPattern({ id: 'pat_1', axiom: 'PHI' });
    kTree.addPattern({ id: 'pat_2', axiom: 'PHI' });
    kTree.addLearning({ id: 'lrn_1', axiom: 'PHI' });

    const stats = kTree.getStats();
    assert.strictEqual(stats.PHI.patterns, 2);
    assert.strictEqual(stats.PHI.learnings, 1);
  });

  it('should export and import', () => {
    kTree.addPattern({ id: 'pat_1', axiom: 'VERIFY' });
    kTree.addLearning({ id: 'lrn_1', axiom: 'VERIFY' });

    const exported = kTree.export();
    assert.ok(exported.globalRoot);
    assert.ok(exported.axioms.VERIFY);

    const imported = KnowledgeTree.import(exported);
    assert.strictEqual(imported.getPatterns('VERIFY').length, 1);
  });

  it('should reject invalid axiom', () => {
    assert.throws(() => {
      kTree.addPattern({ id: 'pat_1', axiom: 'INVALID' });
    });
  });
});

describe('Pattern Management', () => {
  it('should create pattern', () => {
    const pattern = createPattern({
      content: 'test content',
      axiom: 'VERIFY',
      strength: 0.8,
      sources: 5,
    });

    assert.ok(pattern.id.startsWith('pat_'));
    assert.ok(pattern.content_hash.startsWith('sha256:'));
    assert.strictEqual(pattern.axiom, 'VERIFY');
    assert.strictEqual(pattern.sources, 5);
  });

  it('should bound strength to φ⁻¹', () => {
    const pattern = createPattern({
      content: 'test',
      axiom: 'PHI',
      strength: 0.9, // Should be capped
      sources: 3,
    });

    assert.ok(pattern.strength <= PHI_INV);
  });

  it('should create learning', () => {
    const learning = createLearning({
      type: 'insight',
      content: 'learned something',
      axiom: 'CULTURE',
      confidence: 0.7,
      contributor: 'user123',
    });

    assert.ok(learning.id.startsWith('lrn_'));
    assert.strictEqual(learning.type, 'insight');
    assert.ok(learning.contributor.startsWith('sha256:')); // Hashed
  });

  it('should validate pattern emergence', () => {
    const validPattern = createPattern({
      content: 'test',
      axiom: 'BURN',
      strength: 0.6,
      sources: MIN_PATTERN_SOURCES,
    });

    const result = validatePatternEmergence(validPattern);
    assert.strictEqual(result.valid, true);
  });

  it('should reject pattern with insufficient sources', () => {
    const pattern = {
      id: 'pat_123',
      content_hash: 'sha256:abc',
      axiom: 'PHI',
      strength: 0.5,
      sources: 1, // Not enough
    };

    const result = validatePatternEmergence(pattern);
    assert.strictEqual(result.valid, false);
    assert.ok(result.errors.some((e) => e.includes('sources')));
  });

  it('should check pattern formation', () => {
    const observations = [
      { source: 'node1' },
      { source: 'node2' },
      { source: 'node3' },
    ];

    const result = checkPatternFormation(observations);
    assert.strictEqual(result.isPattern, true);
    assert.strictEqual(result.sources, 3);
    assert.ok(result.strength > 0);
  });

  it('should reject insufficient observations', () => {
    const observations = [
      { source: 'node1' },
      { source: 'node2' },
    ];

    const result = checkPatternFormation(observations);
    assert.strictEqual(result.isPattern, false);
  });

  it('should merge patterns', () => {
    const patterns = [
      createPattern({ content: 'a', axiom: 'VERIFY', strength: 0.5, sources: 3 }),
      createPattern({ content: 'b', axiom: 'VERIFY', strength: 0.6, sources: 4 }),
    ];

    const merged = mergePatterns(patterns);
    assert.strictEqual(merged.axiom, 'VERIFY');
    assert.strictEqual(merged.sources, 7); // Sum
    assert.ok(merged.strength > 0.5); // Boosted
  });

  it('should calculate pattern decay', () => {
    const pattern = createPattern({
      content: 'test',
      axiom: 'PHI',
      strength: 0.6, // Use a value below PHI_INV cap
      sources: 5,
    });

    // Immediately
    const strength1 = calculatePatternDecay(pattern);
    assert.ok(strength1 > 0.59); // Close to original strength

    // Simulate old pattern (1 week ago = half-life)
    const oldPattern = { ...pattern, created_at: Date.now() - 7 * 24 * 60 * 60 * 1000 };
    const strength2 = calculatePatternDecay(oldPattern);
    assert.ok(strength2 < strength1 / 1.5); // Decayed significantly
  });
});
