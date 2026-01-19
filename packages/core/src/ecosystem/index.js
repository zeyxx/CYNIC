/**
 * CYNIC Ecosystem Monitor
 *
 * "Le chien qui surveille l'horizon" - Tracking the ecosystem
 *
 * Monitors external sources for updates, news, and signals:
 * 1. GitHub - Repos, commits, releases, issues
 * 2. Twitter/X - Mentions, announcements, sentiment (TODO)
 * 3. Web Search - News, articles, updates
 * 4. Source Discovery - Find new relevant sources
 *
 * @module @cynic/core/ecosystem
 * @philosophy Stay informed, stay skeptical
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

export const ECOSYSTEM_CONSTANTS = {
  /** Max items per source fetch (Fib(8) = 21) */
  MAX_ITEMS_PER_FETCH: 21,

  /** Cache TTL in ms (φ * 1 hour ≈ 97 min) */
  CACHE_TTL: Math.round(1.618 * 60 * 60 * 1000),

  /** Minimum fetch interval in ms (φ⁻¹ * 1 hour ≈ 37 min) */
  MIN_FETCH_INTERVAL: Math.round(PHI_INV * 60 * 60 * 1000),

  /** Relevance decay factor (φ⁻²) */
  RELEVANCE_DECAY: PHI_INV_2,

  /** Max sources to track (Fib(7) = 13) */
  MAX_SOURCES: 13,
};

// =============================================================================
// SOURCE TYPES
// =============================================================================

/**
 * Source types for ecosystem monitoring
 */
export const SourceType = {
  GITHUB: 'GITHUB',
  TWITTER: 'TWITTER',
  WEB: 'WEB',
  RSS: 'RSS',
  DISCORD: 'DISCORD',
  CUSTOM: 'CUSTOM',
};

/**
 * Update types from sources
 */
export const UpdateType = {
  COMMIT: 'COMMIT',
  RELEASE: 'RELEASE',
  ISSUE: 'ISSUE',
  PR: 'PR',
  ANNOUNCEMENT: 'ANNOUNCEMENT',
  NEWS: 'NEWS',
  MENTION: 'MENTION',
  ALERT: 'ALERT',
};

/**
 * Priority levels for updates
 */
export const Priority = {
  CRITICAL: 'CRITICAL',   // Breaking changes, security
  HIGH: 'HIGH',           // New features, major updates
  MEDIUM: 'MEDIUM',       // Regular updates
  LOW: 'LOW',             // Minor changes
  INFO: 'INFO',           // FYI only
};

// =============================================================================
// SOURCE BASE CLASS
// =============================================================================

/**
 * Base class for ecosystem sources
 */
// Counter for unique IDs
let sourceIdCounter = 0;

export class Source {
  constructor(config) {
    this.id = config.id || `src_${Date.now().toString(36)}_${(++sourceIdCounter).toString(36)}`;
    this.name = config.name;
    this.type = config.type || SourceType.CUSTOM;
    this.enabled = config.enabled !== false;
    this.priority = config.priority || Priority.MEDIUM;

    // Tracking
    this.lastFetch = null;
    this.fetchCount = 0;
    this.errorCount = 0;
    this.updates = [];

    // Rate limiting
    this.minInterval = config.minInterval || ECOSYSTEM_CONSTANTS.MIN_FETCH_INTERVAL;
  }

  /**
   * Check if source can be fetched (rate limiting)
   */
  canFetch() {
    if (!this.enabled) return false;
    if (!this.lastFetch) return true;
    return Date.now() - this.lastFetch >= this.minInterval;
  }

  /**
   * Fetch updates from source (override in subclass)
   * @returns {Promise<Array>} Array of updates
   */
  async fetch() {
    throw new Error('fetch() must be implemented by subclass');
  }

  /**
   * Mark fetch as complete
   */
  markFetched(updates = [], error = null) {
    this.lastFetch = Date.now();
    this.fetchCount++;
    if (error) {
      this.errorCount++;
    } else {
      this.updates = updates;
    }
  }

  toJSON() {
    return {
      id: this.id,
      name: this.name,
      type: this.type,
      enabled: this.enabled,
      priority: this.priority,
      lastFetch: this.lastFetch,
      fetchCount: this.fetchCount,
      errorCount: this.errorCount,
      updatesCount: this.updates.length,
    };
  }
}

// =============================================================================
// GITHUB SOURCE
// =============================================================================

/**
 * GitHub source for tracking repos
 */
