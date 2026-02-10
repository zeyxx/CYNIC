/**
 * X/Twitter API Response Parser
 *
 * Parses Twitter's GraphQL API responses to extract
 * tweets, users, and trends from captured traffic.
 *
 * "CYNIC sees what you see" - κυνικός
 *
 * @module @cynic/mcp/services/x-parser
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('XParser');

/**
 * Parse X/Twitter GraphQL response
 * @param {string} endpoint - API endpoint path
 * @param {Object} data - Response data
 * @returns {Object} Parsed data { tweets: [], users: [], trends: [] }
 */
export function parseXResponse(endpoint, data) {
  const result = {
    tweets: [],
    users: [],
    trends: [],
  };

  try {
    // Determine response type based on endpoint
    if (endpoint.includes('HomeTimeline') || endpoint.includes('HomeLatestTimeline')) {
      parseHomeTimeline(data, result);
    } else if (endpoint.includes('UserTweets') || endpoint.includes('UserTweetsAndReplies')) {
      parseUserTweets(data, result);
    } else if (endpoint.includes('TweetDetail')) {
      parseTweetDetail(data, result);
    } else if (endpoint.includes('SearchTimeline')) {
      parseSearchTimeline(data, result);
    } else if (endpoint.includes('ListLatestTweetsTimeline')) {
      parseListTimeline(data, result);
    } else if (endpoint.includes('Explore') || endpoint.includes('GenericTimelineById')) {
      parseTrends(data, result);
    } else if (endpoint.includes('UserByScreenName') || endpoint.includes('UserByRestId')) {
      parseUserProfile(data, result);
    } else {
      // Generic tweet extraction fallback
      extractTweetsRecursive(data, result);
    }
  } catch (err) {
    log.debug('Parse error', { endpoint, error: err.message });
  }

  return result;
}

/**
 * Parse home timeline response
 */
function parseHomeTimeline(data, result) {
  const instructions = data?.data?.home?.home_timeline_urt?.instructions ||
                       data?.data?.viewer?.home_timeline?.timeline?.instructions ||
                       [];

  for (const instruction of instructions) {
    if (instruction.type === 'TimelineAddEntries') {
      for (const entry of instruction.entries || []) {
        extractTweetFromEntry(entry, result);
      }
    }
  }
}

/**
 * Parse user tweets response
 */
function parseUserTweets(data, result) {
  const instructions = data?.data?.user?.result?.timeline_v2?.timeline?.instructions ||
                       data?.data?.user?.result?.timeline?.timeline?.instructions ||
                       [];

  for (const instruction of instructions) {
    if (instruction.type === 'TimelineAddEntries') {
      for (const entry of instruction.entries || []) {
        extractTweetFromEntry(entry, result);
      }
    }
  }
}

/**
 * Parse tweet detail (single tweet + conversation)
 */
function parseTweetDetail(data, result) {
  const instructions = data?.data?.threaded_conversation_with_injections_v2?.instructions ||
                       data?.data?.tweetResult?.result?.tweet_results?.instructions ||
                       [];

  for (const instruction of instructions) {
    if (instruction.type === 'TimelineAddEntries') {
      for (const entry of instruction.entries || []) {
        extractTweetFromEntry(entry, result);
      }
    }
  }

  // Also check for single tweet result
  const tweetResult = data?.data?.tweetResult?.result ||
                      data?.data?.tweet?.result;
  if (tweetResult) {
    extractTweetFromResult(tweetResult, result);
  }
}

/**
 * Parse search timeline response
 */
function parseSearchTimeline(data, result) {
  const instructions = data?.data?.search_by_raw_query?.search_timeline?.timeline?.instructions ||
                       data?.data?.search?.timeline?.timeline?.instructions ||
                       [];

  for (const instruction of instructions) {
    if (instruction.type === 'TimelineAddEntries') {
      for (const entry of instruction.entries || []) {
        extractTweetFromEntry(entry, result);
      }
    }
  }
}

/**
 * Parse list timeline response
 */
function parseListTimeline(data, result) {
  const instructions = data?.data?.list?.tweets_timeline?.timeline?.instructions || [];

  for (const instruction of instructions) {
    if (instruction.type === 'TimelineAddEntries') {
      for (const entry of instruction.entries || []) {
        extractTweetFromEntry(entry, result);
      }
    }
  }
}

/**
 * Parse trends/explore response
 */
function parseTrends(data, result) {
  const instructions = data?.data?.viewer?.explore_response?.timeline?.instructions ||
                       data?.data?.timeline?.timeline?.instructions ||
                       [];

  for (const instruction of instructions) {
    if (instruction.type === 'TimelineAddEntries') {
      for (const entry of instruction.entries || []) {
        // Check for trend items
        const trend = entry?.content?.itemContent?.trend;
        if (trend) {
          result.trends.push({
            trendName: trend.name,
            trendType: trend.name?.startsWith('#') ? 'hashtag' : 'keyword',
            tweetVolume: parseTrendVolume(trend.trend_metadata?.meta_description),
            rank: entry?.sortIndex || null,
            location: trend.trend_metadata?.domain_context || null,
          });
        }
        // Also extract any tweets shown with trends
        extractTweetFromEntry(entry, result);
      }
    }
  }
}

