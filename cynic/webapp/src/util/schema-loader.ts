/**
 * CYNIC Schema Loader
 * Fetches and caches OrganismSchema from the backend API
 * Cache TTL: 1 hour (3600000 ms)
 */

import { apiClient } from '../api/client';
import type { OrganismSchema } from '../types/api';

/**
 * Cache configuration
 */
const CACHE_TTL = 3600000; // 1 hour in milliseconds

/**
 * Private cache state
 */
let cachedSchema: OrganismSchema | null = null;
let schemaExpiry: number = 0;

/**
 * Load the organism schema from the backend API with caching
 *
 * Fetches fresh schema from /api/organism/schema, caches for 1 hour,
 * and returns from cache on subsequent calls. If fetch fails but cache
 * exists, logs warning and returns cached schema (graceful degradation).
 *
 * @returns Promise resolving to OrganismSchema
 * @throws Error if fetch fails and no cache exists
 */
export async function loadSchema(): Promise<OrganismSchema> {
  // Check if cache is still valid
  const now = Date.now();
  if (cachedSchema !== null && now < schemaExpiry) {
    console.log('*sniff* Using cached schema');
    return cachedSchema;
  }

  // Cache miss or expired - fetch from API
  try {
    console.log('Fetching schema from API...');
    const schema = await apiClient.getSchema();

    // Update cache with new data
    cachedSchema = schema;
    schemaExpiry = now + CACHE_TTL;

    console.log('*tail wag* Schema loaded and cached');
    return schema;
  } catch (error) {
    // If we have a cached schema (even expired), use it
    if (cachedSchema !== null) {
      console.warn(
        `Failed to fetch schema: ${error instanceof Error ? error.message : String(error)}. ` +
        'Using cached schema.'
      );
      return cachedSchema;
    }

    // No cache available - throw error
    throw new Error(
      `Failed to load schema: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Get the currently cached schema without fetching
 *
 * Returns the cached schema if it exists and is not expired,
 * otherwise returns null.
 *
 * @returns Cached OrganismSchema or null if not available
 */
export function getSchema(): OrganismSchema | null {
  const now = Date.now();
  if (cachedSchema !== null && now < schemaExpiry) {
    return cachedSchema;
  }
  return null;
}

/**
 * Clear the schema cache and force a refresh on next load
 *
 * Use this to invalidate the cache and fetch fresh data
 * from the API on the next loadSchema() call.
 */
export function clearCache(): void {
  console.log('*sniff* Clearing schema cache');
  cachedSchema = null;
  schemaExpiry = 0;
}
