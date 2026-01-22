#!/usr/bin/env node
/**
 * CYNIC Contributor Discovery - Auto-learn from everyone
 *
 * "Le chien connaît tous les humains" - The dog knows all humans
 *
 * Automatically discovers, analyzes, and learns from ALL contributors
 * across the entire ecosystem. Builds rich profiles for personalized
 * interaction and pattern detection.
 *
 * This is the RAIL in CYNIC's brain for human understanding.
 *
 * Security Note: Uses execSync with controlled inputs only (git config values,
 * filesystem paths). No user-provided input is passed to shell commands.
 *
 * @module cynic/lib/contributor-discovery
 */

'use strict';

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// =============================================================================
// φ CONSTANTS
// =============================================================================

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;

// =============================================================================
// PATHS
// =============================================================================

const CYNIC_DIR = path.join(process.env.HOME || '/root', '.cynic');
const LEARNING_DIR = path.join(CYNIC_DIR, 'learning');
const CONTRIBUTORS_FILE = path.join(LEARNING_DIR, 'contributors.json');
const PROFILES_DIR = path.join(LEARNING_DIR, 'profiles');

function ensureDirs() {
  for (const dir of [CYNIC_DIR, LEARNING_DIR, PROFILES_DIR]) {
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
  }
}

// =============================================================================
// GIT HELPERS
// =============================================================================

/**
 * Run a git command safely
 * Note: Only use with controlled inputs, never with user-provided data
 */
function git(cmd, cwd = process.cwd()) {
  try {
    return execSync(`git ${cmd}`, {
      cwd,
      encoding: 'utf-8',
      maxBuffer: 50 * 1024 * 1024,
      stdio: ['pipe', 'pipe', 'pipe']
    }).trim();
  } catch (e) {
    return '';
  }
}

function isGitRepo(dir) {
  return fs.existsSync(path.join(dir, '.git'));
}

// =============================================================================
// DISCOVERY ENGINE
// =============================================================================

/**
 * Discover all repositories in a workspace
 */
function discoverRepos(workspacePath = '/workspaces') {
  if (!fs.existsSync(workspacePath)) return [];

  return fs.readdirSync(workspacePath)
    .map(name => path.join(workspacePath, name))
    .filter(p => fs.statSync(p).isDirectory() && isGitRepo(p));
}

/**
 * Discover all contributors in a repository
 */
function discoverContributors(repoPath) {
  const output = git('log --all --format="%an|%ae" | sort | uniq -c | sort -rn', repoPath);
  if (!output) return [];

  return output.split('\n')
    .filter(line => line.trim())
    .map(line => {
      const match = line.trim().match(/^\s*(\d+)\s+(.+)\|(.+)$/);
      if (!match) return null;
      return {
        commits: parseInt(match[1]),
        name: match[2],
        email: match[3],
      };
    })
    .filter(Boolean);
}

/**
 * Full ecosystem scan - discover everyone
 */
function fullEcosystemScan(workspacePath = '/workspaces') {
  ensureDirs();

  const repos = discoverRepos(workspacePath);
  const allContributors = {};
  const repoStats = {};

  for (const repoPath of repos) {
    const repoName = path.basename(repoPath);
    const contributors = discoverContributors(repoPath);
    const commitCount = parseInt(git('rev-list --all --count', repoPath)) || 0;

    repoStats[repoName] = {
      path: repoPath,
      commits: commitCount,
      contributors: contributors.length,
    };

    for (const c of contributors) {
      const key = c.email.toLowerCase();
      if (!allContributors[key]) {
        allContributors[key] = {
          email: c.email,
          names: new Set(),
          repos: {},
          totalCommits: 0,
          discoveredAt: new Date().toISOString(),
        };
      }
      allContributors[key].names.add(c.name);
      allContributors[key].repos[repoName] = c.commits;
      allContributors[key].totalCommits += c.commits;
    }
  }

  // Convert Sets to arrays for JSON
  for (const c of Object.values(allContributors)) {
    c.names = [...c.names];
    c.primaryName = c.names[0];
  }

  return { repos: repoStats, contributors: allContributors };
}

// =============================================================================
// PROFILE BUILDER
// =============================================================================

/**
 * Build detailed profile for a contributor
 */