/**
 * Parse user profile response
 */
function parseUserProfile(data, result) {
  const user = data?.data?.user?.result ||
               data?.data?.user_result?.result;

  if (user) {
    extractUserFromResult(user, result);
  }
}

/**
 * Extract tweet from timeline entry
 */
function extractTweetFromEntry(entry, result) {
  const content = entry?.content;

  // Tweet entry
  if (content?.entryType === 'TimelineTimelineItem' && content?.itemContent?.tweet_results) {
    extractTweetFromResult(content.itemContent.tweet_results.result, result);
  }

  // Module (e.g., conversation thread)
  if (content?.entryType === 'TimelineTimelineModule') {
    for (const item of content.items || []) {
      if (item?.item?.itemContent?.tweet_results) {
        extractTweetFromResult(item.item.itemContent.tweet_results.result, result);
      }
    }
  }
}

/**
 * Extract tweet from GraphQL result object
 */
function extractTweetFromResult(tweetResult, result) {
  if (!tweetResult) return;

  // Handle tweet with visibility results wrapper
  const tweet = tweetResult.tweet || tweetResult;
  if (!tweet?.legacy) return;

  const legacy = tweet.legacy;
  const coreRaw = tweet.core?.user_results?.result;

  // Unwrap user wrapper (UserWithVisibilityResults → .result)
  const core = (coreRaw && !coreRaw.legacy && coreRaw.result?.legacy)
    ? coreRaw.result
    : coreRaw;

  // Extract user if present
  if (core) {
    extractUserFromResult(core, result);
  }

  // Build tweet object (camelCase — canonical format for all consumers)
  const tweetData = {
    tweetId: legacy.id_str || tweet.rest_id,
    xUserId: legacy.user_id_str || core?.rest_id,
    text: legacy.full_text || legacy.text || '',
    textHtml: null,
    language: legacy.lang || null,
    tweetType: determineTweetType(legacy),
    replyToTweetId: legacy.in_reply_to_status_id_str || null,
    replyToUserId: legacy.in_reply_to_user_id_str || null,
    quoteTweetId: legacy.quoted_status_id_str || null,
    threadId: legacy.conversation_id_str || null,
    media: extractMedia(legacy),
    urls: extractUrls(legacy),
    hashtags: extractHashtags(legacy),
    mentions: extractMentions(legacy),
    likesCount: legacy.favorite_count || 0,
    retweetsCount: legacy.retweet_count || 0,
    repliesCount: legacy.reply_count || 0,
    quotesCount: legacy.quote_count || 0,
    viewsCount: parseInt(tweet.views?.count) || 0,
    bookmarksCount: legacy.bookmark_count || 0,
    postedAt: new Date(legacy.created_at),
    isRetweet: !!legacy.retweeted_status_result,
    isQuote: !!legacy.quoted_status_id_str,
    source: 'proxy',
  };

  // Avoid duplicates
  if (!result.tweets.find(t => t.tweetId === tweetData.tweetId)) {
    result.tweets.push(tweetData);
  }

  // Recursively extract quoted tweet
  if (tweet.quoted_status_result?.result) {
    extractTweetFromResult(tweet.quoted_status_result.result, result);
  }

  // Recursively extract retweeted tweet
  if (legacy.retweeted_status_result?.result) {
    extractTweetFromResult(legacy.retweeted_status_result.result, result);
  }
}

/**
 * Extract user from GraphQL result object
 *
 * Handles multiple Twitter API wrapper types:
 * - Direct User object (has .legacy.screen_name)
 * - UserWithVisibilityResults (has .result containing the real User)
 * - UserUnavailable (has no .legacy — skipped)
 */
function extractUserFromResult(userResult, result) {
  if (!userResult) return;

  // Unwrap UserWithVisibilityResults and similar wrappers
  // These have .result containing the actual User object
  let user = userResult;
  if (!user.legacy && user.result?.legacy) {
    user = user.result;
  }

  if (!user?.legacy) return;

  const legacy = user.legacy;
  const username = legacy.screen_name || legacy.screenName;

  // Skip users without username — can't store without it
  if (!username) return;

  const userData = {
    xUserId: user.rest_id,
    username,
    displayName: legacy.name,
    bio: legacy.description || null,
    location: legacy.location || null,
    website: legacy.entities?.url?.urls?.[0]?.expanded_url || null,
    profileImageUrl: legacy.profile_image_url_https?.replace('_normal', '_400x400') || null,
    bannerUrl: legacy.profile_banner_url || null,
    followersCount: legacy.followers_count || 0,
    followingCount: legacy.friends_count || 0,
    tweetCount: legacy.statuses_count || 0,
    verified: user.is_blue_verified || legacy.verified || false,
    createdOnX: legacy.created_at ? new Date(legacy.created_at) : null,
  };

  // Avoid duplicates
  if (!result.users.find(u => u.xUserId === userData.xUserId)) {
    result.users.push(userData);
  }
}

