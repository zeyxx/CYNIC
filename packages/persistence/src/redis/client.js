/**
 * Redis Client - Session & Cache Manager
 *
 * Uses ioredis for Redis connections.
 * Handles sessions, library cache, and ephemeral state.
 *
 * @module @cynic/persistence/redis
 */

'use strict';

import Redis from 'ioredis';
import { secureToken } from '@cynic/core';

// Singleton instance
let redis = null;

/**
 * Default TTLs (φ-derived, in seconds)
 */
export const TTL = {
  SESSION: 61800,        // ~17h - φ⁻¹ × 100000
  LIBRARY_CACHE: 86400,  // 24h - library docs cache
  PATTERN_CACHE: 3600,   // 1h - pattern cache
  RATE_LIMIT: 60,        // 1min - rate limit window
  LOCK: 30,              // 30s - distributed lock
};

/**
 * Key prefixes for namespacing
 */
export const PREFIX = {
  SESSION: 'cynic:session:',
  USER: 'cynic:user:',
  LIBRARY: 'cynic:lib:',
  PATTERN: 'cynic:pattern:',
  RATE: 'cynic:rate:',
  LOCK: 'cynic:lock:',
  JUDGMENT: 'cynic:jdg:',
};

/**
 * Lua script for atomic lock release (Redis EVAL, not JS eval)
 * This is safe - it runs server-side in Redis, not in Node.js
 */
const RELEASE_LOCK_SCRIPT = `
  if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
  else
    return 0
  end
`;

/**
 * Redis Client wrapper
 */
export class RedisClient {
  constructor(url) {
    this.url = url || process.env.CYNIC_REDIS_URL;
    this.client = null;
  }

  /**
   * Connect to Redis
   */
  async connect() {
    if (this.client) return this;

    if (!this.url) {
      throw new Error('CYNIC_REDIS_URL not set');
    }

    this.client = new Redis(this.url, {
      maxRetriesPerRequest: 3,
      retryDelayOnFailover: 100,
      lazyConnect: true,
    });

    await this.client.connect();
    console.log('🐕 Redis connected');

    return this;
  }

  /**
   * Disconnect from Redis
   */
  async close() {
    if (this.client) {
      await this.client.quit();
      this.client = null;
      console.log('🐕 Redis disconnected');
    }
  }

  // ==========================================================================
  // BASIC OPERATIONS
  // ==========================================================================

  async get(key) {
    const value = await this.client.get(key);
    try {
      return value ? JSON.parse(value) : null;
    } catch {
      return value;
    }
  }

  async set(key, value, ttl = null) {
    const serialized = typeof value === 'string' ? value : JSON.stringify(value);
    if (ttl) {
      await this.client.setex(key, ttl, serialized);
    } else {
      await this.client.set(key, serialized);
    }
  }

  async del(key) {
    return this.client.del(key);
  }

  async exists(key) {
    return this.client.exists(key);
  }

  async expire(key, ttl) {
    return this.client.expire(key, ttl);
  }

  async ttl(key) {
    return this.client.ttl(key);
  }

  // ==========================================================================
  // SESSION OPERATIONS
  // ==========================================================================

  async getSession(sessionId) {
    return this.get(`${PREFIX.SESSION}${sessionId}`);
  }

  async setSession(sessionId, data, ttl = TTL.SESSION) {
    await this.set(`${PREFIX.SESSION}${sessionId}`, data, ttl);
  }

  async deleteSession(sessionId) {
    return this.del(`${PREFIX.SESSION}${sessionId}`);
  }

  async touchSession(sessionId, ttl = TTL.SESSION) {
    return this.expire(`${PREFIX.SESSION}${sessionId}`, ttl);
  }

  // ==========================================================================
  // LIBRARY CACHE
  // ==========================================================================

  async getLibraryDoc(libraryId, query) {
    const key = `${PREFIX.LIBRARY}${libraryId}:${this._hash(query)}`;
    return this.get(key);
  }

  async setLibraryDoc(libraryId, query, content, ttl = TTL.LIBRARY_CACHE) {
    const key = `${PREFIX.LIBRARY}${libraryId}:${this._hash(query)}`;
    await this.set(key, content, ttl);
  }

  // ==========================================================================
  // RATE LIMITING
  // ==========================================================================

  async checkRateLimit(identifier, limit = 100, window = TTL.RATE_LIMIT) {
    const key = `${PREFIX.RATE}${identifier}`;
    const current = await this.client.incr(key);

    if (current === 1) {
      await this.client.expire(key, window);
    }

    return {
      allowed: current <= limit,
      current,
      limit,
      remaining: Math.max(0, limit - current),
    };
  }

  // ==========================================================================
  // DISTRIBUTED LOCK (using Redis Lua scripting for atomicity)
  // ==========================================================================

  async acquireLock(resource, ttl = TTL.LOCK) {
    const key = `${PREFIX.LOCK}${resource}`;
    const token = secureToken();

    const acquired = await this.client.set(key, token, 'EX', ttl, 'NX');
    return acquired ? token : null;
  }

  /**
   * Release a distributed lock atomically
   * Uses Redis EVAL (Lua scripting) - NOT JavaScript eval
   * This is safe: Lua runs server-side in Redis
   */
  async releaseLock(resource, token) {
    const key = `${PREFIX.LOCK}${resource}`;
    // ioredis evalsha/eval runs Lua on Redis server, not JS
    return this.client.call('EVAL', RELEASE_LOCK_SCRIPT, 1, key, token);
  }

  // ==========================================================================
  // HELPERS
  // ==========================================================================

  _hash(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      hash = ((hash << 5) - hash) + str.charCodeAt(i);
      hash = hash & hash;
    }
    return Math.abs(hash).toString(36);
  }

  /**
   * Health check
   */
  async health() {
    try {
      const start = Date.now();
      await this.client.ping();
      const latency = Date.now() - start;

      const info = await this.client.info('memory');
      const memMatch = info.match(/used_memory_human:(\S+)/);

      return {
        status: 'healthy',
        latency,
        memory: memMatch ? memMatch[1] : 'unknown',
      };
    } catch (error) {
      return {
        status: 'unhealthy',
        error: error.message,
      };
    }
  }
}

/**
 * Get shared Redis instance (singleton)
 */
export function getRedis(url) {
  if (!redis) {
    redis = new RedisClient(url);
  }
  return redis;
}

export default RedisClient;
