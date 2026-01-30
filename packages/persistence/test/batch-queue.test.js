/**
 * @cynic/persistence - BatchQueue Tests
 *
 * v1.1: Tests for batch operations
 *
 * @module @cynic/persistence/test/batch-queue
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { BatchQueue, DEFAULT_BATCH_CONFIG, createTableBatchQueue } from '../src/batch-queue.js';

// =============================================================================
// DEFAULT CONFIG TESTS
// =============================================================================

describe('DEFAULT_BATCH_CONFIG', () => {
  it('should have expected values', () => {
    assert.strictEqual(DEFAULT_BATCH_CONFIG.batchSize, 13); // Fibonacci
    assert.strictEqual(DEFAULT_BATCH_CONFIG.flushIntervalMs, 5000);
    assert.strictEqual(DEFAULT_BATCH_CONFIG.maxQueueSize, 89); // Fibonacci
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(DEFAULT_BATCH_CONFIG));
  });
});

// =============================================================================
// BATCH QUEUE TESTS
// =============================================================================

describe('BatchQueue', () => {
  let queue;
  let flushedItems;

  beforeEach(() => {
    flushedItems = [];
  });

  afterEach(async () => {
    if (queue && !queue.isClosed) {
      await queue.close();
    }
  });

  describe('Construction', () => {
    it('should require flushFn', () => {
      assert.throws(() => new BatchQueue(), /requires.*flushFn/i);
    });

    it('should create with options', () => {
      queue = new BatchQueue({
        name: 'test-queue',
        flushFn: async () => {},
        batchSize: 5,
        flushIntervalMs: 1000,
      });

      assert.strictEqual(queue.length, 0);
      assert.ok(queue.isEmpty);
      assert.ok(!queue.isClosed);
    });

    it('should use defaults', () => {
      queue = new BatchQueue({
        flushFn: async () => {},
      });

      assert.strictEqual(queue.length, 0);
    });
  });

  describe('Add', () => {
    it('should add items to queue', async () => {
      queue = new BatchQueue({
        name: 'add-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 5,
        flushIntervalMs: 60000, // Long interval to prevent auto-flush
      });

      await queue.add({ id: 1 });
      await queue.add({ id: 2 });

      assert.strictEqual(queue.length, 2);
      assert.ok(!queue.isEmpty);
    });

    it('should auto-flush when batch size reached', async () => {
      queue = new BatchQueue({
        name: 'batch-flush-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 3,
        flushIntervalMs: 60000,
      });

      await queue.add({ id: 1 });
      await queue.add({ id: 2 });
      await queue.add({ id: 3 });

      // Wait for async flush
      await new Promise(resolve => setTimeout(resolve, 50));

      assert.strictEqual(flushedItems.length, 3);
      assert.strictEqual(queue.length, 0);
    });

    it('should force flush when max queue size reached', async () => {
      queue = new BatchQueue({
        name: 'max-flush-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 100, // High batch size
        maxQueueSize: 5, // Low max queue size
        flushIntervalMs: 60000,
      });

      // Add items up to max queue size
      for (let i = 1; i <= 5; i++) {
        await queue.add({ id: i });
      }

      assert.strictEqual(flushedItems.length, 5);
      assert.strictEqual(queue.length, 0);
    });

    it('should throw when adding to closed queue', async () => {
      queue = new BatchQueue({
        name: 'closed-test',
        flushFn: async () => {},
        flushIntervalMs: 60000,
      });

      await queue.close();

      await assert.rejects(
        () => queue.add({ id: 1 }),
        /closed/i,
      );
    });
  });

  describe('AddMany', () => {
    it('should add multiple items', async () => {
      queue = new BatchQueue({
        name: 'add-many-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 10,
        flushIntervalMs: 60000,
      });

      await queue.addMany([{ id: 1 }, { id: 2 }, { id: 3 }]);

      assert.strictEqual(queue.length, 3);
    });

    it('should handle empty array', async () => {
      queue = new BatchQueue({
        name: 'empty-test',
        flushFn: async () => {},
        flushIntervalMs: 60000,
      });

      await queue.addMany([]);

      assert.strictEqual(queue.length, 0);
    });
  });

  describe('Flush', () => {
    it('should flush manually', async () => {
      queue = new BatchQueue({
        name: 'manual-flush-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 100,
        flushIntervalMs: 60000,
      });

      await queue.add({ id: 1 });
      await queue.add({ id: 2 });

      const count = await queue.flush();

      assert.strictEqual(count, 2);
      assert.strictEqual(flushedItems.length, 2);
      assert.strictEqual(queue.length, 0);
    });

    it('should return 0 for empty queue', async () => {
      queue = new BatchQueue({
        name: 'empty-flush-test',
        flushFn: async () => {},
        flushIntervalMs: 60000,
      });

      const count = await queue.flush();

      assert.strictEqual(count, 0);
    });

    it('should handle flush errors gracefully', async () => {
      let errorCaught = null;

      queue = new BatchQueue({
        name: 'error-test',
        flushFn: async () => { throw new Error('Flush failed'); },
        flushIntervalMs: 60000,
        onError: (err) => { errorCaught = err; },
      });

      await queue.add({ id: 1 });
      await queue.flush();

      assert.ok(errorCaught);
      assert.strictEqual(errorCaught.message, 'Flush failed');
      // Items should be put back on queue
      assert.strictEqual(queue.length, 1);
    });
  });

  describe('Periodic Flush', () => {
    it('should flush on interval', async () => {
      queue = new BatchQueue({
        name: 'interval-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 100,
        flushIntervalMs: 50, // Short interval for testing
      });

      await queue.add({ id: 1 });
      await queue.add({ id: 2 });

      // Wait for interval flush
      await new Promise(resolve => setTimeout(resolve, 100));

      assert.strictEqual(flushedItems.length, 2);
    });
  });

  describe('Close', () => {
    it('should flush remaining items on close', async () => {
      queue = new BatchQueue({
        name: 'close-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 100,
        flushIntervalMs: 60000,
      });

      await queue.add({ id: 1 });
      await queue.add({ id: 2 });

      await queue.close();

      assert.strictEqual(flushedItems.length, 2);
      assert.ok(queue.isClosed);
    });
  });

  describe('Stats', () => {
    it('should track statistics', async () => {
      queue = new BatchQueue({
        name: 'stats-test',
        flushFn: async (items) => { flushedItems.push(...items); },
        batchSize: 2,
        flushIntervalMs: 60000,
      });

      await queue.add({ id: 1 });
      await queue.add({ id: 2 });

      // Wait for async flush
      await new Promise(resolve => setTimeout(resolve, 50));

      const stats = queue.getStats();

      assert.strictEqual(stats.totalAdded, 2);
      assert.strictEqual(stats.totalFlushed, 2);
      assert.strictEqual(stats.flushCount, 1);
      assert.strictEqual(stats.errors, 0);
      assert.ok(stats.lastFlush);
    });
  });
});

// =============================================================================
// CREATE TABLE BATCH QUEUE TESTS
// =============================================================================

describe('createTableBatchQueue', () => {
  it('should create a batch queue for a table', async () => {
    const insertedRows = [];
    const mockClient = {
      batchInsert: mock.fn(async (table, columns, rows) => {
        insertedRows.push(...rows);
        return { rows: [], rowCount: rows.length };
      }),
    };

    const queue = createTableBatchQueue({
      client: mockClient,
      table: 'test_table',
      columns: ['name', 'value'],
      rowMapper: (item) => [item.name, item.value],
      queueOptions: {
        batchSize: 2,
        flushIntervalMs: 60000,
      },
    });

    await queue.add({ name: 'item1', value: 100 });
    await queue.add({ name: 'item2', value: 200 });

    // Wait for async flush
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.strictEqual(insertedRows.length, 2);
    assert.deepStrictEqual(insertedRows[0], ['item1', 100]);
    assert.deepStrictEqual(insertedRows[1], ['item2', 200]);

    await queue.close();
  });

  it('should use batchUpsert when conflictColumns provided', async () => {
    const mockClient = {
      batchUpsert: mock.fn(async () => ({ rows: [], rowCount: 0 })),
    };

    const queue = createTableBatchQueue({
      client: mockClient,
      table: 'test_table',
      columns: ['name', 'value'],
      rowMapper: (item) => [item.name, item.value],
      conflictColumns: ['name'],
      queueOptions: {
        batchSize: 1,
        flushIntervalMs: 60000,
      },
    });

    await queue.add({ name: 'item1', value: 100 });

    // Wait for async flush
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.strictEqual(mockClient.batchUpsert.mock.calls.length, 1);

    await queue.close();
  });
});