/**
 * Determine tweet type from legacy data
 */
function determineTweetType(legacy) {
  if (legacy.in_reply_to_status_id_str) return 'reply';
  if (legacy.retweeted_status_result) return 'retweet';
  if (legacy.quoted_status_id_str) return 'quote';
  if (legacy.self_thread) return 'thread';
  return 'tweet';
}

/**
 * Extract media from legacy entities
 */
function extractMedia(legacy) {
  const media = [];

  for (const m of legacy.extended_entities?.media || legacy.entities?.media || []) {
    media.push({
      type: m.type || 'photo',
      url: m.media_url_https || m.url,
      expandedUrl: m.expanded_url,
      thumbnail: m.media_url_https + ':thumb',
      altText: m.ext_alt_text || null,
      duration: m.video_info?.duration_millis || null,
    });
  }

  return media;
}

/**
 * Extract URLs from legacy entities
 */
function extractUrls(legacy) {
  const urls = [];

  for (const u of legacy.entities?.urls || []) {
    urls.push({
      url: u.url,
      expandedUrl: u.expanded_url,
      displayUrl: u.display_url,
    });
  }

  return urls;
}

/**
 * Extract hashtags from legacy entities
 */
function extractHashtags(legacy) {
  return (legacy.entities?.hashtags || []).map(h => h.text.toLowerCase());
}

/**
 * Extract mentions from legacy entities
 */
function extractMentions(legacy) {
  return (legacy.entities?.user_mentions || []).map(m => m.screen_name);
}

/**
 * Parse trend volume from meta description
 */
function parseTrendVolume(description) {
  if (!description) return null;

  // Examples: "10K Tweets", "1.2M Tweets", "500 Tweets"
  const match = description.match(/([\d.]+)([KMB]?)\s*(?:Tweets|posts)/i);
  if (!match) return null;

  let volume = parseFloat(match[1]);
  const suffix = match[2]?.toUpperCase();

  if (suffix === 'K') volume *= 1000;
  else if (suffix === 'M') volume *= 1000000;
  else if (suffix === 'B') volume *= 1000000000;

  return Math.round(volume);
}

/**
 * Recursively extract tweets from unknown structure
 */
function extractTweetsRecursive(obj, result, depth = 0) {
  if (depth > 10 || !obj || typeof obj !== 'object') return;

  // Check if this looks like a tweet result
  if (obj.legacy?.full_text && obj.rest_id) {
    extractTweetFromResult(obj, result);
    return;
  }

  // Check for tweet_results wrapper
  if (obj.tweet_results?.result) {
    extractTweetFromResult(obj.tweet_results.result, result);
    return;
  }

  // Recurse into arrays and objects
  if (Array.isArray(obj)) {
    for (const item of obj) {
      extractTweetsRecursive(item, result, depth + 1);
    }
  } else {
    for (const key of Object.keys(obj)) {
      extractTweetsRecursive(obj[key], result, depth + 1);
    }
  }
}

/**
 * Known GraphQL mutations → action types
 */
const MUTATION_MAP = {
  FavoriteTweet:    'like',
  UnfavoriteTweet:  'unlike',
  CreateRetweet:    'retweet',
  DeleteRetweet:    'unretweet',
  CreateBookmark:   'bookmark',
  DeleteBookmark:   'unbookmark',
  CreateTweet:      'tweet',
  DeleteTweet:      'delete_tweet',
  // User mutations removed: only tracking tweet-related actions
};

/**
 * Parse a GraphQL mutation from request body
 * @param {string} endpoint - API endpoint path (contains mutation name)
 * @param {Object} requestBody - Parsed JSON request body
 * @returns {Object|null} Action object or null if not a tracked mutation
 */
export function parseMutation(endpoint, requestBody) {
  try {
    // Extract mutation name from endpoint: /i/api/graphql/{queryId}/FavoriteTweet
    const mutationName = endpoint.split('/').pop()?.split('?')[0];
    const actionType = MUTATION_MAP[mutationName];

    if (!actionType) return null;

    const vars = requestBody?.variables || {};

    return {
      actionType,
      tweetId: vars.tweet_id || vars.source_tweet_id || null,
      targetUserId: vars.user_id || null,
      rawVariables: vars,
      mutationName,
    };
  } catch {
    return null;
  }
}

export default { parseXResponse, parseMutation };