export class GitHubSource extends Source {
  constructor(config) {
    super({
      ...config,
      type: SourceType.GITHUB,
      name: config.name || `GitHub: ${config.owner}/${config.repo}`,
    });

    this.owner = config.owner;
    this.repo = config.repo;
    this.branch = config.branch || 'main';
    this.trackCommits = config.trackCommits !== false;
    this.trackReleases = config.trackReleases !== false;
    this.trackIssues = config.trackIssues || false;
    this.trackPRs = config.trackPRs || false;

    // GitHub API base
    this.apiBase = 'https://api.github.com';
    // Safe access to process.env for Node.js or browser environments
    this.token = config.token || (typeof process !== 'undefined' && process.env?.GITHUB_TOKEN);
  }

  /**
   * Make GitHub API request
   */
  async _request(endpoint) {
    const headers = {
      'Accept': 'application/vnd.github.v3+json',
      'User-Agent': 'CYNIC-Ecosystem-Monitor',
    };
    if (this.token) {
      headers['Authorization'] = `token ${this.token}`;
    }

    const response = await fetch(`${this.apiBase}${endpoint}`, { headers });
    if (!response.ok) {
      throw new Error(`GitHub API error: ${response.status}`);
    }
    return response.json();
  }

  /**
   * Fetch all tracked data from GitHub
   */
  async fetch() {
    const updates = [];

    try {
      // Fetch commits
      if (this.trackCommits) {
        const commits = await this._request(
          `/repos/${this.owner}/${this.repo}/commits?sha=${this.branch}&per_page=10`
        );
        for (const commit of commits) {
          updates.push({
            type: UpdateType.COMMIT,
            source: this.id,
            sourceType: SourceType.GITHUB,
            id: commit.sha.slice(0, 7),
            title: commit.commit.message.split('\n')[0],
            description: commit.commit.message,
            url: commit.html_url,
            author: commit.author?.login || commit.commit.author.name,
            timestamp: new Date(commit.commit.author.date).getTime(),
            priority: this._inferCommitPriority(commit.commit.message),
            meta: {
              sha: commit.sha,
              repo: `${this.owner}/${this.repo}`,
            },
          });
        }
      }

      // Fetch releases
      if (this.trackReleases) {
        const releases = await this._request(
          `/repos/${this.owner}/${this.repo}/releases?per_page=5`
        );
        for (const release of releases) {
          updates.push({
            type: UpdateType.RELEASE,
            source: this.id,
            sourceType: SourceType.GITHUB,
            id: release.tag_name,
            title: release.name || release.tag_name,
            description: release.body,
            url: release.html_url,
            author: release.author?.login,
            timestamp: new Date(release.published_at).getTime(),
            priority: release.prerelease ? Priority.MEDIUM : Priority.HIGH,
            meta: {
              tag: release.tag_name,
              prerelease: release.prerelease,
              repo: `${this.owner}/${this.repo}`,
            },
          });
        }
      }

      // Fetch issues (if enabled)
      if (this.trackIssues) {
        const issues = await this._request(
          `/repos/${this.owner}/${this.repo}/issues?state=open&per_page=10`
        );
        for (const issue of issues) {
          if (issue.pull_request) continue; // Skip PRs
          updates.push({
            type: UpdateType.ISSUE,
            source: this.id,
            sourceType: SourceType.GITHUB,
            id: `#${issue.number}`,
            title: issue.title,
            description: issue.body,
            url: issue.html_url,
            author: issue.user?.login,
            timestamp: new Date(issue.created_at).getTime(),
            priority: this._inferIssuePriority(issue),
            meta: {
              number: issue.number,
              labels: issue.labels.map(l => l.name),
              repo: `${this.owner}/${this.repo}`,
            },
          });
        }
      }

      this.markFetched(updates);
      return updates;

    } catch (error) {
      this.markFetched([], error);
      throw error;
    }
  }

  /**
   * Infer priority from commit message
   */
  _inferCommitPriority(message) {
    const lower = message.toLowerCase();
    if (lower.includes('breaking') || lower.includes('security')) {
      return Priority.CRITICAL;
    }
    if (lower.includes('feat') || lower.startsWith('feat:')) {
      return Priority.HIGH;
    }
    if (lower.includes('fix') || lower.startsWith('fix:')) {
      return Priority.MEDIUM;
    }
    if (lower.includes('docs') || lower.includes('chore')) {
      return Priority.LOW;
    }
    return Priority.INFO;
  }