function buildContributorProfile(email, workspacePath = '/workspaces') {
  const repos = discoverRepos(workspacePath);
  const profile = {
    email,
    names: new Set(),
    analyzedAt: new Date().toISOString(),

    // Aggregate stats
    totalCommits: 0,
    totalLinesAdded: 0,
    totalLinesDeleted: 0,
    repoContributions: {},

    // Timing patterns
    hourDistribution: Array(24).fill(0),
    dayDistribution: Array(7).fill(0),

    // Work patterns
    commitTypes: {},
    scopes: {},
    fileTypes: {},

    // Session analysis
    sessions: [],

    // All commits for deep analysis
    commits: [],
  };

  // Sanitize email for git command (basic validation)
  const safeEmail = email.replace(/[^a-zA-Z0-9@._+-]/g, '');

  for (const repoPath of repos) {
    const repoName = path.basename(repoPath);

    // Get commits - using sanitized email
    const commitData = git(
      `log --all --author="${safeEmail}" --format="%H|%at|%s"`,
      repoPath
    );

    if (!commitData) continue;

    const commits = commitData.split('\n')
      .filter(l => l)
      .map(line => {
        const parts = line.split('|');
        const hash = parts[0];
        const ts = parts[1];
        const msg = parts.slice(2).join('|'); // Message might contain |
        return {
          hash,
          timestamp: parseInt(ts) * 1000,
          message: msg || '',
          repo: repoName
        };
      });

    if (commits.length === 0) continue;

    profile.commits.push(...commits);
    profile.repoContributions[repoName] = commits.length;
    profile.totalCommits += commits.length;

    // Get contributor name
    const nameOutput = git(`log --all --author="${safeEmail}" --format="%an" | head -1`, repoPath);
    if (nameOutput) profile.names.add(nameOutput);

    // Get line stats
    const statsOutput = git(
      `log --all --author="${safeEmail}" --shortstat --format=""`,
      repoPath
    );

    for (const line of statsOutput.split('\n')) {
      const insMatch = line.match(/(\d+) insertion/);
      const delMatch = line.match(/(\d+) deletion/);
      if (insMatch) profile.totalLinesAdded += parseInt(insMatch[1]);
      if (delMatch) profile.totalLinesDeleted += parseInt(delMatch[1]);
    }

    // Get file types
    const filesOutput = git(
      `log --all --author="${safeEmail}" --name-only --format=""`,
      repoPath
    );

    for (const file of filesOutput.split('\n').filter(f => f.trim())) {
      const ext = path.extname(file) || 'no-ext';
      profile.fileTypes[ext] = (profile.fileTypes[ext] || 0) + 1;
    }
  }

  // Analyze all commits
  for (const commit of profile.commits) {
    // Hour/day distribution
    const date = new Date(commit.timestamp);
    profile.hourDistribution[date.getUTCHours()]++;
    profile.dayDistribution[date.getUTCDay()]++;

    // Commit type
    const typeMatch = commit.message.match(/^(\w+)(\([^)]+\))?[:\s]/);
    const type = typeMatch ? typeMatch[1] : 'other';
    profile.commitTypes[type] = (profile.commitTypes[type] || 0) + 1;

    // Scope
    const scopeMatch = commit.message.match(/\(([^)]+)\)/);
    const scope = scopeMatch ? scopeMatch[1] : 'global';
    profile.scopes[scope] = (profile.scopes[scope] || 0) + 1;
  }

  // Session detection (gaps > 2 hours)
  if (profile.commits.length > 0) {
    const sorted = [...profile.commits].sort((a, b) => a.timestamp - b.timestamp);
    let session = { start: sorted[0].timestamp, commits: 1 };

    for (let i = 1; i < sorted.length; i++) {
      const gap = sorted[i].timestamp - sorted[i-1].timestamp;
      if (gap > 2 * 60 * 60 * 1000) {
        session.end = sorted[i-1].timestamp;
        session.duration = (session.end - session.start) / (1000 * 60 * 60);
        profile.sessions.push(session);
        session = { start: sorted[i].timestamp, commits: 1 };
      } else {
        session.commits++;
      }
    }
    session.end = sorted[sorted.length - 1].timestamp;
    session.duration = (session.end - session.start) / (1000 * 60 * 60);
    profile.sessions.push(session);
  }

  // Derive insights
  profile.names = [...profile.names];
  profile.primaryName = profile.names[0] || email.split('@')[0];

  profile.insights = deriveInsights(profile);

  // Remove raw commits to save space (keep count)
  profile.commitCount = profile.commits.length;
  delete profile.commits;

  return profile;
}

/**
 * Derive insights from profile data
 */
