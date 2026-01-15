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
import 'dotenv/config';
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