  /**
   * Infer priority from issue
   */
  _inferIssuePriority(issue) {
    const labels = issue.labels.map(l => l.name.toLowerCase());
    if (labels.some(l => l.includes('critical') || l.includes('security'))) {
      return Priority.CRITICAL;
    }
    if (labels.some(l => l.includes('bug') || l.includes('urgent'))) {
      return Priority.HIGH;
    }
    if (labels.some(l => l.includes('enhancement') || l.includes('feature'))) {
      return Priority.MEDIUM;
    }
    return Priority.LOW;
  }

  toJSON() {
    return {
      ...super.toJSON(),
      owner: this.owner,
      repo: this.repo,
      branch: this.branch,
      trackCommits: this.trackCommits,
      trackReleases: this.trackReleases,
      trackIssues: this.trackIssues,
      trackPRs: this.trackPRs,
    };
  }
}

// =============================================================================
// WEB SEARCH SOURCE
// =============================================================================

/**
 * Web search source for tracking news
 * Note: Requires external search API or scraping
 */
export class WebSearchSource extends Source {
  constructor(config) {
    super({
      ...config,
      type: SourceType.WEB,
      name: config.name || `Web: ${config.query}`,
    });

    this.query = config.query;
    this.domains = config.domains || []; // Allowed domains
    this.excludeDomains = config.excludeDomains || [];
  }

  /**
   * Fetch is a placeholder - needs external API
   */
  async fetch() {
    // This would need integration with a search API
    // For now, return empty with a note
    console.warn('WebSearchSource.fetch() requires external API integration');
    this.markFetched([]);
    return [];
  }

  toJSON() {
    return {
      ...super.toJSON(),
      query: this.query,
      domains: this.domains,
    };
  }
}

// =============================================================================
// ECOSYSTEM MONITOR
// =============================================================================

/**
 * Main ecosystem monitor - manages all sources
 */
export class EcosystemMonitor {
  constructor(config = {}) {
    this.sources = new Map();
    this.updateCache = [];
    this.maxCacheSize = config.maxCacheSize || 100;
    this.onUpdate = config.onUpdate || null;

    // Discovered sources (for source discovery feature)
    this.discoveredSources = [];

    // Stats
    this.stats = {
      totalFetches: 0,
      totalUpdates: 0,
      lastFullFetch: null,
      errors: 0,
    };
  }

  /**
   * Register a source
   */
  registerSource(source) {
    if (this.sources.size >= ECOSYSTEM_CONSTANTS.MAX_SOURCES) {
      throw new Error(`Max sources (${ECOSYSTEM_CONSTANTS.MAX_SOURCES}) reached`);
    }
    this.sources.set(source.id, source);
    return source.id;
  }

  /**
   * Register GitHub repo as source
   */
  trackGitHubRepo(owner, repo, options = {}) {
    const source = new GitHubSource({
      owner,
      repo,
      ...options,
    });
    return this.registerSource(source);
  }

  /**
   * Unregister a source
   */
  unregisterSource(sourceId) {
    return this.sources.delete(sourceId);
  }

  /**
   * Get source by ID
   */
  getSource(sourceId) {
    return this.sources.get(sourceId);
  }

  /**
   * List all sources
   */
  listSources() {
    return Array.from(this.sources.values()).map(s => s.toJSON());
  }

  /**
   * Fetch updates from a specific source
   */
  async fetchSource(sourceId) {
    const source = this.sources.get(sourceId);
    if (!source) {
      throw new Error(`Source not found: ${sourceId}`);
    }

    if (!source.canFetch()) {
      return { skipped: true, reason: 'rate_limited' };
    }

    try {
      const updates = await source.fetch();
      this.stats.totalFetches++;
      this.stats.totalUpdates += updates.length;

      // Add to cache
      this._addToCache(updates);

      // Callback
      if (this.onUpdate && updates.length > 0) {
        this.onUpdate(updates, source);
      }

      return { success: true, updates };

    } catch (error) {
      this.stats.errors++;
      return { success: false, error: error.message };
    }
  }

  /**
   * Fetch all sources that are ready
   */
  async fetchAll() {
    const results = {
      fetched: 0,
      skipped: 0,
      errors: 0,
      updates: [],
    };

    for (const [id, source] of this.sources) {
      if (!source.canFetch()) {
        results.skipped++;
        continue;
      }

      try {
        const updates = await source.fetch();
        results.fetched++;
        results.updates.push(...updates);
        this.stats.totalUpdates += updates.length;
      } catch (error) {
        results.errors++;
        this.stats.errors++;
      }
    }

    this.stats.totalFetches += results.fetched;
    this.stats.lastFullFetch = Date.now();

    // Add to cache
    this._addToCache(results.updates);

    // Callback
    if (this.onUpdate && results.updates.length > 0) {
      this.onUpdate(results.updates, null);
    }

    return results;
  }