function deriveInsights(profile) {
  const insights = {};

  // Peak hours
  const hourPeaks = profile.hourDistribution
    .map((count, hour) => ({ hour, count }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 3);
  insights.peakHours = hourPeaks;

  // Work style
  const nightCommits = profile.hourDistribution.slice(22, 24).reduce((a, b) => a + b, 0) +
                       profile.hourDistribution.slice(0, 6).reduce((a, b) => a + b, 0);
  const dayCommits = profile.hourDistribution.slice(9, 18).reduce((a, b) => a + b, 0);
  insights.workStyle = nightCommits > dayCommits * 0.5 ? 'night-owl' : 'day-worker';

  // Timezone inference
  const peakHour = hourPeaks[0]?.hour || 12;
  if (peakHour >= 7 && peakHour <= 18) {
    insights.inferredTimezone = 'Europe (CET/CEST)';
  } else if (peakHour >= 13 && peakHour <= 23) {
    insights.inferredTimezone = 'Americas (EST/PST)';
  } else {
    insights.inferredTimezone = 'Asia/Pacific or night worker';
  }

  // Personality traits
  const feat = profile.commitTypes.feat || 0;
  const fix = profile.commitTypes.fix || 0;
  const test = profile.commitTypes.test || 0;
  const docs = profile.commitTypes.docs || 0;
  const debug = profile.commitTypes.debug || 0;
  const total = profile.totalCommits || 1;

  insights.personality = {
    builder: feat / total > 0.4 ? 'high' : feat / total > 0.2 ? 'medium' : 'low',
    fixer: fix / total > 0.3 ? 'high' : fix / total > 0.15 ? 'medium' : 'low',
    tester: test / total > 0.1 ? 'high' : test / total > 0.05 ? 'medium' : 'low',
    documenter: docs / total > 0.05 ? 'high' : docs > 0 ? 'medium' : 'low',
    debugger: debug / total > 0.1 ? 'high' : debug > 0 ? 'medium' : 'low',
  };

  // Velocity metrics
  if (profile.sessions.length > 0) {
    const avgCommitsPerSession = profile.totalCommits / profile.sessions.length;
    const avgSessionDuration = profile.sessions.reduce((sum, s) => sum + s.duration, 0) / profile.sessions.length;

    insights.velocity = {
      avgCommitsPerSession: Math.round(avgCommitsPerSession * 10) / 10,
      avgSessionHours: Math.round(avgSessionDuration * 10) / 10,
      totalSessions: profile.sessions.length,
      flowSessions: profile.sessions.filter(s => s.commits >= 10).length,
    };
  }

  // Focus areas
  const topScopes = Object.entries(profile.scopes)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5);
  insights.focusAreas = topScopes.map(([scope, count]) => ({
    area: scope,
    commits: count,
    percent: Math.round(count / total * 100),
  }));

  // φ-scores
  insights.phiScores = {
    velocity: Math.min(100, Math.round((profile.totalCommits / Math.max(1, profile.sessions.length)) * PHI_INV * 10)),
    depth: Math.min(100, Math.round(Math.log(Math.max(1, profile.totalLinesAdded)) / Math.log(1000000) * 100)),
    breadth: Math.min(100, Math.round(Object.keys(profile.repoContributions).length * 20)),
  };

  return insights;
}

// =============================================================================
// STORAGE
// =============================================================================

/**
 * Load all known contributors
 */
function loadContributors() {
  ensureDirs();
  if (fs.existsSync(CONTRIBUTORS_FILE)) {
    try {
      return JSON.parse(fs.readFileSync(CONTRIBUTORS_FILE, 'utf-8'));
    } catch (e) {
      return { contributors: {}, repos: {}, lastScan: null };
    }
  }
  return { contributors: {}, repos: {}, lastScan: null };
}

/**
 * Save contributors data
 */
function saveContributors(data) {
  ensureDirs();
  data.lastScan = new Date().toISOString();
  fs.writeFileSync(CONTRIBUTORS_FILE, JSON.stringify(data, null, 2));
}

/**
 * Load a contributor's profile
 */
function loadProfile(email) {
  ensureDirs();
  const safeName = email.replace(/[^a-zA-Z0-9]/g, '_');
  const profilePath = path.join(PROFILES_DIR, `${safeName}.json`);

  if (fs.existsSync(profilePath)) {
    try {
      return JSON.parse(fs.readFileSync(profilePath, 'utf-8'));
    } catch (e) {
      return null;
    }
  }
  return null;
}

/**
 * Save a contributor's profile
 */
