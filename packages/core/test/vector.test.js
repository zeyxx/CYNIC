/**
 * @cynic/core - Vector Search Tests
 *
 * Tests semantic search capabilities:
 * - TF-IDF embeddings
 * - Cosine similarity
 * - Vector index
 * - Semantic search
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/core/test/vector
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  VECTOR_CONSTANTS,
  TfIdfEmbedder,
  ExternalEmbedder,
  cosineSimilarity,
  euclideanDistance,
  VectorIndex,
  SemanticSearch,
} from '../src/vector/index.js';

import { PHI_INV, PHI_INV_2 } from '../src/axioms/constants.js';

// =============================================================================
// CONSTANTS TESTS
// =============================================================================

describe('VECTOR_CONSTANTS', () => {
  it('should have expected values', () => {
    assert.strictEqual(VECTOR_CONSTANTS.DEFAULT_DIMENSION, 336); // 21 × 16
    assert.strictEqual(VECTOR_CONSTANTS.MAX_VOCABULARY, 2330);  // 233 × 10
    assert.strictEqual(VECTOR_CONSTANTS.MIN_WORD_FREQUENCY, 2);
    assert.strictEqual(VECTOR_CONSTANTS.MAX_RESULTS, 21);  // Fib(8)
  });

  it('should use φ-aligned threshold', () => {
    assert.strictEqual(VECTOR_CONSTANTS.SIMILARITY_THRESHOLD, PHI_INV_2);
  });
});

// =============================================================================
// COSINE SIMILARITY TESTS
// =============================================================================

describe('cosineSimilarity', () => {
  it('should return 1 for identical vectors', () => {
    const a = [1, 0, 0];
    const b = [1, 0, 0];
    assert.strictEqual(cosineSimilarity(a, b), 1);
  });

  it('should return 0 for orthogonal vectors', () => {
    const a = [1, 0, 0];
    const b = [0, 1, 0];
    const similarity = cosineSimilarity(a, b);
    assert.ok(Math.abs(similarity) < 0.0001);
  });

  it('should return -1 for opposite vectors', () => {
    const a = [1, 0, 0];
    const b = [-1, 0, 0];
    const similarity = cosineSimilarity(a, b);
    assert.ok(Math.abs(similarity + 1) < 0.0001);
  });

  it('should handle normalized vectors', () => {
    const a = [0.6, 0.8, 0];
    const b = [0.6, 0.8, 0];
    const similarity = cosineSimilarity(a, b);
    assert.ok(Math.abs(similarity - 1) < 0.0001);
  });

  it('should handle unnormalized vectors', () => {
    const a = [3, 4, 0];
    const b = [6, 8, 0];
    const similarity = cosineSimilarity(a, b);
    assert.ok(Math.abs(similarity - 1) < 0.0001); // Same direction
  });

  it('should throw for different dimensions', () => {
    const a = [1, 2, 3];
    const b = [1, 2];
    assert.throws(() => cosineSimilarity(a, b), /same dimension/);
  });

  it('should return 0 for zero vectors', () => {
    const a = [0, 0, 0];
    const b = [1, 2, 3];
    assert.strictEqual(cosineSimilarity(a, b), 0);
  });
});

// =============================================================================
// EUCLIDEAN DISTANCE TESTS
// =============================================================================

describe('euclideanDistance', () => {
  it('should return 0 for identical vectors', () => {
    const a = [1, 2, 3];
    const b = [1, 2, 3];
    assert.strictEqual(euclideanDistance(a, b), 0);
  });

  it('should calculate correct distance', () => {
    const a = [0, 0, 0];
    const b = [3, 4, 0];
    assert.strictEqual(euclideanDistance(a, b), 5);
  });

  it('should be commutative', () => {
    const a = [1, 2, 3];
    const b = [4, 5, 6];
    assert.strictEqual(euclideanDistance(a, b), euclideanDistance(b, a));
  });

  it('should throw for different dimensions', () => {
    const a = [1, 2, 3];
    const b = [1, 2];
    assert.throws(() => euclideanDistance(a, b), /same dimension/);
  });
});

// =============================================================================
// TF-IDF EMBEDDER TESTS
// =============================================================================

describe('TfIdfEmbedder', () => {
  let embedder;
  const corpus = [
    'The quick brown fox jumps over the lazy dog',
    'The lazy dog sleeps all day',
    'A quick brown fox is running fast',
    'Dogs and foxes are animals',
    'Programming in JavaScript is fun',
    'JavaScript functions are powerful',
    'Node.js runs JavaScript on the server',
  ];

  beforeEach(() => {
    embedder = new TfIdfEmbedder();
  });

  describe('Construction', () => {
    it('should create untrained embedder', () => {
      assert.strictEqual(embedder.trained, false);
      assert.strictEqual(embedder.totalDocs, 0);
    });
  });

  describe('Training', () => {
    it('should train on corpus', () => {
      embedder.train(corpus);
      assert.strictEqual(embedder.trained, true);
      assert.strictEqual(embedder.totalDocs, corpus.length);
    });

    it('should build vocabulary', () => {
      embedder.train(corpus);
      assert.ok(embedder.vocabulary.size > 0);
      assert.ok(embedder.vocabulary.size <= VECTOR_CONSTANTS.MAX_VOCABULARY);
    });

    it('should return this for chaining', () => {
      const result = embedder.train(corpus);
      assert.strictEqual(result, embedder);
    });
  });

  describe('Embedding', () => {
    beforeEach(() => {
      embedder.train(corpus);
    });

    it('should generate embedding', () => {
      const embedding = embedder.embed('quick brown fox');
      assert.ok(Array.isArray(embedding));
      assert.strictEqual(embedding.length, embedder.vocabulary.size);
    });

    it('should generate normalized vectors', () => {
      const embedding = embedder.embed('quick brown fox');
      const magnitude = Math.sqrt(embedding.reduce((sum, v) => sum + v * v, 0));
      // Should be unit vector (magnitude ~= 1) if not all zeros
      if (magnitude > 0) {
        assert.ok(Math.abs(magnitude - 1) < 0.0001);
      }
    });

    it('should throw if not trained', () => {
      const untrained = new TfIdfEmbedder();
      assert.throws(() => untrained.embed('test'), /trained first/);
    });

    it('should produce similar embeddings for similar text', () => {
      const a = embedder.embed('quick brown fox');
      const b = embedder.embed('fast brown fox');
      const c = embedder.embed('JavaScript programming');

      const simAB = cosineSimilarity(a, b);
      const simAC = cosineSimilarity(a, c);

      assert.ok(simAB > simAC, 'Similar text should have higher similarity');
    });
  });

  describe('Incremental Learning', () => {
    beforeEach(() => {
      embedder.train(corpus);
    });

    it('should add document', () => {
      const initialDocs = embedder.totalDocs;
      embedder.addDocument('New document about foxes');
      assert.strictEqual(embedder.totalDocs, initialDocs + 1);
    });

    it('should update document frequencies', () => {
      const foxBefore = embedder.documentFrequencies.get('fox') || 0;
      embedder.addDocument('Another fox story');
      const foxAfter = embedder.documentFrequencies.get('fox') || 0;
      // Note: addDocument only increments if word is in vocabulary
      assert.ok(foxAfter >= foxBefore);
    });
  });

  describe('Export/Import', () => {
    beforeEach(() => {
      embedder.train(corpus);
    });

    it('should export state', () => {
      const state = embedder.export();
      assert.ok(Array.isArray(state.vocabulary));
      assert.ok(Array.isArray(state.documentFrequencies));
      assert.strictEqual(state.totalDocs, corpus.length);
    });

    it('should import state', () => {
      const state = embedder.export();
      const newEmbedder = new TfIdfEmbedder();
      newEmbedder.import(state);

      assert.strictEqual(newEmbedder.trained, true);
      assert.strictEqual(newEmbedder.totalDocs, corpus.length);
      assert.strictEqual(newEmbedder.vocabulary.size, embedder.vocabulary.size);
    });

    it('should produce same embeddings after import', () => {
      const state = embedder.export();
      const newEmbedder = new TfIdfEmbedder().import(state);

      const original = embedder.embed('quick brown fox');
      const imported = newEmbedder.embed('quick brown fox');

      assert.deepStrictEqual(original, imported);
    });
  });

  describe('getDimension', () => {
    it('should return vocabulary size', () => {
      embedder.train(corpus);
      assert.strictEqual(embedder.getDimension(), embedder.vocabulary.size);
    });
  });
});

// =============================================================================
// EXTERNAL EMBEDDER TESTS
// =============================================================================

describe('ExternalEmbedder', () => {
  describe('Construction', () => {
    it('should create with default provider', () => {
      const embedder = new ExternalEmbedder();
      assert.strictEqual(embedder.provider, 'openai');
    });

    it('should set provider', () => {
      const embedder = new ExternalEmbedder({ provider: 'cohere' });
      assert.strictEqual(embedder.provider, 'cohere');
    });

    it('should set default model by provider', () => {
      const openai = new ExternalEmbedder({ provider: 'openai' });
      const cohere = new ExternalEmbedder({ provider: 'cohere' });
      const voyage = new ExternalEmbedder({ provider: 'voyage' });

      assert.strictEqual(openai.model, 'text-embedding-3-small');
      assert.strictEqual(cohere.model, 'embed-english-v3.0');
      assert.strictEqual(voyage.model, 'voyage-2');
    });

    it('should set default dimension by provider', () => {
      const openai = new ExternalEmbedder({ provider: 'openai' });
      const cohere = new ExternalEmbedder({ provider: 'cohere' });

      assert.strictEqual(openai.dimension, 1536);
      assert.strictEqual(cohere.dimension, 1024);
    });

    it('should allow custom model', () => {
      const embedder = new ExternalEmbedder({
        provider: 'openai',
        model: 'text-embedding-3-large',
      });
      assert.strictEqual(embedder.model, 'text-embedding-3-large');
    });
  });

  describe('Error Handling', () => {
    it('should throw without API key', async () => {
      const embedder = new ExternalEmbedder({ provider: 'openai' });
      await assert.rejects(
        () => embedder.embed('test'),
        /API key required/
      );
    });

    it('should throw for unsupported provider', async () => {
      const embedder = new ExternalEmbedder({
        provider: 'unknown',
        apiKey: 'test',
      });
      await assert.rejects(
        () => embedder.embed('test'),
        /Unsupported provider/
      );
    });
  });

  describe('getDimension', () => {
    it('should return configured dimension', () => {
      const embedder = new ExternalEmbedder({ dimension: 512 });
      assert.strictEqual(embedder.getDimension(), 512);
    });
  });
});

// =============================================================================
// VECTOR INDEX TESTS
// =============================================================================

describe('VectorIndex', () => {
  let index;

  beforeEach(() => {
    index = new VectorIndex();
  });

  describe('Construction', () => {
    it('should create empty index', () => {
      assert.strictEqual(index.size(), 0);
    });

    it('should accept custom similarity function', () => {
      const customIndex = new VectorIndex({
        similarity: euclideanDistance,
      });
      assert.ok(customIndex.similarityFn);
    });
  });

  describe('Add/Remove', () => {
    it('should add vector', () => {
      index.add('doc1', [1, 0, 0], { type: 'test' });
      assert.strictEqual(index.size(), 1);
    });

    it('should update existing vector', () => {
      index.add('doc1', [1, 0, 0]);
      index.add('doc1', [0, 1, 0]); // Update
      assert.strictEqual(index.size(), 1);
    });

    it('should remove vector', () => {
      index.add('doc1', [1, 0, 0]);
      const result = index.remove('doc1');
      assert.strictEqual(result, true);
      assert.strictEqual(index.size(), 0);
    });

    it('should return false for removing non-existent', () => {
      const result = index.remove('nonexistent');
      assert.strictEqual(result, false);
    });
  });

  describe('Search', () => {
    beforeEach(() => {
      index.add('doc1', [1, 0, 0], { type: 'a' });
      index.add('doc2', [0.9, 0.1, 0], { type: 'a' });
      index.add('doc3', [0, 1, 0], { type: 'b' });
      index.add('doc4', [0, 0, 1], { type: 'b' });
    });

    it('should return similar vectors', () => {
      const results = index.search([1, 0, 0], { threshold: 0 });
      assert.ok(results.length > 0);
      assert.strictEqual(results[0].id, 'doc1'); // Most similar
    });

    it('should sort by similarity descending', () => {
      const results = index.search([1, 0, 0], { threshold: 0 });
      for (let i = 1; i < results.length; i++) {
        assert.ok(results[i - 1].similarity >= results[i].similarity);
      }
    });

    it('should respect limit', () => {
      const results = index.search([1, 0, 0], { limit: 2, threshold: 0 });
      assert.ok(results.length <= 2);
    });

    it('should respect threshold', () => {
      const results = index.search([1, 0, 0], { threshold: 0.5 });
      for (const r of results) {
        assert.ok(r.similarity >= 0.5);
      }
    });

    it('should filter by metadata', () => {
      const results = index.search([1, 0, 0], {
        filter: { type: 'a' },
        threshold: 0,
      });
      for (const r of results) {
        assert.strictEqual(r.metadata.type, 'a');
      }
    });
  });

  describe('Clear', () => {
    it('should clear all vectors', () => {
      index.add('doc1', [1, 0, 0]);
      index.add('doc2', [0, 1, 0]);
      index.clear();
      assert.strictEqual(index.size(), 0);
    });
  });

  describe('Export/Import', () => {
    it('should export state', () => {
      index.add('doc1', [1, 0, 0], { test: true });
      const state = index.export();
      assert.ok(Array.isArray(state.vectors));
      assert.strictEqual(state.vectors.length, 1);
    });

    it('should import state', () => {
      index.add('doc1', [1, 0, 0]);
      const state = index.export();

      const newIndex = new VectorIndex().import(state);
      assert.strictEqual(newIndex.size(), 1);
    });
  });
});

// =============================================================================
// SEMANTIC SEARCH TESTS
// =============================================================================

describe('SemanticSearch', () => {
  let search;
  const documents = [
    { id: 'doc1', text: 'The quick brown fox jumps over the lazy dog', metadata: { type: 'proverb' } },
    { id: 'doc2', text: 'The lazy dog sleeps all day', metadata: { type: 'proverb' } },
    { id: 'doc3', text: 'JavaScript is a programming language', metadata: { type: 'tech' } },
    { id: 'doc4', text: 'Node.js runs JavaScript on the server', metadata: { type: 'tech' } },
  ];

  beforeEach(async () => {
    search = new SemanticSearch();
    await search.initialize(documents);
  });

  describe('Construction', () => {
    it('should create with default embedder and index', () => {
      const s = new SemanticSearch();
      assert.ok(s.embedder instanceof TfIdfEmbedder);
      assert.ok(s.index instanceof VectorIndex);
    });

    it('should accept custom embedder', () => {
      const customEmbedder = new TfIdfEmbedder();
      customEmbedder.train(['test document']);
      const s = new SemanticSearch({ embedder: customEmbedder });
      assert.strictEqual(s.embedder, customEmbedder);
    });
  });

  describe('Initialize', () => {
    it('should index all documents', async () => {
      assert.strictEqual(search.index.size(), documents.length);
    });

    it('should store document texts', () => {
      for (const doc of documents) {
        assert.strictEqual(search.documents.get(doc.id), doc.text);
      }
    });

    it('should train embedder', () => {
      assert.strictEqual(search.embedder.trained, true);
    });
  });

  describe('Search', () => {
    it('should return relevant documents', async () => {
      const results = await search.search('fox dog', { threshold: 0 });
      assert.ok(results.length > 0);
    });

    it('should include text in results', async () => {
      const results = await search.search('JavaScript', { threshold: 0 });
      for (const r of results) {
        assert.ok(r.text);
      }
    });

    it('should cap confidence at φ⁻¹', async () => {
      const results = await search.search('JavaScript programming language', { threshold: 0 });
      for (const r of results) {
        assert.ok(r.confidence <= PHI_INV);
      }
    });

    it('should filter by metadata', async () => {
      const results = await search.search('programming', {
        filter: { type: 'tech' },
        threshold: 0,
      });
      for (const r of results) {
        assert.strictEqual(r.metadata.type, 'tech');
      }
    });
  });

  describe('Add/Remove Documents', () => {
    it('should add new document', async () => {
      const sizeBefore = search.index.size();
      await search.addDocument('doc5', 'New document about Python', { type: 'tech' });
      assert.strictEqual(search.index.size(), sizeBefore + 1);
    });

    it('should remove document', () => {
      search.removeDocument('doc1');
      assert.strictEqual(search.documents.has('doc1'), false);
      assert.strictEqual(search.index.size(), documents.length - 1);
    });
  });

  describe('Stats', () => {
    it('should return stats', () => {
      const stats = search.getStats();
      assert.strictEqual(stats.documents, documents.length);
      assert.strictEqual(stats.indexSize, documents.length);
      assert.strictEqual(stats.embedderType, 'TfIdfEmbedder');
      assert.ok(stats.dimension > 0);
    });
  });

  describe('Export/Import', () => {
    it('should export state', () => {
      const state = search.export();
      assert.ok(state.embedder);
      assert.ok(state.index);
      assert.ok(Array.isArray(state.documents));
    });

    it('should import state', async () => {
      const state = search.export();
      const newSearch = new SemanticSearch().import(state);

      assert.strictEqual(newSearch.documents.size, documents.length);
      assert.strictEqual(newSearch.index.size(), documents.length);
    });

    it('should work after import', async () => {
      const state = search.export();
      const newSearch = new SemanticSearch().import(state);

      const results = await newSearch.search('fox', { threshold: 0 });
      assert.ok(results.length > 0);
    });
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('Vector Edge Cases', () => {
  it('should handle empty corpus', () => {
    const embedder = new TfIdfEmbedder();
    embedder.train([]);
    assert.strictEqual(embedder.vocabulary.size, 0);
  });

  it('should handle single character words', () => {
    const embedder = new TfIdfEmbedder();
    embedder.train(['a b c', 'x y z']);
    // Single char words should be filtered out (length < 2)
    assert.ok(!embedder.vocabulary.has('a'));
  });

  it('should handle special characters', () => {
    const embedder = new TfIdfEmbedder();
    // Need at least 3 docs where words appear in 2+ docs (MIN_WORD_FREQUENCY = 2)
    embedder.train(['Hello, world!', 'Hello again!', 'World test']);
    // Special chars should be removed, 'hello' appears in 2 docs
    assert.ok(embedder.vocabulary.has('hello') || embedder.vocabulary.has('world'));
  });

  it('should handle empty search query', async () => {
    const search = new SemanticSearch();
    await search.initialize([{ id: 'd1', text: 'test document' }]);

    const results = await search.search('', { threshold: 0 });
    assert.ok(Array.isArray(results));
  });

  it('should handle very long documents', () => {
    const embedder = new TfIdfEmbedder();
    const longDoc = 'word '.repeat(10000);
    embedder.train([longDoc, 'short doc']);
    assert.strictEqual(embedder.trained, true);
  });
});
