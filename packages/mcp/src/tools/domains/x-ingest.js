/**
 * X/Twitter Ingest Tools
 *
 * Zero-cost ingestion of X content into LocalXStore.
 * Uses the free oEmbed API (no API key required) or accepts raw text.
 *
 * "Le chien ramasse les os sans payer" - κυνικός
 *
 * @module @cynic/mcp/tools/domains/x-ingest
 */

'use strict';

import { createLogger, PHI_INV, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('XIngest');

const MAX_CONFIDENCE = PHI_INV; // 0.618

// oEmbed endpoint (free, no auth)
const OEMBED_URL = 'https://publish.twitter.com/oembed';

// Valid X/Twitter URL patterns
const X_URL_PATTERN = /^https?:\/\/(x\.com|twitter\.com)\/(\w+)\/status\/(\d+)/;

/**
 * Extract tweet ID and username from an X/Twitter URL
 * @param {string} url
 * @returns {{ tweetId: string, username: string } | null}
 */
function parseXUrl(url) {
  const match = url.match(X_URL_PATTERN);
  if (!match) return null;
  return { tweetId: match[3], username: match[2] };
}

/**
 * Extract tweet text from oEmbed HTML
 * The HTML is a blockquote containing the tweet text and metadata
 * @param {string} html
 * @returns {string}
 */
function extractTextFromOembedHtml(html) {
  if (!html) return '';

  // Remove the trailing citation and link elements
  let text = html
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<a[^>]*>pic\.twitter\.com[^<]*<\/a>/gi, '')
    .replace(/<a[^>]*>([^<]*)<\/a>/gi, '$1')
    .replace(/<blockquote[^>]*>/gi, '')
    .replace(/<\/blockquote>/gi, '')
    .replace(/<p[^>]*>/gi, '')
    .replace(/<\/p>/gi, '\n')
    .replace(/&mdash;.*$/s, '')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/<[^>]+>/g, '')
    .trim();

  // Clean up multiple newlines
  text = text.replace(/\n{3,}/g, '\n\n').trim();

  return text;
}

/**
 * Extract hashtags from tweet text
 * @param {string} text
 * @returns {string[]}
 */
function extractHashtags(text) {
  const matches = text.match(/#\w+/g);
  return matches ? matches.map(h => h.slice(1).toLowerCase()) : [];
}

/**
 * Extract mentions from tweet text
 * @param {string} text
 * @returns {string[]}
 */
function extractMentions(text) {
  const matches = text.match(/@\w+/g);
  return matches ? matches.map(m => m.slice(1).toLowerCase()) : [];
}

/**
 * Fetch tweet data via oEmbed API (free, no auth)
 * @param {string} url - Tweet URL
 * @returns {Promise<Object|null>}
 */
async function fetchOembed(url) {
  const oembedUrl = `${OEMBED_URL}?url=${encodeURIComponent(url)}&omit_script=true`;

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 8000);

  try {
    const response = await fetch(oembedUrl, {
      signal: controller.signal,
      headers: { 'User-Agent': 'CYNIC/1.0 (tweet ingestion)' },
    });

    if (!response.ok) {
      if (response.status === 404) return null; // Tweet deleted or private
      throw new Error(`oEmbed returned ${response.status}`);
    }

    return await response.json();
  } catch (err) {
    if (err.name === 'AbortError') {
      log.debug('oEmbed timeout', { url });
      return null;
    }
    throw err;
  } finally {
    clearTimeout(timeout);
  }
}

/**
 * Create X Ingest tool
 * @param {Object} localXStore - LocalXStore instance
 * @returns {Object} Tool definition
 */
export function createXIngestTool(localXStore) {
  return {
    name: 'brain_x_ingest',
    description: `Ingest tweets into CYNIC's local X store. Two modes:
1. URL mode: Provide a tweet URL — fetches via free oEmbed API (no API key needed)
2. Text mode: Provide raw text + username — stores directly
Supports single or bulk ingestion (up to 20 URLs). All data stays LOCAL.`,
    inputSchema: {
      type: 'object',
      properties: {
        url: {
          type: 'string',
          description: 'Single tweet URL (https://x.com/user/status/123)',
        },
        urls: {
          type: 'array',
          items: { type: 'string' },
          description: 'Multiple tweet URLs for bulk ingestion (max 20)',
        },
        text: {
          type: 'string',
          description: 'Raw tweet text (for manual mode, requires username)',
        },
        username: {
          type: 'string',
          description: 'Author username without @ (required for text mode)',
        },
        displayName: {
          type: 'string',
          description: 'Author display name (optional, for text mode)',
        },
        isMyTweet: {
          type: 'boolean',
          description: 'Mark as your own tweet (default: false)',
        },
      },
    },
    handler: async (params) => {
      const {
        url,
        urls,
        text,
        username,
        displayName,
        isMyTweet = false,
      } = params;

      if (!localXStore) {
        return {
          success: false,
          error: 'Local X store not available.',
          message: '*head tilt* LocalXStore not initialized.',
        };
      }

      // Determine mode
      const allUrls = urls
        ? urls.slice(0, 20)
        : url ? [url] : [];

      if (allUrls.length === 0 && !text) {
        return {
          success: false,
          error: 'Provide url, urls, or text+username',
        };
      }

      // Text mode: manual ingestion
      if (text && !url && !urls) {
        if (!username) {
          return { success: false, error: 'username required for text mode' };
        }
        return _ingestManual(localXStore, { text, username, displayName, isMyTweet });
      }

      // URL mode: oEmbed ingestion
      return _ingestUrls(localXStore, allUrls, { isMyTweet });
    },
  };
}

