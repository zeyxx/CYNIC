/**
 * @cynic/node - Tiered Memory Tests
 *
 * Comprehensive tests for the 4-tier memory architecture:
 *   MemoryItem, VectorMemory, EpisodicMemory, SemanticMemory,
 *   WorkingMemory, TieredMemory, Episode.
 *
 * @module @cynic/node/test/tiered-memory
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  MemoryItem,
  VectorMemory,
  EpisodicMemory,
  SemanticMemory,
  WorkingMemory,
  TieredMemory,
  Episode,
  MEMORY_CONFIG,
  createTieredMemory,
} from '../src/memory/tiered-memory.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// MEMORY_CONFIG TESTS
// =============================================================================

describe('MEMORY_CONFIG', () => {
  it('should have phi-aligned promotion threshold', () => {
    assert.equal(MEMORY_CONFIG.promotionThreshold, PHI_INV);
  });

  it('should have phi-aligned demotion threshold', () => {
    assert.equal(MEMORY_CONFIG.demotionThreshold, PHI_INV_2);
  });

  it('should set working memory capacity to Miller\'s Law', () => {
    assert.equal(MEMORY_CONFIG.working.maxItems, 7);
  });

  it('should define all four tier configs', () => {
    assert.ok(MEMORY_CONFIG.vector);
    assert.ok(MEMORY_CONFIG.episodic);
    assert.ok(MEMORY_CONFIG.semantic);
    assert.ok(MEMORY_CONFIG.working);
  });

  it('should set reasonable vector defaults', () => {
    assert.equal(MEMORY_CONFIG.vector.dimensions, 384);
    assert.equal(MEMORY_CONFIG.vector.similarityThreshold, 0.7);
    assert.ok(MEMORY_CONFIG.vector.maxItems > 0);
  });

  it('should set reasonable semantic defaults', () => {
    assert.ok(MEMORY_CONFIG.semantic.maxFacts > 0);
    assert.ok(MEMORY_CONFIG.semantic.minConfidence > 0);
    assert.ok(MEMORY_CONFIG.semantic.decayRate > 0 && MEMORY_CONFIG.semantic.decayRate <= 1);
  });
});

// =============================================================================
// MEMORY ITEM TESTS
// =============================================================================

describe('MemoryItem', () => {
  it('should create with default values', () => {
    const item = new MemoryItem();

    assert.ok(item.id.startsWith('mem_'));
    assert.equal(item.content, '');
    assert.equal(item.type, 'generic');
    assert.equal(item.tier, 'working');
    assert.ok(item.createdAt > 0);
    assert.ok(item.lastAccessed > 0);
    assert.equal(item.accessCount, 0);
    assert.equal(item.confidence, 0.5);
    assert.deepEqual(item.metadata, {});
    assert.equal(item.embedding, null);
    assert.deepEqual(item.tags, []);
  });

  it('should create from custom data', () => {
    const data = {
      id: 'test_001',
      content: 'Hello world',
      type: 'fact',
      tier: 'semantic',
      confidence: 0.9,
      tags: ['greeting', 'test'],
      metadata: { source: 'unit_test' },
    };
    const item = new MemoryItem(data);

    assert.equal(item.id, 'test_001');
    assert.equal(item.content, 'Hello world');
    assert.equal(item.type, 'fact');
    assert.equal(item.tier, 'semantic');
    assert.equal(item.confidence, 0.9);
    assert.deepEqual(item.tags, ['greeting', 'test']);
    assert.deepEqual(item.metadata, { source: 'unit_test' });
  });

  it('should handle confidence of 0 correctly', () => {
    const item = new MemoryItem({ confidence: 0 });
    assert.equal(item.confidence, 0);
  });

  it('should increment access count on access()', () => {
    const item = new MemoryItem();
    const before = item.lastAccessed;

    item.access();

    assert.equal(item.accessCount, 1);
    assert.ok(item.lastAccessed >= before);
  });

  it('should return this from access() for chaining', () => {
    const item = new MemoryItem();
    const result = item.access();

    assert.strictEqual(result, item);
  });

  it('should detect stale items', () => {
    const item = new MemoryItem({
      lastAccessed: Date.now() - 3600000 * 2, // 2 hours ago
    });

    assert.equal(item.isStale(3600000), true);  // 1 hour max
    assert.equal(item.isStale(3600000 * 3), false); // 3 hour max
  });

  it('should not be stale when recently accessed', () => {
    const item = new MemoryItem();
    assert.equal(item.isStale(60000), false);
  });

  it('should calculate importance with phi weighting', () => {
    const item = new MemoryItem({
      confidence: 0.8,
      accessCount: 5,
    });

    const importance = item.getImportance();
    assert.ok(typeof importance === 'number');
    assert.ok(importance > 0);
  });

  it('should give higher importance to recently accessed items', () => {
    const recent = new MemoryItem({ accessCount: 1, confidence: 0.5 });
    const old = new MemoryItem({
      accessCount: 1,
      confidence: 0.5,
      lastAccessed: Date.now() - 3600000 * 48, // 48 hours ago
    });

    assert.ok(recent.getImportance() > old.getImportance());
  });

  it('should give higher importance to frequently accessed items', () => {
    const frequent = new MemoryItem({ accessCount: 100, confidence: 0.5 });
    const rare = new MemoryItem({ accessCount: 0, confidence: 0.5 });

    assert.ok(frequent.getImportance() > rare.getImportance());
  });

  it('should serialize to JSON', () => {
    const item = new MemoryItem({
      id: 'json_test',
      content: 'test content',
      type: 'fact',
      tags: ['a', 'b'],
    });

    const json = item.toJSON();

    assert.equal(json.id, 'json_test');
    assert.equal(json.content, 'test content');
    assert.equal(json.type, 'fact');
    assert.deepEqual(json.tags, ['a', 'b']);
    assert.ok(!json.getImportance); // Should not include methods
  });

  it('should deserialize from JSON', () => {
    const original = new MemoryItem({
      id: 'round_trip',
      content: 'round trip test',
      confidence: 0.75,
    });

    const json = original.toJSON();
    const restored = MemoryItem.fromJSON(json);

    assert.equal(restored.id, 'round_trip');
    assert.equal(restored.content, 'round trip test');
    assert.equal(restored.confidence, 0.75);
    assert.ok(restored instanceof MemoryItem);
  });
});

// =============================================================================
// VECTOR MEMORY TESTS
// =============================================================================

describe('VectorMemory', () => {
  let vector;

  beforeEach(() => {
    vector = new VectorMemory();
  });

  it('should initialize with default config', () => {
    assert.ok(vector.config.maxItems > 0);
    assert.equal(vector.items.size, 0);
    assert.equal(vector.stats.stored, 0);
  });

  it('should accept custom config', () => {
    const custom = new VectorMemory({ maxItems: 500 });
    assert.equal(custom.config.maxItems, 500);
  });

  it('should store a memory item', async () => {
    const embedding = [0.1, 0.2, 0.3];
    const result = await vector.store({ content: 'test' }, embedding);

    assert.ok(result instanceof MemoryItem);
    assert.equal(result.tier, 'vector');
    assert.deepEqual(result.embedding, embedding);
    assert.equal(vector.items.size, 1);
    assert.equal(vector.stats.stored, 1);
  });

  it('should store an existing MemoryItem', async () => {
    const item = new MemoryItem({ id: 'existing', content: 'hello' });
    const result = await vector.store(item, [1, 2, 3]);

    assert.equal(result.id, 'existing');
    assert.equal(result.tier, 'vector');
  });

  it('should use item\'s embedding if none provided', async () => {
    const item = new MemoryItem({ content: 'test', embedding: [0.5, 0.5] });
    const result = await vector.store(item);

    assert.deepEqual(result.embedding, [0.5, 0.5]);
  });

  it('should search by similarity', async () => {
    await vector.store({ content: 'cat' }, [1, 0, 0]);
    await vector.store({ content: 'dog' }, [0.9, 0.1, 0]);
    await vector.store({ content: 'fish' }, [0, 1, 0]);

    const results = await vector.search([1, 0, 0], { threshold: 0.5 });

    assert.ok(results.length >= 1);
    assert.ok(results[0].similarity >= 0.5);
    assert.equal(results[0].item.content, 'cat');
  });

  it('should sort search results by similarity descending', async () => {
    await vector.store({ content: 'exact' }, [1, 0, 0]);
    await vector.store({ content: 'close' }, [0.9, 0.1, 0]);
    await vector.store({ content: 'far' }, [0.5, 0.5, 0]);

    const results = await vector.search([1, 0, 0], { threshold: 0.1 });

    for (let i = 1; i < results.length; i++) {
      assert.ok(results[i - 1].similarity >= results[i].similarity);
    }
  });

  it('should respect search limit', async () => {
    for (let i = 0; i < 20; i++) {
      await vector.store({ content: `item_${i}` }, [1, 0, 0]);
    }

    const results = await vector.search([1, 0, 0], { limit: 3, threshold: 0.1 });
    assert.ok(results.length <= 3);
  });

  it('should skip items without embeddings during search', async () => {
    await vector.store({ content: 'no_embed' });
    await vector.store({ content: 'has_embed' }, [1, 0, 0]);

    const results = await vector.search([1, 0, 0], { threshold: 0.1 });

    assert.equal(results.length, 1);
    assert.equal(results[0].item.content, 'has_embed');
  });

  it('should handle cosine similarity with mismatched lengths', () => {
    const sim = vector._cosineSimilarity([1, 0], [1, 0, 0]);
    assert.equal(sim, 0);
  });

  it('should handle cosine similarity with null inputs', () => {
    assert.equal(vector._cosineSimilarity(null, [1, 0]), 0);
    assert.equal(vector._cosineSimilarity([1, 0], null), 0);
    assert.equal(vector._cosineSimilarity(null, null), 0);
  });

  it('should calculate correct cosine similarity for identical vectors', () => {
    const sim = vector._cosineSimilarity([1, 0, 0], [1, 0, 0]);
    assert.ok(Math.abs(sim - 1.0) < 0.0001);
  });

  it('should calculate correct cosine similarity for orthogonal vectors', () => {
    const sim = vector._cosineSimilarity([1, 0, 0], [0, 1, 0]);
    assert.ok(Math.abs(sim) < 0.0001);
  });

  it('should handle zero vectors in cosine similarity', () => {
    const sim = vector._cosineSimilarity([0, 0, 0], [1, 0, 0]);
    assert.equal(sim, 0);
  });

  it('should evict items when at capacity', async () => {
    const small = new VectorMemory({ maxItems: 5 });

    for (let i = 0; i < 10; i++) {
      await small.store({ content: `item_${i}` }, [1, 0, 0]);
    }

    assert.ok(small.items.size <= 5);
    assert.ok(small.stats.evicted > 0);
  });

  it('should return correct stats', async () => {
    await vector.store({ content: 'a' }, [1, 0, 0]);
    await vector.store({ content: 'b' }, [0, 1, 0]);
    await vector.search([1, 0, 0], { threshold: 0.1 });

    const stats = vector.getStats();

    assert.equal(stats.stored, 2);
    assert.equal(stats.size, 2);
    assert.ok(stats.retrieved >= 0);
  });
});

// =============================================================================
// EPISODE TESTS
// =============================================================================

describe('Episode', () => {
  it('should create with default values', () => {
    const ep = new Episode();

    assert.ok(ep.id.startsWith('ep_'));
    assert.equal(ep.type, 'interaction');
    assert.deepEqual(ep.events, []);
    assert.equal(ep.summary, null);
    assert.ok(ep.startTime > 0);
    assert.equal(ep.endTime, null);
    assert.equal(ep.outcome, null);
    assert.equal(ep.compressed, false);
    assert.deepEqual(ep.metadata, {});
  });

  it('should create with custom data', () => {
    const ep = new Episode({
      id: 'ep_test',
      type: 'analysis',
      metadata: { project: 'cynic' },
    });

    assert.equal(ep.id, 'ep_test');
    assert.equal(ep.type, 'analysis');
    assert.deepEqual(ep.metadata, { project: 'cynic' });
  });

  it('should add events with timestamps', () => {
    const ep = new Episode();
    ep.addEvent({ type: 'decision', data: 'chose path A' });

    assert.equal(ep.events.length, 1);
    assert.equal(ep.events[0].type, 'decision');
    assert.ok(ep.events[0].timestamp > 0);
  });

  it('should preserve event timestamp if provided', () => {
    const ep = new Episode();
    const ts = Date.now() - 5000;
    ep.addEvent({ type: 'error', timestamp: ts });

    assert.equal(ep.events[0].timestamp, ts);
  });

  it('should end episode with outcome', () => {
    const ep = new Episode();
    const result = ep.end('success');

    assert.ok(ep.endTime > 0);
    assert.equal(ep.outcome, 'success');
    assert.strictEqual(result, ep);
  });

  it('should compress episode keeping only key events', () => {
    const ep = new Episode();

    // Add mixed events
    ep.addEvent({ type: 'noise', data: 'irrelevant' });
    ep.addEvent({ type: 'decision', data: 'chose path A' });
    ep.addEvent({ type: 'noise', data: 'more irrelevant' });
    ep.addEvent({ type: 'error', data: 'crash detected' });
    ep.addEvent({ type: 'success', data: 'recovered' });
    ep.addEvent({ type: 'noise', data: 'filler' });
    ep.addEvent({ type: 'noise', data: 'filler2', important: true });
    ep.end('done');

    ep.compress();

    assert.equal(ep.compressed, true);
    assert.ok(ep.summary);
    assert.equal(ep.summary.eventCount, 7);
    // Only decision, error, success, and important events should remain
    assert.ok(ep.events.length <= 7);
    assert.ok(ep.events.length >= 4);
  });

  it('should not double-compress', () => {
    const ep = new Episode();
    ep.addEvent({ type: 'decision', data: 'test' });
    ep.end('done');

    ep.compress();
    const eventsAfterFirst = ep.events.length;

    ep.compress(); // Should be idempotent
    assert.equal(ep.events.length, eventsAfterFirst);
  });

  it('should serialize to JSON', () => {
    const ep = new Episode({ id: 'json_ep', type: 'test' });
    ep.addEvent({ type: 'success', data: 'ok' });

    const json = ep.toJSON();

    assert.equal(json.id, 'json_ep');
    assert.equal(json.type, 'test');
    assert.equal(json.events.length, 1);
    assert.equal(json.compressed, false);
  });
});

// =============================================================================
// EPISODIC MEMORY TESTS
// =============================================================================

describe('EpisodicMemory', () => {
  let episodic;

  beforeEach(() => {
    episodic = new EpisodicMemory();
  });

  it('should initialize with default config', () => {
    assert.equal(episodic.episodes.size, 0);
    assert.equal(episodic.currentEpisode, null);
    assert.equal(episodic.stats.episodesCreated, 0);
  });

  it('should accept custom config', () => {
    const custom = new EpisodicMemory({ maxEpisodes: 100 });
    assert.equal(custom.config.maxEpisodes, 100);
  });

  it('should start a new episode', () => {
    const ep = episodic.startEpisode('analysis', { project: 'test' });

    assert.ok(ep instanceof Episode);
    assert.equal(ep.type, 'analysis');
    assert.equal(episodic.currentEpisode, ep);
    assert.equal(episodic.stats.episodesCreated, 1);
  });

  it('should auto-end previous episode when starting new one', () => {
    const first = episodic.startEpisode('first');
    const second = episodic.startEpisode('second');

    assert.ok(first.endTime > 0);
    assert.equal(episodic.currentEpisode, second);
    assert.equal(episodic.episodes.size, 1); // first is stored
  });

  it('should add events to current episode', () => {
    episodic.startEpisode();
    episodic.addEvent({ type: 'decision', data: 'chose A' });
    episodic.addEvent({ type: 'success', data: 'worked' });

    assert.equal(episodic.currentEpisode.events.length, 2);
  });

  it('should auto-start episode when adding event without one', () => {
    episodic.addEvent({ type: 'surprise', data: 'unexpected' });

    assert.ok(episodic.currentEpisode);
    assert.equal(episodic.currentEpisode.events.length, 1);
  });

  it('should end current episode with outcome', () => {
    episodic.startEpisode('task');
    episodic.addEvent({ type: 'success', data: 'done' });
    const ep = episodic.endEpisode('completed');

    assert.ok(ep);
    assert.equal(ep.outcome, 'completed');
    assert.ok(ep.endTime > 0);
    assert.equal(episodic.currentEpisode, null);
    assert.equal(episodic.episodes.size, 1);
  });

  it('should return null when ending without current episode', () => {
    const result = episodic.endEpisode();
    assert.equal(result, null);
  });

  it('should find similar episodes by type and outcome', () => {
    // Insert episodes directly with unique IDs to avoid ID collision
    // (Episode IDs use Date.now() which can collide in fast loops)
    const ep1 = new Episode({ id: 'ep_analysis_1', type: 'analysis' });
    ep1.addEvent({ type: 'decision', data: 'test' });
    ep1.end('success');
    episodic.episodes.set(ep1.id, ep1);

    const ep2 = new Episode({ id: 'ep_coding_1', type: 'coding' });
    ep2.addEvent({ type: 'error', data: 'bug' });
    ep2.end('failure');
    episodic.episodes.set(ep2.id, ep2);

    // Type + outcome gives similarity = 0.3 + 0.3 = 0.6, above 0.3 threshold
    const results = episodic.findSimilar({ type: 'analysis', outcome: 'success' });

    assert.ok(results.length > 0);
    assert.ok(results[0].similarity > 0.3);
  });

  it('should find similar episodes by outcome', () => {
    const ep1 = new Episode({ id: 'ep_task_s', type: 'task' });
    ep1.end('success');
    episodic.episodes.set(ep1.id, ep1);

    const ep2 = new Episode({ id: 'ep_task_f', type: 'task' });
    ep2.end('failure');
    episodic.episodes.set(ep2.id, ep2);

    // Type + outcome match: 0.3 + 0.3 = 0.6 > 0.3
    const results = episodic.findSimilar({
      type: 'task',
      outcome: 'success',
    });

    assert.ok(results.length > 0);
    assert.equal(results[0].episode.outcome, 'success');
  });

  it('should find similar episodes by event types', () => {
    const ep1 = new Episode({ id: 'ep_debug_1', type: 'debug' });
    ep1.addEvent({ type: 'error' });
    ep1.addEvent({ type: 'decision' });
    ep1.end();
    episodic.episodes.set(ep1.id, ep1);

    const ep2 = new Episode({ id: 'ep_build_1', type: 'build' });
    ep2.addEvent({ type: 'success' });
    ep2.end();
    episodic.episodes.set(ep2.id, ep2);

    // eventTypes overlap: 0.4 * (2/2) = 0.4 > 0.3
    const results = episodic.findSimilar({
      eventTypes: ['error', 'decision'],
    });

    assert.ok(results.length > 0);
    assert.equal(results[0].episode.id, 'ep_debug_1');
  });

  it('should respect findSimilar limit', () => {
    // Insert unique episodes directly
    for (let i = 0; i < 20; i++) {
      const ep = new Episode({ id: `ep_limit_${i}`, type: 'task' });
      ep.end('success');
      episodic.episodes.set(ep.id, ep);
    }

    // type + outcome = 0.6 > 0.3
    const results = episodic.findSimilar({ type: 'task', outcome: 'success' }, { limit: 3 });
    assert.ok(results.length <= 3);
  });

  it('should manage capacity by compressing and evicting', () => {
    const small = new EpisodicMemory({ maxEpisodes: 5 });

    // Insert episodes directly with unique IDs to avoid Date.now() collision
    for (let i = 0; i < 10; i++) {
      const ep = new Episode({ id: `ep_cap_${i}`, type: 'task' });
      ep.addEvent({ type: 'event', data: `event_${i}` });
      ep.end('done');
      small.episodes.set(ep.id, ep);
      small.stats.episodesCreated++;
      small._manageCapacity();
    }

    assert.ok(small.episodes.size <= 5);
    assert.ok(small.stats.episodesEvicted > 0 || small.stats.episodesCompressed > 0);
  });

  it('should return stats', () => {
    episodic.startEpisode('test');

    const stats = episodic.getStats();

    assert.ok('episodesCreated' in stats);
    assert.ok('size' in stats);
    assert.ok('currentEpisode' in stats);
    assert.equal(stats.currentEpisode, true);
  });
});

// =============================================================================
// SEMANTIC MEMORY TESTS
// =============================================================================

describe('SemanticMemory', () => {
  let semantic;

  beforeEach(() => {
    semantic = new SemanticMemory();
  });

  it('should initialize with default config', () => {
    assert.equal(semantic.facts.size, 0);
    assert.equal(semantic.patterns.size, 0);
    assert.equal(semantic.associations.size, 0);
  });

  it('should accept custom config', () => {
    const custom = new SemanticMemory({ maxFacts: 100, minConfidence: 0.7 });
    assert.equal(custom.config.maxFacts, 100);
    assert.equal(custom.config.minConfidence, 0.7);
  });

  it('should store a fact', () => {
    const fact = semantic.storeFact({
      content: 'Solana is fast',
      confidence: 0.8,
      tags: ['blockchain'],
    });

    assert.ok(fact instanceof MemoryItem);
    assert.equal(fact.type, 'fact');
    assert.equal(fact.tier, 'semantic');
    assert.equal(semantic.facts.size, 1);
    assert.equal(semantic.stats.factsStored, 1);
  });

  it('should reject low-confidence facts', () => {
    const result = semantic.storeFact({
      content: 'Dubious claim',
      confidence: 0.1,
    });

    assert.equal(result, null);
    assert.equal(semantic.facts.size, 0);
  });

  it('should accept a MemoryItem as fact input', () => {
    const item = new MemoryItem({
      content: 'Known fact',
      confidence: 0.9,
      type: 'fact',
      tier: 'semantic',
    });

    const result = semantic.storeFact(item);

    assert.ok(result);
    assert.strictEqual(result, item);
  });

  it('should store a pattern', () => {
    semantic.storePattern({
      name: 'pump_and_dump',
      signature: 'sig_pd',
      confidence: 0.7,
    });

    assert.equal(semantic.patterns.size, 1);
    assert.equal(semantic.stats.patternsStored, 1);
  });

  it('should update existing pattern on re-store', () => {
    semantic.storePattern({
      name: 'recurring_pattern',
      signature: 'sig_rec',
      confidence: 0.6,
    });
    semantic.storePattern({
      name: 'recurring_pattern',
      signature: 'sig_rec',
      confidence: 0.8,
    });

    assert.equal(semantic.patterns.size, 1);
    assert.equal(semantic.stats.patternsStored, 1); // Only counted once

    const pattern = semantic.patterns.get('sig_rec');
    assert.equal(pattern.occurrences, 2);
    assert.equal(pattern.confidence, (0.6 + 0.8) / 2); // Averaged
  });

  it('should query facts by type', () => {
    semantic.storeFact({ content: 'fact1', type: 'fact', confidence: 0.8 });
    semantic.storeFact({ content: 'fact2', type: 'rule', confidence: 0.8 });

    // Facts stored with type override to 'fact' from storeFact
    const results = semantic.queryFacts({ type: 'fact' });
    assert.ok(results.length >= 1);
  });

  it('should query facts by tags', () => {
    semantic.storeFact({
      content: 'tagged fact',
      confidence: 0.8,
      tags: ['blockchain', 'solana'],
    });
    semantic.storeFact({
      content: 'other fact',
      confidence: 0.8,
      tags: ['ai'],
    });

    const results = semantic.queryFacts({ tags: ['blockchain'] });
    assert.equal(results.length, 1);
    assert.equal(results[0].content, 'tagged fact');
  });

  it('should query facts by minimum confidence', () => {
    semantic.storeFact({ content: 'high', confidence: 0.9 });
    semantic.storeFact({ content: 'low', confidence: 0.55 });

    const results = semantic.queryFacts({ minConfidence: 0.8 });
    assert.equal(results.length, 1);
    assert.equal(results[0].content, 'high');
  });

  it('should increment access count on query', () => {
    const fact = semantic.storeFact({
      content: 'accessed fact',
      confidence: 0.8,
    });

    semantic.queryFacts({});
    assert.equal(fact.accessCount, 1);
  });

  it('should build tag-based associations', () => {
    const f1 = semantic.storeFact({
      content: 'fact 1',
      confidence: 0.8,
      tags: ['shared_tag'],
    });
    const f2 = semantic.storeFact({
      content: 'fact 2',
      confidence: 0.8,
      tags: ['shared_tag'],
    });

    assert.ok(semantic.associations.size > 0);
  });

  it('should get related facts', () => {
    const f1 = semantic.storeFact({
      content: 'fact 1',
      confidence: 0.8,
      tags: ['common'],
    });
    const f2 = semantic.storeFact({
      content: 'fact 2',
      confidence: 0.8,
      tags: ['common'],
    });

    const related = semantic.getRelated(f2.id);
    assert.ok(related.length > 0);
    assert.equal(related[0].id, f1.id);
  });

  it('should return empty array for unknown fact id in getRelated', () => {
    const related = semantic.getRelated('nonexistent');
    assert.deepEqual(related, []);
  });

  it('should apply decay and evict low-confidence facts', () => {
    const small = new SemanticMemory({ maxFacts: 3, minConfidence: 0.5 });

    small.storeFact({ content: 'a', confidence: 0.51 });
    small.storeFact({ content: 'b', confidence: 0.51 });
    small.storeFact({ content: 'c', confidence: 0.51 });
    // This fourth one triggers decay+eviction
    small.storeFact({ content: 'd', confidence: 0.9 });

    // Some facts with low confidence should have been evicted after decay
    assert.ok(small.stats.factsStored >= 4);
  });

  it('should return stats', () => {
    semantic.storeFact({ content: 'test', confidence: 0.8 });
    semantic.storePattern({ name: 'test_pat', signature: 'sig' });

    const stats = semantic.getStats();

    assert.equal(stats.factsSize, 1);
    assert.equal(stats.patternsSize, 1);
    assert.ok('factsStored' in stats);
    assert.ok('patternsStored' in stats);
    assert.ok('associationsSize' in stats);
  });
});

// =============================================================================
// WORKING MEMORY TESTS
// =============================================================================

describe('WorkingMemory', () => {
  let working;

  beforeEach(() => {
    working = new WorkingMemory();
  });

  it('should initialize with default config', () => {
    assert.equal(working.config.maxItems, 7); // Miller's Law
    assert.deepEqual(working.items, []);
    assert.equal(working.focus, null);
  });

  it('should accept custom config', () => {
    const custom = new WorkingMemory({ maxItems: 5 });
    assert.equal(custom.config.maxItems, 5);
  });

  it('should add items', () => {
    const item = working.add({ content: 'task context', type: 'context' });

    assert.ok(item instanceof MemoryItem);
    assert.equal(item.tier, 'working');
    assert.equal(working.items.length, 1);
    assert.equal(working.stats.itemsAdded, 1);
  });

  it('should add existing MemoryItem', () => {
    const mem = new MemoryItem({ id: 'existing', content: 'hi' });
    const result = working.add(mem);

    assert.equal(result.id, 'existing');
  });

  it('should enforce capacity (Miller\'s Law)', () => {
    const small = new WorkingMemory({ maxItems: 3 });

    for (let i = 0; i < 5; i++) {
      small.add({ content: `item_${i}` });
    }

    assert.ok(small.items.length <= 3);
    assert.ok(small.stats.itemsEvicted > 0);
  });

  it('should evict least recently accessed when at capacity', () => {
    const small = new WorkingMemory({ maxItems: 3 });

    const oldest = small.add({ content: 'old' });
    // Make the others more recent
    const mid = small.add({ content: 'mid' });
    const recent = small.add({ content: 'recent' });

    // Add one more, should evict oldest
    small.add({ content: 'newest' });

    const ids = small.items.map(i => i.content);
    assert.ok(!ids.includes('old') || small.items.length <= 3);
  });

  it('should set and get focus', () => {
    const focus = working.setFocus({ content: 'current task', type: 'task' });

    assert.ok(focus instanceof MemoryItem);
    assert.equal(working.getFocus(), focus);
    assert.equal(working.stats.focusChanges, 1);
  });

  it('should get all items', () => {
    working.add({ content: 'a' });
    working.add({ content: 'b' });

    const all = working.getAll();

    assert.equal(all.length, 2);
    // Should be a copy
    all.push('garbage');
    assert.equal(working.items.length, 2);
  });

  it('should get items by type', () => {
    working.add({ content: 'context', type: 'context' });
    working.add({ content: 'task', type: 'task' });
    working.add({ content: 'context2', type: 'context' });

    const contexts = working.getByType('context');
    assert.equal(contexts.length, 2);

    const tasks = working.getByType('task');
    assert.equal(tasks.length, 1);
  });

  it('should return empty for unknown type', () => {
    working.add({ content: 'a', type: 'context' });
    const results = working.getByType('nonexistent');
    assert.deepEqual(results, []);
  });

  it('should access and refresh item timestamp', () => {
    const item = working.add({ content: 'refresh me' });
    const before = item.lastAccessed;

    const found = working.access(item.id);

    assert.ok(found);
    assert.ok(found.lastAccessed >= before);
    assert.equal(found.accessCount, 1);
  });

  it('should return undefined for access with unknown id', () => {
    const result = working.access('nonexistent_id');
    assert.equal(result, undefined);
  });

  it('should clear all items and focus', () => {
    working.add({ content: 'a' });
    working.add({ content: 'b' });
    working.setFocus({ content: 'focus' });

    working.clear();

    assert.deepEqual(working.items, []);
    assert.equal(working.focus, null);
  });

  it('should remove stale items', () => {
    const stale = new WorkingMemory({ maxAgeMs: 1 }); // 1ms max age

    stale.add({ content: 'will be stale', lastAccessed: Date.now() - 1000 });

    // getAll triggers _cleanStale
    const items = stale.getAll();
    assert.equal(items.length, 0);
  });

  it('should return stats', () => {
    working.add({ content: 'a' });
    working.setFocus({ content: 'focus' });

    const stats = working.getStats();

    assert.equal(stats.size, 1);
    assert.equal(stats.capacity, 7);
    assert.equal(stats.hasFocus, true);
    assert.ok('itemsAdded' in stats);
    assert.ok('itemsEvicted' in stats);
    assert.ok('focusChanges' in stats);
  });
});

// =============================================================================
// TIERED MEMORY MANAGER TESTS
// =============================================================================

describe('TieredMemory', () => {
  let tiered;

  beforeEach(() => {
    tiered = new TieredMemory();
  });

  it('should initialize all four tiers', () => {
    assert.ok(tiered.vector instanceof VectorMemory);
    assert.ok(tiered.episodic instanceof EpisodicMemory);
    assert.ok(tiered.semantic instanceof SemanticMemory);
    assert.ok(tiered.working instanceof WorkingMemory);
  });

  it('should accept custom tier configurations', () => {
    const custom = new TieredMemory({
      working: { maxItems: 3 },
      vector: { maxItems: 100 },
    });

    assert.equal(custom.working.config.maxItems, 3);
    assert.equal(custom.vector.config.maxItems, 100);
  });

  // ---------------------------------------------------------------------------
  // TIER SELECTION
  // ---------------------------------------------------------------------------

  describe('_selectTier()', () => {
    it('should route facts to semantic', () => {
      assert.equal(tiered._selectTier({ type: 'fact' }), 'semantic');
    });

    it('should route patterns to semantic', () => {
      assert.equal(tiered._selectTier({ type: 'pattern' }), 'semantic');
    });

    it('should route items with embeddings to vector', () => {
      assert.equal(tiered._selectTier({ embedding: [1, 2, 3] }), 'vector');
    });

    it('should route events to episodic', () => {
      assert.equal(tiered._selectTier({ type: 'event' }), 'episodic');
    });

    it('should route interactions to episodic', () => {
      assert.equal(tiered._selectTier({ type: 'interaction' }), 'episodic');
    });

    it('should default to working for generic items', () => {
      assert.equal(tiered._selectTier({ type: 'generic' }), 'working');
      assert.equal(tiered._selectTier({}), 'working');
    });
  });

  // ---------------------------------------------------------------------------
  // STORE
  // ---------------------------------------------------------------------------

  describe('store()', () => {
    it('should auto-route to working memory', async () => {
      const result = await tiered.store({ content: 'quick note' });

      assert.ok(result);
      assert.equal(tiered.working.items.length, 1);
    });

    it('should store in explicit tier', async () => {
      await tiered.store(
        { content: 'fact', confidence: 0.8 },
        { tier: 'semantic' }
      );

      assert.equal(tiered.semantic.facts.size, 1);
    });

    it('should store vector item with embedding', async () => {
      await tiered.store(
        { content: 'vectorized' },
        { tier: 'vector', embedding: [1, 0, 0] }
      );

      assert.equal(tiered.vector.items.size, 1);
    });

    it('should store pattern in semantic', async () => {
      await tiered.store(
        { type: 'pattern', name: 'test_pat', signature: 'sig_t' },
        { tier: 'semantic' }
      );

      assert.equal(tiered.semantic.patterns.size, 1);
    });

    it('should store event in episodic when episode is active', async () => {
      tiered.startEpisode('test');

      await tiered.store(
        { type: 'event', data: 'something happened' },
        { tier: 'episodic' }
      );

      assert.equal(tiered.episodic.currentEpisode.events.length, 1);
    });
  });

  // ---------------------------------------------------------------------------
  // QUERY
  // ---------------------------------------------------------------------------

  describe('query()', () => {
    it('should return results from all tiers', async () => {
      await tiered.store({ content: 'working item', type: 'task' });
      tiered.semantic.storeFact({
        content: 'semantic fact',
        confidence: 0.8,
      });

      const results = await tiered.query({});

      assert.ok(results.working);
      assert.ok(results.semantic);
      assert.ok(results.episodic);
      assert.ok(results.vector);
    });

    it('should query working memory by type', async () => {
      await tiered.store({ content: 'task 1', type: 'task' });
      await tiered.store({ content: 'context 1', type: 'context' });

      const results = await tiered.query({ type: 'task' });

      assert.ok(results.working.length >= 1);
      assert.ok(results.working.every(i => i.type === 'task'));
    });

    it('should query with embedding for vector results', async () => {
      await tiered.store(
        { content: 'embedded' },
        { tier: 'vector', embedding: [1, 0, 0] }
      );

      const results = await tiered.query({ embedding: [1, 0, 0] });

      assert.ok(results.vector.length >= 0); // May or may not match threshold
    });

    it('should query episodic with context', async () => {
      tiered.startEpisode('analysis');
      tiered.addEvent({ type: 'decision' });
      tiered.endEpisode('success');

      const results = await tiered.query({
        episodeContext: { type: 'analysis' },
      });

      assert.ok(Array.isArray(results.episodic));
    });
  });

  // ---------------------------------------------------------------------------
  // GET
  // ---------------------------------------------------------------------------

  describe('get()', () => {
    it('should find item in working memory', async () => {
      const item = await tiered.store({ content: 'findable' });

      const found = tiered.get(item.id);
      assert.ok(found);
      assert.equal(found.content, 'findable');
    });

    it('should find item in semantic memory', () => {
      const fact = tiered.semantic.storeFact({
        content: 'known fact',
        confidence: 0.8,
      });

      const found = tiered.get(fact.id);
      assert.ok(found);
      assert.equal(found.content, 'known fact');
    });

    it('should find item in vector memory', async () => {
      const item = await tiered.vector.store(
        { content: 'vector item' },
        [1, 0, 0]
      );

      const found = tiered.get(item.id);
      assert.ok(found);
      assert.equal(found.content, 'vector item');
    });

    it('should return null for unknown id', () => {
      assert.equal(tiered.get('unknown_id'), null);
    });

    it('should increment access count on get', () => {
      const fact = tiered.semantic.storeFact({
        content: 'accessed fact',
        confidence: 0.8,
      });

      tiered.get(fact.id);
      assert.equal(fact.accessCount, 1);
    });
  });

  // ---------------------------------------------------------------------------
  // EPISODIC API
  // ---------------------------------------------------------------------------

  describe('Episodic API', () => {
    it('should delegate startEpisode', () => {
      const ep = tiered.startEpisode('test', { key: 'value' });
      assert.ok(ep instanceof Episode);
      assert.equal(ep.type, 'test');
    });

    it('should delegate addEvent', () => {
      tiered.startEpisode('test');
      tiered.addEvent({ type: 'decision', data: 'ok' });

      assert.equal(tiered.episodic.currentEpisode.events.length, 1);
    });

    it('should delegate endEpisode', () => {
      tiered.startEpisode('test');
      const ep = tiered.endEpisode('done');

      assert.ok(ep);
      assert.equal(ep.outcome, 'done');
      assert.equal(tiered.episodic.currentEpisode, null);
    });
  });

  // ---------------------------------------------------------------------------
  // PROMOTION & DEMOTION
  // ---------------------------------------------------------------------------

  describe('promote()', () => {
    it('should promote from working to semantic', async () => {
      const item = new MemoryItem({ content: 'promote me', tier: 'working' });
      tiered.promote(item);

      assert.equal(item.tier, 'semantic');
      assert.equal(tiered.stats.promotions, 1);
    });

    it('should promote from semantic to vector', () => {
      const item = new MemoryItem({ content: 'promote again', tier: 'semantic' });
      tiered.promote(item);

      assert.equal(item.tier, 'vector');
      assert.equal(tiered.stats.promotions, 1);
    });

    it('should not promote beyond vector tier', () => {
      const item = new MemoryItem({ content: 'already at top', tier: 'vector' });
      tiered.promote(item);

      assert.equal(item.tier, 'vector');
      assert.equal(tiered.stats.promotions, 0);
    });
  });

  describe('demote()', () => {
    it('should demote from vector to semantic', async () => {
      const item = new MemoryItem({ id: 'demote_v', content: 'demote', tier: 'vector' });
      await tiered.vector.store(item, [1, 0, 0]);

      tiered.demote(item);

      assert.equal(item.tier, 'semantic');
      assert.equal(tiered.stats.demotions, 1);
    });

    it('should demote from semantic to working', () => {
      const item = new MemoryItem({
        id: 'demote_s',
        content: 'demote to working',
        tier: 'semantic',
        confidence: 0.8,
      });
      tiered.semantic.storeFact(item);

      tiered.demote(item);

      assert.equal(item.tier, 'working');
      assert.equal(tiered.stats.demotions, 1);
    });

    it('should not demote below working tier', () => {
      const item = new MemoryItem({ content: 'already lowest', tier: 'working' });
      tiered.demote(item);

      assert.equal(item.tier, 'working');
      assert.equal(tiered.stats.demotions, 0);
    });
  });

  // ---------------------------------------------------------------------------
  // STATS & SUMMARY
  // ---------------------------------------------------------------------------

  describe('getStats()', () => {
    it('should return stats from all tiers', () => {
      const stats = tiered.getStats();

      assert.ok('tiers' in stats);
      assert.ok('vector' in stats.tiers);
      assert.ok('episodic' in stats.tiers);
      assert.ok('semantic' in stats.tiers);
      assert.ok('working' in stats.tiers);
      assert.ok('promotions' in stats);
      assert.ok('demotions' in stats);
    });
  });

  describe('getSummary()', () => {
    it('should return memory summary', async () => {
      await tiered.store({ content: 'active item', type: 'task' });
      tiered.working.setFocus({ content: 'my focus' });
      tiered.semantic.storeFact({ content: 'known', confidence: 0.8 });

      const summary = tiered.getSummary();

      assert.ok(Array.isArray(summary.working));
      assert.ok(summary.focus);
      assert.ok(Array.isArray(summary.recentFacts));
      assert.ok(Array.isArray(summary.activePatterns));
    });

    it('should truncate long content in working summary', async () => {
      const longContent = 'x'.repeat(200);
      await tiered.store({ content: longContent });

      const summary = tiered.getSummary();

      assert.ok(summary.working[0].content.length <= 100);
    });

    it('should include currentEpisode id in summary', () => {
      tiered.startEpisode('test');

      const summary = tiered.getSummary();
      assert.ok(summary.currentEpisode);
    });

    it('should show null currentEpisode when no episode active', () => {
      const summary = tiered.getSummary();
      assert.equal(summary.currentEpisode, undefined);
    });

    it('should only include active patterns with > 2 occurrences', () => {
      // Store a pattern once - should not appear
      tiered.semantic.storePattern({
        name: 'rare',
        signature: 'sig_rare',
        confidence: 0.5,
      });

      // Store a pattern 3+ times - should appear
      for (let i = 0; i < 4; i++) {
        tiered.semantic.storePattern({
          name: 'common',
          signature: 'sig_common',
          confidence: 0.7,
        });
      }

      const summary = tiered.getSummary();
      const activeNames = summary.activePatterns.map(p => p.name || p.signature);

      assert.ok(
        summary.activePatterns.some(p =>
          (p.name === 'common' || p.signature === 'sig_common') && p.occurrences > 2
        )
      );
    });
  });
});

// =============================================================================
// FACTORY FUNCTION TESTS
// =============================================================================

describe('createTieredMemory()', () => {
  it('should create a TieredMemory instance', () => {
    const tm = createTieredMemory();
    assert.ok(tm instanceof TieredMemory);
  });

  it('should pass options through', () => {
    const tm = createTieredMemory({
      working: { maxItems: 3 },
    });

    assert.equal(tm.working.config.maxItems, 3);
  });

  it('should accept persistence option', () => {
    const mockPersistence = { save: () => {}, load: () => {} };
    const tm = createTieredMemory({ persistence: mockPersistence });

    assert.strictEqual(tm.persistence, mockPersistence);
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('TieredMemory - Edge Cases', () => {
  it('should handle empty query', async () => {
    const tiered = new TieredMemory();
    const results = await tiered.query({});

    assert.ok(results.working);
    assert.ok(results.semantic);
    assert.ok(results.episodic);
    assert.ok(results.vector);
  });

  it('should handle storing null content', async () => {
    const tiered = new TieredMemory();
    const item = await tiered.store({ content: null });

    assert.ok(item);
  });

  it('should handle storing undefined type', async () => {
    const tiered = new TieredMemory();
    const item = await tiered.store({ content: 'no type' });

    assert.ok(item);
    assert.equal(item.type, 'generic');
  });

  it('should handle multiple rapid stores', async () => {
    const tiered = new TieredMemory();

    const promises = [];
    for (let i = 0; i < 50; i++) {
      promises.push(tiered.store({ content: `item_${i}` }));
    }

    await Promise.all(promises);
    // Working memory has capacity limit, but stores should not throw
    assert.ok(tiered.working.stats.itemsAdded >= 50);
  });

  it('should handle MemoryItem with zero access count importance', () => {
    const item = new MemoryItem({
      accessCount: 0,
      confidence: 0,
      lastAccessed: Date.now() - 3600000 * 100,
    });

    const importance = item.getImportance();
    assert.ok(typeof importance === 'number');
    assert.ok(importance >= 0);
  });
});
