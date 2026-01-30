#!/usr/bin/env node
/**
 * Connection Test - Verify PostgreSQL and Redis connections
 *
 * Skips gracefully if connection strings are not provided.
 *
 * @module @cynic/persistence/test/connection
 */

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import dotenv from 'dotenv';

// Load .env from monorepo root
const __dirname = dirname(fileURLToPath(import.meta.url));
dotenv.config({ path: resolve(__dirname, '../../../.env') });

import { PostgresClient } from '../src/postgres/client.js';
import { RedisClient } from '../src/redis/client.js';

describe('Persistence Connections', () => {
  describe('PostgreSQL', () => {
    const hasPostgres = !!process.env.CYNIC_DATABASE_URL;
    let db;

    before(async () => {
      if (hasPostgres) {
        db = new PostgresClient();
        await db.connect();
      }
    });

    after(async () => {
      if (db) {
        await db.close();
      }
    });

    it('should connect and query', { skip: !hasPostgres }, async () => {
      const { rows } = await db.query('SELECT NOW() as time, current_database() as db');
      assert.ok(rows[0].time, 'Should return current time');
      assert.ok(rows[0].db, 'Should return database name');
    });

    it('should report health', { skip: !hasPostgres }, async () => {
      const health = await db.health();
      assert.strictEqual(health.status, 'healthy', 'Should be healthy');
      assert.ok(typeof health.latency === 'number', 'Should have latency');
      assert.ok(health.pool, 'Should have pool info');
    });

    // v1.1: Batch Operations
    it('should batch insert rows', { skip: !hasPostgres }, async () => {
      // Create a temp table for testing
      const tableName = `test_batch_${Date.now()}`;
      await db.query(`CREATE TEMP TABLE ${tableName} (id SERIAL, name TEXT, value INT)`);

      const result = await db.batchInsert(tableName, ['name', 'value'], [
        ['item1', 100],
        ['item2', 200],
        ['item3', 300],
      ]);

      assert.strictEqual(result.rowCount, 3, 'Should insert 3 rows');

      const { rows } = await db.query(`SELECT * FROM ${tableName} ORDER BY value`);
      assert.strictEqual(rows.length, 3);
      assert.strictEqual(rows[0].name, 'item1');
      assert.strictEqual(rows[2].value, 300);
    });

    it('should batch upsert with conflict handling', { skip: !hasPostgres }, async () => {
      const tableName = `test_upsert_${Date.now()}`;
      await db.query(`CREATE TEMP TABLE ${tableName} (name TEXT PRIMARY KEY, value INT)`);

      // Initial insert
      await db.batchUpsert(tableName, ['name', 'value'], [
        ['item1', 100],
        ['item2', 200],
      ], ['name']);

      // Upsert - item1 should update, item3 should insert
      await db.batchUpsert(tableName, ['name', 'value'], [
        ['item1', 150], // update
        ['item3', 300], // insert
      ], ['name']);

      const { rows } = await db.query(`SELECT * FROM ${tableName} ORDER BY name`);
      assert.strictEqual(rows.length, 3);
      assert.strictEqual(rows[0].value, 150, 'item1 should be updated to 150');
      assert.strictEqual(rows[2].name, 'item3', 'item3 should be inserted');
    });

    it('should batch execute multiple queries in transaction', { skip: !hasPostgres }, async () => {
      const tableName = `test_exec_${Date.now()}`;
      await db.query(`CREATE TEMP TABLE ${tableName} (msg TEXT)`);

      const results = await db.batchExecute([
        { sql: `INSERT INTO ${tableName} (msg) VALUES ($1)`, params: ['msg1'] },
        { sql: `INSERT INTO ${tableName} (msg) VALUES ($1)`, params: ['msg2'] },
        { sql: `INSERT INTO ${tableName} (msg) VALUES ($1)`, params: ['msg3'] },
      ]);

      assert.strictEqual(results.length, 3);
      results.forEach(r => assert.strictEqual(r.rowCount, 1));

      const { rows } = await db.query(`SELECT * FROM ${tableName}`);
      assert.strictEqual(rows.length, 3);
    });

    it('should return empty for batch insert with no rows', { skip: !hasPostgres }, async () => {
      const result = await db.batchInsert('any_table', ['col'], []);
      assert.strictEqual(result.rowCount, 0);
      assert.deepStrictEqual(result.rows, []);
    });
  });

  describe('Redis', () => {
    const hasRedis = !!process.env.CYNIC_REDIS_URL;
    let redis;

    before(async () => {
      if (hasRedis) {
        redis = new RedisClient();
        await redis.connect();
      }
    });

    after(async () => {
      if (redis) {
        await redis.close();
      }
    });

    it('should connect and set/get', { skip: !hasRedis }, async () => {
      const testKey = 'cynic:test:' + Date.now();
      await redis.set(testKey, { test: true, timestamp: Date.now() }, 60);
      const value = await redis.get(testKey);
      assert.strictEqual(value.test, true, 'Should retrieve stored value');
      await redis.del(testKey);
    });

    it('should report health', { skip: !hasRedis }, async () => {
      const health = await redis.health();
      assert.strictEqual(health.status, 'healthy', 'Should be healthy');
      assert.ok(typeof health.latency === 'number', 'Should have latency');
    });
  });
});
