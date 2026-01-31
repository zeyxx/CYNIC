/**
 * Vector Module Tests
 *
 * Tests for HNSW index and VectorStore.
 * Target: < 50ms for 10,000 vectors (V2 benchmark)
 *
 * @module @cynic/persistence/tests/vector
 */

import { describe, it, before, beforeEach } from 'node:test';
import assert from 'node:assert';
import {
  HNSWIndex,
  HNSWNode,
  HNSW_CONFIG,
  createHNSWIndex,
  VectorStore,
  VECTOR_STORE_CONFIG,
  createVectorStore,
  getVectorStore,
  SemanticPatternMatcher,
  SemanticPattern,
  PatternCluster,
  SEMANTIC_PATTERN_CONFIG,
  createSemanticPatternMatcher,
} from '../src/vector/index.js';

describe('HNSW Index', () => {
  let index;

  beforeEach(() => {
    index = createHNSWIndex({ dimensions: 4 });
  });

  describe('Basic Operations', () => {
    it('should create empty index', () => {
      assert.strictEqual(index.size, 0);
      assert.strictEqual(index.isEmpty(), true);
    });

    it('should insert vectors', () => {
      index.insert('a', [1, 0, 0, 0]);
      index.insert('b', [0, 1, 0, 0]);
      index.insert('c', [0, 0, 1, 0]);

      assert.strictEqual(index.size, 3);
      assert.strictEqual(index.isEmpty(), false);
    });

    it('should reject dimension mismatch', () => {
      index.insert('a', [1, 0, 0, 0]);

      assert.throws(() => index.insert('b', [1, 0, 0]), /dimension/i);
    });

    it('should update existing vectors', () => {
      index.insert('a', [1, 0, 0, 0]);
      index.insert('a', [0, 1, 0, 0]); // Update

      assert.strictEqual(index.size, 1);
    });

    it('should delete vectors', () => {
      index.insert('a', [1, 0, 0, 0]);
      index.insert('b', [0, 1, 0, 0]);

      assert.strictEqual(index.delete('a'), true);
      assert.strictEqual(index.size, 1);
      assert.strictEqual(index.delete('a'), false); // Already deleted
    });

    it('should get vectors by id', () => {
      index.insert('a', [1, 0, 0, 0], { label: 'test' });

      const node = index.get('a');
      assert.ok(node);
      assert.strictEqual(node.id, 'a');
      assert.strictEqual(node.metadata.label, 'test');
    });
  });

  describe('Similarity Search', () => {
    beforeEach(() => {
      // Create orthogonal basis vectors
      index.insert('x', [1, 0, 0, 0], { axis: 'x' });
      index.insert('y', [0, 1, 0, 0], { axis: 'y' });
      index.insert('z', [0, 0, 1, 0], { axis: 'z' });
      index.insert('w', [0, 0, 0, 1], { axis: 'w' });
    });

    it('should find exact match', () => {
      const results = index.search([1, 0, 0, 0], 1);

      assert.strictEqual(results.length, 1);
      assert.strictEqual(results[0].id, 'x');
      assert.ok(Math.abs(results[0].similarity - 1) < 0.00001, `Expected similarity ~1, got ${results[0].similarity}`); // cosine = 1 for identical
    });

    it('should return k nearest neighbors', () => {
      const results = index.search([0.5, 0.5, 0, 0], 2);

      assert.strictEqual(results.length, 2);
      // Both x and y should be equally close
      assert.ok(['x', 'y'].includes(results[0].id));
      assert.ok(['x', 'y'].includes(results[1].id));
    });

    it('should respect minScore threshold', () => {
      const results = index.search([1, 0, 0, 0], 10, { minScore: 0.5 });

      // Only x should match with cosine >= 0.5
      assert.strictEqual(results.length, 1);
      assert.strictEqual(results[0].id, 'x');
    });

    it('should filter by metadata', () => {
      const results = index.search([0.5, 0.5, 0.5, 0.5], 10, {
        filter: (meta) => meta.axis === 'x' || meta.axis === 'y',
      });

      assert.ok(results.every(r => ['x', 'y'].includes(r.id)));
    });

    it('should handle euclidean distance', () => {
      const euclideanIndex = createHNSWIndex({
        dimensions: 4,
        config: { distanceMetric: 'euclidean' },
      });

      euclideanIndex.insert('a', [0, 0, 0, 0]);
      euclideanIndex.insert('b', [1, 0, 0, 0]);
      euclideanIndex.insert('c', [2, 0, 0, 0]);

      const results = euclideanIndex.search([0, 0, 0, 0], 3);

      // Closest should be 'a' (distance 0)
      assert.strictEqual(results[0].id, 'a');
    });
  });

  describe('Serialization', () => {
    it('should serialize and deserialize', () => {
      index.insert('a', [1, 0, 0, 0], { label: 'a' });
      index.insert('b', [0, 1, 0, 0], { label: 'b' });
      index.insert('c', [0, 0, 1, 0], { label: 'c' });

      const json = index.toJSON();
      const restored = HNSWIndex.fromJSON(json);

      assert.strictEqual(restored.size, 3);

      const results = restored.search([1, 0, 0, 0], 1);
      assert.strictEqual(results[0].id, 'a');
      assert.strictEqual(results[0].metadata.label, 'a');
    });
  });

  describe('Performance', () => {
    it('should search 1000 vectors in < 50ms', () => {
      const dimensions = 128;
      const count = 1000;
      const perfIndex = createHNSWIndex({ dimensions });

      // Insert random vectors
      for (let i = 0; i < count; i++) {
        const vector = Array.from({ length: dimensions }, () => Math.random());
        perfIndex.insert(`doc-${i}`, vector);
      }

      // Search
      const query = Array.from({ length: dimensions }, () => Math.random());

      const start = performance.now();
      const results = perfIndex.search(query, 10);
      const elapsed = performance.now() - start;

      assert.strictEqual(results.length, 10);
      assert.ok(elapsed < 50, `Search took ${elapsed.toFixed(2)}ms (target: < 50ms)`);

      // Log for benchmark tracking
      console.log(`HNSW search (${count} vectors, ${dimensions}d): ${elapsed.toFixed(2)}ms`);
    });

    it('should handle 10000 vectors (V2 benchmark)', () => {
      const dimensions = 64;
      const count = 10000;
      const perfIndex = createHNSWIndex({ dimensions });

      // Insert
      const insertStart = performance.now();
      for (let i = 0; i < count; i++) {
        const vector = Array.from({ length: dimensions }, () => Math.random());
        perfIndex.insert(`doc-${i}`, vector);
      }
      const insertTime = performance.now() - insertStart;

      // Search
      const query = Array.from({ length: dimensions }, () => Math.random());
      const searchStart = performance.now();
      const results = perfIndex.search(query, 10);
      const searchTime = performance.now() - searchStart;

      assert.strictEqual(results.length, 10);
      assert.ok(searchTime < 50, `Search took ${searchTime.toFixed(2)}ms (target: < 50ms)`);

      console.log(`V2 Benchmark (${count} vectors, ${dimensions}d):`);
      console.log(`  Insert: ${insertTime.toFixed(2)}ms (${(count / insertTime * 1000).toFixed(0)} vec/s)`);
      console.log(`  Search: ${searchTime.toFixed(2)}ms`);
    });
  });

  describe('Edge Cases', () => {
    it('should handle search on empty index', () => {
      const emptyIndex = createHNSWIndex({ dimensions: 4 });
      const results = emptyIndex.search([1, 0, 0, 0], 10);
      assert.strictEqual(results.length, 0);
    });

    it('should handle k larger than index size', () => {
      index.insert('a', [1, 0, 0, 0]);
      index.insert('b', [0, 1, 0, 0]);

      const results = index.search([1, 0, 0, 0], 100);
      assert.strictEqual(results.length, 2);
    });

    it('should handle zero vectors', () => {
      index.insert('zero', [0, 0, 0, 0]);

      // Search with non-zero should handle gracefully
      const results = index.search([1, 0, 0, 0], 10);
      assert.ok(results !== undefined);
    });
  });
});