function saveProfile(profile) {
  ensureDirs();
  const safeName = profile.email.replace(/[^a-zA-Z0-9]/g, '_');
  const profilePath = path.join(PROFILES_DIR, `${safeName}.json`);
  fs.writeFileSync(profilePath, JSON.stringify(profile, null, 2));
}

// =============================================================================
// MAIN API
// =============================================================================

/**
 * Run full discovery and update all profiles
 */
function discover(workspacePath = '/workspaces') {
  // Scan ecosystem
  const scan = fullEcosystemScan(workspacePath);

  // Load existing data
  const existing = loadContributors();

  // Merge new data
  for (const [email, data] of Object.entries(scan.contributors)) {
    if (!existing.contributors[email]) {
      existing.contributors[email] = data;
    } else {
      // Update existing
      existing.contributors[email].repos = {
        ...existing.contributors[email].repos,
        ...data.repos,
      };
      existing.contributors[email].totalCommits = data.totalCommits;
      existing.contributors[email].names = [...new Set([
        ...existing.contributors[email].names,
        ...data.names,
      ])];
    }
  }

  existing.repos = scan.repos;
  saveContributors(existing);

  return existing;
}

/**
 * Get or build profile for a contributor
 */
function getProfile(email, forceRebuild = false) {
  let profile = loadProfile(email);

  // Rebuild if stale (> 1 hour) or forced
  const isStale = profile &&
    (Date.now() - new Date(profile.analyzedAt).getTime() > 60 * 60 * 1000);

  if (!profile || forceRebuild || isStale) {
    profile = buildContributorProfile(email);
    saveProfile(profile);
  }

  return profile;
}

/**
 * Get profile for current git user
 */
function getCurrentUserProfile() {
  const email = git('config user.email');
  if (!email) return null;
  return getProfile(email);
}

/**
 * Format profile for display
 */
function formatProfile(profile) {
  const p = profile;
  const i = p.insights || {};

  return `
╔══════════════════════════════════════════════════════════════╗
║  ${(p.primaryName || p.email).padEnd(58)}║
╠══════════════════════════════════════════════════════════════╣
║  Email: ${p.email.slice(0, 52).padEnd(52)}║
║  Repos: ${Object.keys(p.repoContributions || {}).length.toString().padEnd(52)}║
║  Commits: ${(p.totalCommits || 0).toString().padEnd(50)}║
║  Lines: +${(p.totalLinesAdded || 0).toLocaleString()} / -${(p.totalLinesDeleted || 0).toLocaleString()}${' '.repeat(Math.max(0, 38 - (p.totalLinesAdded || 0).toLocaleString().length - (p.totalLinesDeleted || 0).toLocaleString().length))}║
╠══════════════════════════════════════════════════════════════╣
║  WORK STYLE                                                  ║
║  • ${(i.workStyle || 'unknown').padEnd(56)}║
║  • Timezone: ${(i.inferredTimezone || 'unknown').padEnd(46)}║
║  • Peak hours: ${(i.peakHours || []).map(h => h.hour + 'h').join(', ').padEnd(44)}║
╠══════════════════════════════════════════════════════════════╣
║  PERSONALITY                                                 ║
║  • Builder: ${(i.personality?.builder || '-').padEnd(47)}║
║  • Fixer: ${(i.personality?.fixer || '-').padEnd(49)}║
║  • Tester: ${(i.personality?.tester || '-').padEnd(48)}║
║  • Documenter: ${(i.personality?.documenter || '-').padEnd(44)}║
╠══════════════════════════════════════════════════════════════╣
║  φ-SCORES                                                    ║
║  • Velocity: ${(i.phiScores?.velocity || 0).toString().padEnd(46)}║
║  • Depth: ${(i.phiScores?.depth || 0).toString().padEnd(49)}║
║  • Breadth: ${(i.phiScores?.breadth || 0).toString().padEnd(47)}║
╚══════════════════════════════════════════════════════════════╝
`.trim();
}

// =============================================================================
// EXPORTS
// =============================================================================

module.exports = {
  // Discovery
  discover,
  discoverRepos,
  discoverContributors,
  fullEcosystemScan,

  // Profiles
  getProfile,
  getCurrentUserProfile,
  buildContributorProfile,

  // Storage
  loadContributors,
  saveContributors,
  loadProfile,
  saveProfile,

  // Display
  formatProfile,

  // Paths
  CONTRIBUTORS_FILE,
  PROFILES_DIR,
};
