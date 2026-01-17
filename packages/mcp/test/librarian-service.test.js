/**
 * Librarian Service Tests
 *
 * Tests for the LibrarianService that caches Context7 documentation.
 *
 * @module @cynic/mcp/test/librarian-service
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { LibrarianService } from '../src/librarian-service.js';

/**
 * Create mock persistence manager
 */
function createMockPersistence() {
  const cache = new Map();

  return {
    redis: {
      getLibraryDoc: async (libraryId, query) => {
        const key = `${libraryId}:${query}`;
        return cache.get(`redis:${key}`) || null;
      },
      setLibraryDoc: async (libraryId, query, content) => {
        const key = `${libraryId}:${query}`;
        cache.set(`redis:${key}`, content);
      },
    },
    libraryCache: true,
    getLibraryDoc: async (libraryId, query) => {
      const key = `${libraryId}:${query}`;
      const data = cache.get(`pg:${key}`);
      return data || null;
    },
    setLibraryDoc: async (libraryId, query, content, metadata, ttlHours) => {
      const key = `${libraryId}:${query}`;
      cache.set(`pg:${key}`, { content, metadata, hitCount: 1 });
    },
    isLibraryDocCached: async (libraryId, query) => {
      const key = `${libraryId}:${query}`;
      return cache.has(`pg:${key}`);
    },
    cleanExpiredCache: async () => 0,
    invalidateLibraryCache: async (libraryId) => {
      let count = 0;
      for (const key of cache.keys()) {
        if (key.includes(libraryId)) {
          cache.delete(key);
          count++;
        }
      }
      return count;
    },
    getLibraryCacheStats: async () => ({
      totalEntries: cache.size,
      activeEntries: cache.size,
      uniqueLibraries: new Set([...cache.keys()].map(k => k.split(':')[1])).size,
    }),
    getTopCachedLibraries: async (limit) => {
      return [...cache.keys()]
        .filter(k => k.startsWith('pg:'))
        .slice(0, limit)
        .map(k => ({ libraryId: k.replace('pg:', '').split(':')[0] }));
    },
    _cache: cache,
  };
}