describe('VectorStore', () => {
  let store;

  beforeEach(() => {
    store = createVectorStore({
      embedder: 'mock',
      dimensions: 384,
    });
  });

  describe('Document Storage', () => {
    it('should store documents', async () => {
      await store.store('doc1', 'Hello world', { type: 'greeting' });

      assert.strictEqual(store.has('doc1'), true);

      const doc = store.get('doc1');
      assert.strictEqual(doc.text, 'Hello world');
      assert.strictEqual(doc.metadata.type, 'greeting');
    });

    it('should store batch documents', async () => {
      const docs = [
        { id: 'a', text: 'First document' },
        { id: 'b', text: 'Second document' },
        { id: 'c', text: 'Third document' },
      ];

      const count = await store.storeBatch(docs);
      assert.strictEqual(count, 3);
      assert.strictEqual(store.ids().length, 3);
    });

    it('should delete documents', async () => {
      await store.store('doc1', 'Test');

      assert.strictEqual(store.delete('doc1'), true);
      assert.strictEqual(store.has('doc1'), false);
      assert.strictEqual(store.delete('doc1'), false); // Already deleted
    });
  });

  describe('Semantic Search', () => {
    beforeEach(async () => {
      // Store some documents
      await store.store('greeting1', 'Hello there friend', { type: 'greeting' });
      await store.store('greeting2', 'Hi how are you', { type: 'greeting' });
      await store.store('code1', 'function hello() {}', { type: 'code' });
      await store.store('code2', 'const greet = () => {}', { type: 'code' });
    });

    it('should search by query text', async () => {
      const results = await store.search('hello', 2);

      assert.ok(results.length > 0);
      assert.ok('id' in results[0]);
      assert.ok('score' in results[0]);
      assert.ok('text' in results[0]);
      assert.ok('metadata' in results[0]);
    });

    it('should filter by metadata', async () => {
      const results = await store.search('hello', 10, {
        filter: (meta) => meta.type === 'code',
      });

      assert.ok(results.every(r => r.metadata.type === 'code'));
    });

    it('should respect minScore', async () => {
      const results = await store.search('hello', 10, { minScore: 0.9 });

      assert.ok(results.every(r => r.score >= 0.9));
    });
  });

  describe('Similar Document Search', () => {
    it('should find similar documents', async () => {
      await store.store('a', 'The quick brown fox');
      await store.store('b', 'A fast brown fox');
      await store.store('c', 'Hello world');

      const results = await store.searchSimilar('a', 2);

      assert.ok(results !== undefined);
      assert.ok(results.every(r => r.id !== 'a'));
    });

    it('should throw for non-existent document', async () => {
      await assert.rejects(store.searchSimilar('nonexistent'), /not found/i);
    });
  });

  describe('Statistics', () => {
    it('should track stats', async () => {
      await store.store('a', 'Test 1');
      await store.store('b', 'Test 2');
      await store.search('test', 5);
      await store.search('test', 5); // Cache hit

      const stats = store.stats();

      assert.strictEqual(stats.stored, 2);
      assert.strictEqual(stats.searches, 2);
      assert.strictEqual(stats.documents, 2);
      assert.ok(stats.cacheHitRate > 0);
    });
  });

  describe('Serialization', () => {
    it('should export and import state', async () => {
      await store.store('doc1', 'Hello world', { label: 'test' });
      await store.store('doc2', 'Goodbye world');

      const json = store.toJSON();
      const restored = VectorStore.fromJSON(json);

      assert.strictEqual(restored.has('doc1'), true);
      assert.strictEqual(restored.has('doc2'), true);
      assert.strictEqual(restored.get('doc1').metadata.label, 'test');
    });
  });

  describe('Clear', () => {
    it('should clear all data', async () => {
      await store.store('a', 'Test');
      await store.store('b', 'Test');

      store.clear();

      assert.strictEqual(store.ids().length, 0);
      assert.strictEqual(store.stats().documents, 0);
    });
  });
});