  /**
   * Add updates to cache
   */
  _addToCache(updates) {
    this.updateCache.unshift(...updates);

    // Trim cache
    if (this.updateCache.length > this.maxCacheSize) {
      this.updateCache = this.updateCache.slice(0, this.maxCacheSize);
    }
  }

  /**
   * Get recent updates from cache
   */
  getRecentUpdates(options = {}) {
    let updates = [...this.updateCache];

    // Filter by type
    if (options.type) {
      updates = updates.filter(u => u.type === options.type);
    }

    // Filter by source
    if (options.sourceId) {
      updates = updates.filter(u => u.source === options.sourceId);
    }

    // Filter by priority
    if (options.minPriority) {
      const priorityOrder = [Priority.CRITICAL, Priority.HIGH, Priority.MEDIUM, Priority.LOW, Priority.INFO];
      const minIndex = priorityOrder.indexOf(options.minPriority);
      updates = updates.filter(u => {
        const index = priorityOrder.indexOf(u.priority);
        return index <= minIndex;
      });
    }

    // Filter by time
    if (options.since) {
      updates = updates.filter(u => u.timestamp >= options.since);
    }

    // Limit
    if (options.limit) {
      updates = updates.slice(0, options.limit);
    }

    return updates;
  }

  /**
   * Discover new relevant sources based on existing ones
   * Uses GitHub API to find related repos and analyzes patterns
   */
  async discoverSources(_context = {}) {
    const suggestions = [];
    const seenOrgs = new Set();
    const seenRepos = new Set();

    // Track what we already have
    for (const source of this.sources.values()) {
      if (source.type === SourceType.GITHUB) {
        seenOrgs.add(source.owner);
        seenRepos.add(`${source.owner}/${source.repo}`);
      }
    }

    // Strategy 1: Find more repos from tracked orgs
    for (const source of this.sources.values()) {
      if (source.type !== SourceType.GITHUB) continue;

      try {
        const headers = {
          'Accept': 'application/vnd.github.v3+json',
          'User-Agent': 'CYNIC-Ecosystem-Monitor',
        };
        if (source.token) {
          headers['Authorization'] = `token ${source.token}`;
        }

        // Get org repos
        const response = await fetch(
          `https://api.github.com/orgs/${source.owner}/repos?sort=updated&per_page=10`,
          { headers }
        );

        if (response.ok) {
          const repos = await response.json();
          for (const repo of repos) {
            const repoKey = `${source.owner}/${repo.name}`;
            if (!seenRepos.has(repoKey) && !repo.archived) {
              suggestions.push({
                type: 'github_repo',
                reason: `Same org as ${source.repo}`,
                source: {
                  owner: source.owner,
                  repo: repo.name,
                  description: repo.description,
                  stars: repo.stargazers_count,
                  updated: repo.updated_at,
                },
                confidence: Math.min(0.618, repo.stargazers_count / 1000),
              });
              seenRepos.add(repoKey);
            }
          }
        }
      } catch (_error) {
        // Silently skip - rate limits etc.
      }
    }

    // Strategy 2: Analyze recent updates for mentions
    const repoMentionPattern = /github\.com\/([a-zA-Z0-9_-]+)\/([a-zA-Z0-9_-]+)/g;

    for (const update of this.updateCache.slice(0, 50)) {
      const content = `${update.title || ''} ${update.description || ''}`;

      // Find GitHub repo mentions
      let match;
      while ((match = repoMentionPattern.exec(content)) !== null) {
        const [, owner, repo] = match;
        const repoKey = `${owner}/${repo}`;
        if (!seenRepos.has(repoKey) && !seenOrgs.has(owner)) {
          suggestions.push({
            type: 'github_mention',
            reason: `Mentioned in update: "${update.title?.slice(0, 50)}"`,
            source: { owner, repo },
            confidence: PHI_INV_2,
          });
          seenRepos.add(repoKey);
        }
      }
    }

    // Strategy 3: Known ecosystem relations (hardcoded for Solana)
    const knownRelations = {
      'solana-labs': ['solana-program-library', 'solana-pay', 'wallet-adapter'],
      'helius-labs': ['das-api', 'xray'],
      'coral-xyz': ['backpack', 'sealevel-tools'],
      'metaplex-foundation': ['js', 'mpl-token-metadata', 'sugar'],
      'jup-ag': ['jupiter-quote-api', 'jupiter-swap-api'],
    };

    for (const [org, repos] of Object.entries(knownRelations)) {
      if (seenOrgs.has(org)) {
        for (const repo of repos) {
          const repoKey = `${org}/${repo}`;
          if (!seenRepos.has(repoKey)) {
            suggestions.push({
              type: 'known_relation',
              reason: `Related to tracked ${org} repos`,
              source: { owner: org, repo },
              confidence: PHI_INV,
            });
            seenRepos.add(repoKey);
          }
        }
      }
    }

    // Sort by confidence
    suggestions.sort((a, b) => (b.confidence || 0) - (a.confidence || 0));

    // Limit to reasonable number
    this.discoveredSources = suggestions.slice(0, ECOSYSTEM_CONSTANTS.MAX_SOURCES);
    return this.discoveredSources;
  }