/**
 * Ingest a tweet from manual text input
 * @param {Object} store - LocalXStore
 * @param {Object} data - Tweet data
 * @returns {Object} Result
 */
function _ingestManual(store, { text, username, displayName, isMyTweet }) {
  try {
    // Generate a pseudo-ID for manual entries
    const tweetId = `manual_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

    // Upsert user
    store.upsertUser({
      x_user_id: `manual_${username}`,
      username,
      display_name: displayName || username,
    });

    // Create tweet
    const tweet = store.createTweet({
      tweet_id: tweetId,
      x_user_id: `manual_${username}`,
      text,
      created_at: new Date().toISOString(),
      hashtags: extractHashtags(text),
      mentions: extractMentions(text),
      is_my_tweet: isMyTweet,
      source: 'manual',
    });

    // Emit SOCIAL_CAPTURE → event bus
    if (tweet) {
      try {
        globalEventBus.publish(EventType.SOCIAL_CAPTURE, {
          source: 'manual', tweets: 1, users: 1,
        }, { source: 'XIngest' });
      } catch { /* non-blocking */ }
    }

    return {
      success: true,
      mode: 'manual',
      ingested: tweet ? 1 : 0,
      tweet: tweet ? {
        id: tweetId,
        text: text.slice(0, 200),
        author: username,
      } : null,
      message: tweet
        ? `*tail wag* Tweet from @${username} ingested locally.`
        : `*sniff* Tweet already existed.`,
    };
  } catch (err) {
    log.error('Manual ingest error', { error: err.message });
    return { success: false, error: err.message };
  }
}

/**
 * Ingest tweets from URLs via oEmbed
 * @param {Object} store - LocalXStore
 * @param {string[]} urls - Tweet URLs
 * @param {Object} options
 * @returns {Promise<Object>} Result
 */
async function _ingestUrls(store, urls, { isMyTweet = false }) {
  const results = { ingested: 0, skipped: 0, failed: 0, tweets: [] };

  // Validate URLs first
  const validUrls = [];
  for (const u of urls) {
    const parsed = parseXUrl(u);
    if (parsed) {
      validUrls.push({ url: u, ...parsed });
    } else {
      results.failed++;
      results.tweets.push({ url: u, status: 'invalid_url' });
    }
  }

  // Fetch in parallel (batches of 5 to be polite)
  const batchSize = 5;
  for (let i = 0; i < validUrls.length; i += batchSize) {
    const batch = validUrls.slice(i, i + batchSize);

    const oembedResults = await Promise.allSettled(
      batch.map(({ url }) => fetchOembed(url)),
    );

    for (let j = 0; j < batch.length; j++) {
      const { url, tweetId, username } = batch[j];
      const result = oembedResults[j];

      if (result.status === 'rejected' || !result.value) {
        results.failed++;
        results.tweets.push({
          url,
          status: 'fetch_failed',
          error: result.status === 'rejected' ? result.reason?.message : 'not found',
        });
        continue;
      }

      const oembed = result.value;

      try {
        // Extract data from oEmbed
        const text = extractTextFromOembedHtml(oembed.html);
        const authorUsername = oembed.author_url
          ? oembed.author_url.replace(/^https?:\/\/(x\.com|twitter\.com)\//, '')
          : username;

        // Upsert user
        store.upsertUser({
          x_user_id: `oembed_${authorUsername}`,
          username: authorUsername,
          display_name: oembed.author_name || authorUsername,
        });

        // Create tweet
        const tweet = store.createTweet({
          tweet_id: tweetId,
          x_user_id: `oembed_${authorUsername}`,
          text,
          created_at: new Date().toISOString(),
          hashtags: extractHashtags(text),
          mentions: extractMentions(text),
          is_my_tweet: isMyTweet,
          source: 'oembed',
        });

        if (tweet) {
          results.ingested++;
          results.tweets.push({
            url,
            status: 'ingested',
            id: tweetId,
            text: text.slice(0, 100),
            author: authorUsername,
          });
        } else {
          results.skipped++;
          results.tweets.push({ url, status: 'duplicate', id: tweetId });
        }
      } catch (err) {
        results.failed++;
        results.tweets.push({ url, status: 'store_error', error: err.message });
      }
    }
  }

  // Emit SOCIAL_CAPTURE → event bus (batch)
  if (results.ingested > 0) {
    try {
      globalEventBus.publish(EventType.SOCIAL_CAPTURE, {
        source: 'oembed', tweets: results.ingested, users: results.ingested,
      }, { source: 'XIngest' });
    } catch { /* non-blocking */ }
  }

  const total = urls.length;
  const confidence = Math.min(results.ingested / Math.max(total, 1), MAX_CONFIDENCE);

  return {
    success: true,
    mode: 'oembed',
    summary: {
      total,
      ingested: results.ingested,
      skipped: results.skipped,
      failed: results.failed,
    },
    tweets: results.tweets,
    confidence: Math.round(confidence * 100) / 100,
    message: results.ingested > 0
      ? `*tail wag* ${results.ingested}/${total} tweets ingested locally.`
      : `*sniff* No new tweets ingested (${results.skipped} duplicates, ${results.failed} failed).`,
    privacy: 'All data stored locally in ~/.cynic/x-local.db',
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

export const xIngestFactory = {
  name: 'x-ingest',
  domain: 'social',
  requires: ['localXStore'],

  create(options) {
    const { localXStore } = options;

    if (!localXStore) {
      log.debug('X ingest tools skipped: localXStore not available');
      return [];
    }

    return [
      createXIngestTool(localXStore),
    ];
  },
};

export default xIngestFactory;