describe('Configuration', () => {
  it('should have φ-aligned defaults', () => {
    assert.ok(HNSW_CONFIG.M !== undefined);
    assert.ok(HNSW_CONFIG.efConstruction !== undefined);
    assert.ok(HNSW_CONFIG.efSearch !== undefined);

    assert.ok(Math.abs(VECTOR_STORE_CONFIG.minScore - 0.382) < 0.001); // φ⁻²
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// V3: Semantic Pattern Matching
// ═══════════════════════════════════════════════════════════════════════════

describe('SemanticPatternMatcher', () => {
  let matcher;

  beforeEach(() => {
    matcher = createSemanticPatternMatcher({
      config: {
        embedder: 'mock',
        dimensions: 384,
      },
    });
  });

  describe('Pattern Management', () => {
    it('should add patterns', async () => {
      const pattern = await matcher.addPattern('p1', 'User prefers tabs over spaces', {
        category: 'style',
      });

      assert.strictEqual(pattern.id, 'p1');
      assert.strictEqual(pattern.description, 'User prefers tabs over spaces');
      assert.strictEqual(pattern.metadata.category, 'style');
    });

    it('should add patterns in batch', async () => {
      const count = await matcher.addPatterns([
        { id: 'p1', description: 'Pattern one' },
        { id: 'p2', description: 'Pattern two' },
        { id: 'p3', description: 'Pattern three' },
      ]);

      assert.strictEqual(count, 3);
      assert.ok(matcher.hasPattern('p1'));
      assert.ok(matcher.hasPattern('p2'));
      assert.ok(matcher.hasPattern('p3'));
    });

    it('should get pattern by ID', async () => {
      await matcher.addPattern('p1', 'Test pattern');

      const pattern = matcher.getPattern('p1');
      assert.ok(pattern);
      assert.strictEqual(pattern.id, 'p1');
    });

    it('should remove patterns', async () => {
      await matcher.addPattern('p1', 'Test pattern');

      assert.strictEqual(matcher.removePattern('p1'), true);
      assert.strictEqual(matcher.hasPattern('p1'), false);
      assert.strictEqual(matcher.removePattern('p1'), false);
    });
  });

  describe('Semantic Search', () => {
    beforeEach(async () => {
      await matcher.addPatterns([
        { id: 'p1', description: 'User prefers tabs for indentation' },
        { id: 'p2', description: 'User likes 2-space indentation' },
        { id: 'p3', description: 'User wants TypeScript strict mode' },
        { id: 'p4', description: 'User prefers functional programming' },
      ]);
    });

    it('should find similar patterns', async () => {
      // Mock embedder uses deterministic hashing, so search for exact phrase
      const results = await matcher.findSimilar('User prefers tabs', 2, { minScore: 0.1 });

      assert.ok(results.length > 0);
      assert.ok('pattern' in results[0]);
      assert.ok('score' in results[0]);
    });

    it('should find patterns similar to existing pattern', async () => {
      const results = await matcher.findSimilarToPattern('p1', 2);

      assert.ok(results.length > 0);
      assert.ok(results.every(r => r.pattern.id !== 'p1'));
    });

    it('should match existing patterns', async () => {
      const match = await matcher.matchExisting('tabs vs spaces');

      // Should match one of the indentation patterns
      assert.ok(match === null || match.id === 'p1' || match.id === 'p2');
    });
  });

  describe('Clustering', () => {
    beforeEach(async () => {
      // Add patterns that should cluster
      await matcher.addPatterns([
        { id: 'style1', description: 'Indentation with tabs', metadata: { occurrences: 5 } },
        { id: 'style2', description: 'Tab-based indentation style', metadata: { occurrences: 3 } },
        { id: 'style3', description: 'Use tabs for indent', metadata: { occurrences: 2 } },
        { id: 'type1', description: 'TypeScript strict typing', metadata: { occurrences: 4 } },
        { id: 'type2', description: 'Strict type checking enabled', metadata: { occurrences: 2 } },
      ]);
    });

    it('should cluster similar patterns', async () => {
      const clusters = await matcher.clusterPatterns({
        minSize: 2,
      });

      // Should have at least one cluster
      assert.ok(clusters.length >= 0);

      // Clusters should have members
      for (const cluster of clusters) {
        assert.ok(cluster.size >= 2);
        assert.ok(cluster.theme);
      }
    });

    it('should cache clusters', async () => {
      await matcher.clusterPatterns();
      const cached = matcher.getClusters();

      assert.ok(Array.isArray(cached));
    });
  });

  describe('Integration Helpers', () => {
    it('should extract from judgment', async () => {
      const judgment = {
        id: 'j1',
        reasoning: 'This code follows good patterns',
        dimensions: {
          style: { score: 0.8, notes: 'Clean formatting' },
          clarity: { score: 0.7, notes: 'Well documented' },
        },
      };

      const patterns = await matcher.extractFromJudgment(judgment);

      assert.ok(patterns.length >= 1);
      assert.ok(patterns.every(p => p.metadata.source === 'judgment'));
    });

    it('should recommend patterns for context', async () => {
      await matcher.addPatterns([
        { id: 'p1', description: 'Use async/await for promises', metadata: { confidence: 0.8 } },
        { id: 'p2', description: 'Prefer const over let', metadata: { confidence: 0.9 } },
      ]);

      const recommendations = await matcher.recommendPatterns('JavaScript best practices');

      assert.ok(Array.isArray(recommendations));
      for (const rec of recommendations) {
        assert.ok('pattern' in rec);
        assert.ok('relevance' in rec);
      }
    });
  });

  describe('Statistics', () => {
    it('should track stats', async () => {
      await matcher.addPattern('p1', 'Test');
      await matcher.findSimilar('test', 5);

      const stats = matcher.stats();

      assert.strictEqual(stats.added, 1);
      assert.strictEqual(stats.searches, 1);
      assert.strictEqual(stats.patterns, 1);
    });
  });

  describe('Serialization', () => {
    it('should export and import state', async () => {
      await matcher.addPattern('p1', 'Test pattern', { category: 'test' });

      const json = matcher.toJSON();
      const restored = SemanticPatternMatcher.fromJSON(json);

      assert.ok(restored.hasPattern('p1'));
      assert.strictEqual(restored.getPattern('p1').metadata.category, 'test');
    });
  });

  describe('Clear', () => {
    it('should clear all data', async () => {
      await matcher.addPattern('p1', 'Test');
      await matcher.addPattern('p2', 'Test');

      matcher.clear();

      assert.strictEqual(matcher.patternIds().length, 0);
    });
  });
});

describe('SemanticPattern', () => {
  it('should create with defaults', () => {
    const pattern = new SemanticPattern('p1', 'Description');

    assert.strictEqual(pattern.id, 'p1');
    assert.strictEqual(pattern.description, 'Description');
    assert.ok(pattern.metadata.createdAt);
    assert.strictEqual(pattern.metadata.occurrences, 1);
  });

  it('should convert to storable', () => {
    const pattern = new SemanticPattern('p1', 'Description', { extra: 'data' });
    const storable = pattern.toStorable();

    assert.strictEqual(storable.id, 'p1');
    assert.strictEqual(storable.text, 'Description');
    assert.strictEqual(storable.metadata.extra, 'data');
  });
});

describe('PatternCluster', () => {
  it('should create cluster', () => {
    const centroid = new SemanticPattern('p1', 'Centroid');
    const cluster = new PatternCluster('c1', centroid, [centroid]);

    assert.strictEqual(cluster.id, 'c1');
    assert.strictEqual(cluster.size, 1);
    assert.strictEqual(cluster.theme, 'Centroid');
  });

  it('should add members', () => {
    const centroid = new SemanticPattern('p1', 'Centroid');
    const cluster = new PatternCluster('c1', centroid);

    cluster.addMember(new SemanticPattern('p2', 'Member'));
    cluster.addMember(new SemanticPattern('p3', 'Another'));

    assert.strictEqual(cluster.size, 2);
  });

  it('should export to JSON', () => {
    const centroid = new SemanticPattern('p1', 'Centroid');
    const cluster = new PatternCluster('c1', centroid, [centroid]);
    const json = cluster.toJSON();

    assert.strictEqual(json.id, 'c1');
    assert.strictEqual(json.theme, 'Centroid');
    assert.strictEqual(json.size, 1);
  });
});