  /**
   * Get monitor status
   */
  getStatus() {
    return {
      sources: this.listSources(),
      stats: this.stats,
      cacheSize: this.updateCache.length,
      discoveredSources: this.discoveredSources.length,
    };
  }

  /**
   * Register default Solana ecosystem sources
   */
  registerSolanaDefaults() {
    // Solana core
    this.trackGitHubRepo('solana-labs', 'solana', {
      trackReleases: true,
      trackCommits: false, // Too many
    });

    // Web3.js
    this.trackGitHubRepo('solana-labs', 'solana-web3.js', {
      trackReleases: true,
      trackCommits: true,
    });

    // Helius
    this.trackGitHubRepo('helius-labs', 'helius-sdk', {
      trackReleases: true,
      trackCommits: true,
    });

    this.trackGitHubRepo('helius-labs', 'laserstream-sdk', {
      trackReleases: true,
      trackCommits: true,
    });

    // Anchor
    this.trackGitHubRepo('coral-xyz', 'anchor', {
      trackReleases: true,
      trackCommits: false,
    });

    return this.listSources();
  }
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/**
 * Create a pre-configured ecosystem monitor for Solana
 */
export function createSolanaMonitor(options = {}) {
  const monitor = new EcosystemMonitor(options);
  monitor.registerSolanaDefaults();
  return monitor;
}

/**
 * Extract update summary for reporting
 */
export function summarizeUpdates(updates) {
  const summary = {
    total: updates.length,
    byType: {},
    byPriority: {},
    bySource: {},
    highlights: [],
  };

  for (const update of updates) {
    // Count by type
    summary.byType[update.type] = (summary.byType[update.type] || 0) + 1;

    // Count by priority
    summary.byPriority[update.priority] = (summary.byPriority[update.priority] || 0) + 1;

    // Count by source
    summary.bySource[update.source] = (summary.bySource[update.source] || 0) + 1;

    // Collect highlights (CRITICAL or HIGH priority)
    if (update.priority === Priority.CRITICAL || update.priority === Priority.HIGH) {
      summary.highlights.push({
        type: update.type,
        title: update.title,
        url: update.url,
        priority: update.priority,
      });
    }
  }

  return summary;
}

// =============================================================================
// E-SCORE INTEGRATION
// =============================================================================

/**
 * Culture signals derived from ecosystem updates
 * Maps update patterns to E-Score dimensions
 */
export const CultureSignals = {
  DEVELOPMENT_VELOCITY: 'development_velocity',    // How fast they ship
  COMMUNITY_ENGAGEMENT: 'community_engagement',    // Issues, PRs, discussions
  TRANSPARENCY: 'transparency',                    // Release notes, docs
  SECURITY_FOCUS: 'security_focus',               // Security-related updates
  INNOVATION: 'innovation',                       // New features vs fixes
  MAINTENANCE: 'maintenance',                     // Bug fixes, deps updates
};

/**
 * Calculate E-Score signals from ecosystem updates
 * @param {Array} updates - Updates from ecosystem monitor
 * @param {Object} options - Calculation options
 * @returns {Object} E-Score culture signals
 */
export function calculateEScoreSignals(updates, options = {}) {
  const { timeWindowMs = 7 * 24 * 60 * 60 * 1000 } = options; // 7 days default
  const now = Date.now();
  const cutoff = now - timeWindowMs;

  // Filter to time window
  const recentUpdates = updates.filter(u => u.timestamp >= cutoff);

  if (recentUpdates.length === 0) {
    return {
      signals: {},
      confidence: 0,
      message: 'No recent updates to analyze',
    };
  }

  // Calculate signals
  const signals = {};

  // 1. Development Velocity (commits + releases per day)
  const commits = recentUpdates.filter(u => u.type === UpdateType.COMMIT).length;
  const releases = recentUpdates.filter(u => u.type === UpdateType.RELEASE).length;
  const days = timeWindowMs / (24 * 60 * 60 * 1000);
  const velocity = (commits + releases * 5) / days; // Releases weighted 5x
  signals[CultureSignals.DEVELOPMENT_VELOCITY] = Math.min(1, velocity / 10); // Normalize to 0-1

  // 2. Community Engagement (issues, PRs)
  const issues = recentUpdates.filter(u => u.type === UpdateType.ISSUE).length;
  const prs = recentUpdates.filter(u => u.type === UpdateType.PR).length;
  signals[CultureSignals.COMMUNITY_ENGAGEMENT] = Math.min(1, (issues + prs) / 20);

  // 3. Transparency (releases with descriptions)
  const releasesWithDesc = recentUpdates.filter(
    u => u.type === UpdateType.RELEASE && u.description && u.description.length > 50
  ).length;
  signals[CultureSignals.TRANSPARENCY] = releases > 0 ? releasesWithDesc / releases : 0;

  // 4. Security Focus (security-related updates)
  const securityUpdates = recentUpdates.filter(u => {
    const text = `${u.title || ''} ${u.description || ''}`.toLowerCase();
    return text.includes('security') || text.includes('vulnerability') ||
           text.includes('cve') || u.priority === Priority.CRITICAL;
  }).length;
  signals[CultureSignals.SECURITY_FOCUS] = securityUpdates > 0 ? 1 : 0.5; // Bonus if found

  // 5. Innovation vs Maintenance ratio
  const features = recentUpdates.filter(u => {
    const text = `${u.title || ''}`.toLowerCase();
    return text.includes('feat') || text.includes('add') || text.includes('new');
  }).length;
  const fixes = recentUpdates.filter(u => {
    const text = `${u.title || ''}`.toLowerCase();
    return text.includes('fix') || text.includes('bug') || text.includes('patch');
  }).length;
  const total = features + fixes;
  signals[CultureSignals.INNOVATION] = total > 0 ? features / total : 0.5;
  signals[CultureSignals.MAINTENANCE] = total > 0 ? fixes / total : 0.5;

  // Overall confidence based on sample size (max φ⁻¹)
  const confidence = Math.min(PHI_INV, recentUpdates.length / 50);

  // Composite E-Score contribution (weighted average)
  const weights = {
    [CultureSignals.DEVELOPMENT_VELOCITY]: 0.25,
    [CultureSignals.COMMUNITY_ENGAGEMENT]: 0.20,
    [CultureSignals.TRANSPARENCY]: 0.15,
    [CultureSignals.SECURITY_FOCUS]: 0.15,
    [CultureSignals.INNOVATION]: 0.15,
    [CultureSignals.MAINTENANCE]: 0.10,
  };

  let composite = 0;
  for (const [signal, weight] of Object.entries(weights)) {
    composite += (signals[signal] || 0) * weight;
  }

  return {
    signals,
    composite,
    confidence,
    sampleSize: recentUpdates.length,
    timeWindowDays: days,
    breakdown: {
      commits,
      releases,
      issues,
      prs,
      features,
      fixes,
      securityUpdates,
    },
  };
}

/**
 * Add E-Score calculation method to EcosystemMonitor
 */
EcosystemMonitor.prototype.calculateEScore = function(options = {}) {
  return calculateEScoreSignals(this.updateCache, options);
};

/**
 * Get ecosystem health report combining updates and E-Score
 */
EcosystemMonitor.prototype.getHealthReport = function() {
  const summary = summarizeUpdates(this.updateCache);
  const eScore = this.calculateEScore();

  return {
    timestamp: Date.now(),
    sources: this.listSources(),
    updateSummary: summary,
    eScore: eScore,
    discoveredSources: this.discoveredSources.length,
    health: eScore.composite >= 0.5 ? 'healthy' : 'needs_attention',
    verdict: eScore.composite >= PHI_INV ? 'HOWL' :
             eScore.composite >= 0.382 ? 'WAG' :
             eScore.composite >= 0.236 ? 'BARK' : 'GROWL',
  };
};