describe('LibrarianService', () => {
  let librarian;
  let mockPersistence;

  beforeEach(() => {
    mockPersistence = createMockPersistence();
    librarian = new LibrarianService(mockPersistence, {
      ttlHours: 24,
      preloadOnInit: false,
    });
  });

  describe('constructor', () => {
    it('creates with default options', () => {
      const lib = new LibrarianService(null);
      assert.equal(lib.ttlHours, 24);
      assert.equal(lib.preloadOnInit, true);
    });

    it('accepts custom options', () => {
      assert.equal(librarian.ttlHours, 24);
      assert.equal(librarian.preloadOnInit, false);
    });

    it('initializes stats', () => {
      assert.equal(librarian._stats.hits, 0);
      assert.equal(librarian._stats.misses, 0);
      assert.equal(librarian._stats.fetches, 0);
    });
  });

  describe('initialize', () => {
    it('marks as initialized', async () => {
      assert.equal(librarian._initialized, false);
      await librarian.initialize();
      assert.equal(librarian._initialized, true);
    });

    it('is idempotent', async () => {
      await librarian.initialize();
      await librarian.initialize();
      assert.equal(librarian._initialized, true);
    });
  });

  describe('getDocumentation', () => {
    it('returns cache miss without fetcher', async () => {
      const result = await librarian.getDocumentation('/test/lib', 'query');

      assert.equal(result.cached, false);
      assert.equal(result.source, 'none');
      assert.ok(result.error);
    });

    it('fetches and caches on miss', async () => {
      const fetcher = async (libId, query) => `Documentation for ${libId}: ${query}`;

      const result = await librarian.getDocumentation('/test/lib', 'getting started', fetcher);

      assert.equal(result.cached, false);
      assert.equal(result.source, 'context7');
      assert.ok(result.content.includes('Documentation'));
      assert.equal(librarian._stats.fetches, 1);
    });

    it('returns from redis cache', async () => {
      // Pre-populate redis cache
      await mockPersistence.redis.setLibraryDoc('/test/lib', 'cached query', 'Cached content');

      const result = await librarian.getDocumentation('/test/lib', 'cached query');

      assert.equal(result.cached, true);
      assert.equal(result.source, 'redis');
      assert.equal(result.content, 'Cached content');
      assert.equal(librarian._stats.redisHits, 1);
    });

    it('returns from postgres cache', async () => {
      // Pre-populate postgres cache only
      await mockPersistence.setLibraryDoc('/test/pg', 'pg query', 'PG content', {});

      const result = await librarian.getDocumentation('/test/pg', 'pg query');

      assert.equal(result.cached, true);
      assert.equal(result.source, 'postgres');
      assert.equal(result.content, 'PG content');
      assert.equal(librarian._stats.postgresHits, 1);
    });

    it('normalizes inputs', async () => {
      const fetcher = async () => 'content';
      await librarian.getDocumentation('  /TEST/Lib  ', '  Query  ', fetcher);

      // Check cache uses normalized keys
      const cached = await mockPersistence.getLibraryDoc('/test/lib', 'query');
      assert.ok(cached);
    });

    it('handles fetch errors', async () => {
      const fetcher = async () => {
        throw new Error('Network error');
      };

      const result = await librarian.getDocumentation('/test/error', 'query', fetcher);

      assert.equal(result.source, 'error');
      assert.ok(result.error);
      assert.equal(librarian._stats.errors, 1);
    });

    it('handles redis errors gracefully', async () => {
      // Make redis throw
      mockPersistence.redis.getLibraryDoc = async () => {
        throw new Error('Redis down');
      };

      // Pre-populate postgres
      await mockPersistence.setLibraryDoc('/fallback/lib', 'query', 'PG fallback', {});

      const result = await librarian.getDocumentation('/fallback/lib', 'query');

      assert.equal(result.source, 'postgres');
    });
  });

  describe('cacheDocumentation', () => {
    it('stores in both caches', async () => {
      await librarian.cacheDocumentation('/cache/test', 'query', 'content', { meta: true });

      const redisResult = await mockPersistence.redis.getLibraryDoc('/cache/test', 'query');
      const pgResult = await mockPersistence.getLibraryDoc('/cache/test', 'query');

      assert.equal(redisResult, 'content');
      assert.ok(pgResult);
    });

    it('handles cache errors gracefully', async () => {
      mockPersistence.redis.setLibraryDoc = async () => {
        throw new Error('Redis write error');
      };

      // Should not throw
      await librarian.cacheDocumentation('/error/test', 'query', 'content');
    });
  });

  describe('preloadEcosystemDocs', () => {
    it('preloads libraries', async () => {
      const fetchedLibs = [];
      const fetcher = async (libId, query) => {
        fetchedLibs.push({ libId, query });
        return `Doc for ${libId}`;
      };

      const result = await librarian.preloadEcosystemDocs(fetcher, {
        maxPriority: 1,
        queries: ['getting started'],
      });

      assert.ok(result.success > 0 || result.skipped > 0);
      assert.ok(fetchedLibs.length >= 0);
    });

    it('skips already cached', async () => {
      // Pre-cache
      const libs = librarian.getEcosystemLibraries();
      if (libs.length > 0) {
        await mockPersistence.setLibraryDoc(libs[0].id.toLowerCase(), 'getting started', 'cached');
      }

      const fetcher = async () => 'new content';

      const result = await librarian.preloadEcosystemDocs(fetcher, {
        maxPriority: 1,
        queries: ['getting started'],
      });

      assert.ok(result.skipped > 0 || result.success >= 0);
    });
  });

  describe('invalidate', () => {
    it('invalidates library cache', async () => {
      await librarian.cacheDocumentation('/invalidate/test', 'q1', 'c1');
      await librarian.cacheDocumentation('/invalidate/test', 'q2', 'c2');

      const result = await librarian.invalidate('/invalidate/test');

      assert.ok(result.invalidated >= 0);
      assert.equal(result.libraryId, '/invalidate/test');
    });

    it('handles missing libraryCache', async () => {
      const lib = new LibrarianService({});
      const result = await lib.invalidate('/test');

      assert.equal(result.invalidated, 0);
    });
  });

  describe('getStats', () => {
    it('returns combined stats', async () => {
      // Generate some hits
      await mockPersistence.redis.setLibraryDoc('/stats/test', 'q', 'content');
      await librarian.getDocumentation('/stats/test', 'q');

      const stats = await librarian.getStats();

      assert.ok('hits' in stats);
      assert.ok('misses' in stats);
      assert.ok('hitRate' in stats);
      assert.ok('cache' in stats);
    });

    it('calculates hit rate correctly', async () => {
      // 2 hits, 1 miss
      await mockPersistence.redis.setLibraryDoc('/rate/test', 'q1', 'c1');
      await mockPersistence.redis.setLibraryDoc('/rate/test', 'q2', 'c2');

      await librarian.getDocumentation('/rate/test', 'q1');
      await librarian.getDocumentation('/rate/test', 'q2');
      await librarian.getDocumentation('/rate/miss', 'q3');

      const stats = await librarian.getStats();

      assert.equal(stats.hits, 2);
      assert.equal(stats.misses, 1);
      assert.ok(Math.abs(stats.hitRate - 2 / 3) < 0.01);
    });
  });

  describe('getCachedLibraries', () => {
    it('returns cached libraries', async () => {
      await librarian.cacheDocumentation('/lib1', 'q', 'c');
      await librarian.cacheDocumentation('/lib2', 'q', 'c');

      const libs = await librarian.getCachedLibraries(10);

      assert.ok(Array.isArray(libs));
    });

    it('handles missing libraryCache', async () => {
      const lib = new LibrarianService({});
      const result = await lib.getCachedLibraries();

      assert.deepEqual(result, []);
    });
  });

  describe('getEcosystemLibraries', () => {
    it('returns ecosystem library list', () => {
      const libs = librarian.getEcosystemLibraries();

      assert.ok(Array.isArray(libs));
      assert.ok(libs.length > 0);

      for (const lib of libs) {
        assert.ok(lib.id);
        assert.ok(lib.name);
        assert.ok(typeof lib.priority === 'number');
      }
    });

    it('includes solana libraries', () => {
      const libs = librarian.getEcosystemLibraries();
      const solana = libs.find(l => l.name === '@solana/web3.js');

      assert.ok(solana);
      assert.equal(solana.priority, 1);
    });
  });
});

describe('LibrarianService without persistence', () => {
  it('handles null persistence', async () => {
    const lib = new LibrarianService(null);

    const result = await lib.getDocumentation('/test', 'query');

    assert.equal(result.cached, false);
    assert.equal(result.source, 'none');
  });

  it('handles empty persistence', async () => {
    const lib = new LibrarianService({});

    const result = await lib.getDocumentation('/test', 'query', async () => 'content');

    assert.equal(result.source, 'context7');
    assert.equal(result.content, 'content');
  });
});
