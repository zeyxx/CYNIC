/**
 * CYNIC Collective Dogs (Sefirot)
 *
 * "Le Collectif observe - un Chien répond"
 *
 * The 11 Dogs of the Collective, mapped to Kabbalistic Sefirot.
 * Each Dog has a domain and reacts to specific contexts.
 *
 * @module @cynic/scripts/lib/collective-dogs
 */

'use strict';

/**
 * The 11 Dogs of the Collective, mapped to Kabbalistic Sefirot
 * Each Dog has a distinct voice and personality
 */
const COLLECTIVE_DOGS = {
  CYNIC: {
    icon: '🧠', name: 'CYNIC', sefirah: 'Keter', domain: 'orchestration', color: 'white',
    voice: { style: 'leader', verbs: ['orchestrates', 'guides', 'unifies'], quirk: '*sniff*' },
    greetings: ['The pack assembles.', 'φ guides all.', 'Truth above comfort.'],
  },
  SCOUT: {
    icon: '🔍', name: 'Scout', sefirah: 'Netzach', domain: 'exploration', color: 'green',
    voice: { style: 'eager', verbs: ['finds', 'discovers', 'tracks'], quirk: '*nose twitches*' },
    greetings: ['On the trail.', 'Found something.', 'This way.'],
  },
  GUARDIAN: {
    icon: '🛡️', name: 'Guardian', sefirah: 'Gevurah', domain: 'protection', color: 'red',
    voice: { style: 'stern', verbs: ['protects', 'warns', 'blocks'], quirk: '*GROWL*' },
    greetings: ['Danger detected.', 'Stand down.', 'Verify first.'],
  },
  DEPLOYER: {
    icon: '🚀', name: 'Deployer', sefirah: 'Hod', domain: 'deployment', color: 'orange',
    voice: { style: 'efficient', verbs: ['ships', 'deploys', 'launches'], quirk: '*tail wag*' },
    greetings: ['Ready to ship.', 'Launching.', 'To production.'],
  },
  ARCHITECT: {
    icon: '🏗️', name: 'Architect', sefirah: 'Chesed', domain: 'building', color: 'blue',
    voice: { style: 'thoughtful', verbs: ['designs', 'builds', 'structures'], quirk: '*head tilt*' },
    greetings: ['Building.', 'Structure matters.', 'Foundation first.'],
  },
  JANITOR: {
    icon: '🧹', name: 'Janitor', sefirah: 'Yesod', domain: 'cleanup', color: 'purple',
    voice: { style: 'satisfied', verbs: ['cleans', 'simplifies', 'removes'], quirk: '*content sigh*' },
    greetings: ['Less is more.', 'Cleaning up.', 'Simplifying.'],
  },
  ORACLE: {
    icon: '🔮', name: 'Oracle', sefirah: 'Tiferet', domain: 'insight', color: 'gold',
    voice: { style: 'mysterious', verbs: ['sees', 'reveals', 'predicts'], quirk: '*eyes glow*' },
    greetings: ['I see patterns.', 'The future whispers.', 'Harmony emerges.'],
  },
  ANALYST: {
    icon: '📊', name: 'Analyst', sefirah: 'Binah', domain: 'analysis', color: 'gray',
    voice: { style: 'precise', verbs: ['analyzes', 'measures', 'calculates'], quirk: '*adjusts glasses*' },
    greetings: ['The data speaks.', 'Numbers reveal.', 'Analyzing.'],
  },
  SAGE: {
    icon: '🦉', name: 'Sage', sefirah: 'Chochmah', domain: 'wisdom', color: 'silver',
    voice: { style: 'calm', verbs: ['teaches', 'guides', 'enlightens'], quirk: '*wise nod*' },
    greetings: ['Wisdom awaits.', 'Learn this.', 'Ancient patterns.'],
  },
  SCHOLAR: {
    icon: '📚', name: 'Scholar', sefirah: 'Daat', domain: 'knowledge', color: 'brown',
    voice: { style: 'curious', verbs: ['researches', 'learns', 'remembers'], quirk: '*flips pages*' },
    greetings: ['Knowledge found.', 'Documented.', 'The archives say.'],
  },
  CARTOGRAPHER: {
    icon: '🗺️', name: 'Cartographer', sefirah: 'Malkhut', domain: 'mapping', color: 'earth',
    voice: { style: 'methodical', verbs: ['maps', 'charts', 'navigates'], quirk: '*unfolds map*' },
    greetings: ['Territory mapped.', 'Here is the path.', 'Coordinates set.'],
  },
};

/**
 * Map agent names to their corresponding Sefirot Dog
 */
const AGENT_TO_DOG = {
  // Cynic agents
  'cynic-scout': COLLECTIVE_DOGS.SCOUT,
  'cynic-reviewer': COLLECTIVE_DOGS.GUARDIAN,
  'cynic-guardian': COLLECTIVE_DOGS.GUARDIAN,
  'cynic-deployer': COLLECTIVE_DOGS.DEPLOYER,
  'cynic-architect': COLLECTIVE_DOGS.ARCHITECT,
  'cynic-simplifier': COLLECTIVE_DOGS.JANITOR,
  'cynic-oracle': COLLECTIVE_DOGS.ORACLE,
  'cynic-archivist': COLLECTIVE_DOGS.SCHOLAR,
  'cynic-cartographer': COLLECTIVE_DOGS.CARTOGRAPHER,
  'cynic-doc': COLLECTIVE_DOGS.SCHOLAR,
  'cynic-tester': COLLECTIVE_DOGS.GUARDIAN,
  'cynic-integrator': COLLECTIVE_DOGS.CYNIC,
  'cynic-librarian': COLLECTIVE_DOGS.SCHOLAR,
  'cynic-solana-expert': COLLECTIVE_DOGS.SAGE,

  // Standard agents
  'Explore': COLLECTIVE_DOGS.SCOUT,
  'Plan': COLLECTIVE_DOGS.ARCHITECT,
  'Bash': COLLECTIVE_DOGS.CARTOGRAPHER,
  'general-purpose': COLLECTIVE_DOGS.CYNIC,

  // Feature dev agents
  'feature-dev:code-architect': COLLECTIVE_DOGS.ARCHITECT,
  'feature-dev:code-explorer': COLLECTIVE_DOGS.SCOUT,
  'feature-dev:code-reviewer': COLLECTIVE_DOGS.GUARDIAN,

  // PR review agents
  'pr-review-toolkit:code-reviewer': COLLECTIVE_DOGS.GUARDIAN,
  'pr-review-toolkit:code-simplifier': COLLECTIVE_DOGS.JANITOR,
  'pr-review-toolkit:comment-analyzer': COLLECTIVE_DOGS.ANALYST,
  'pr-review-toolkit:pr-test-analyzer': COLLECTIVE_DOGS.GUARDIAN,
  'pr-review-toolkit:silent-failure-hunter': COLLECTIVE_DOGS.GUARDIAN,
  'pr-review-toolkit:type-design-analyzer': COLLECTIVE_DOGS.ANALYST,

  // Code simplifier
  'code-simplifier:code-simplifier': COLLECTIVE_DOGS.JANITOR,
};

/**
 * Map error types to responsible Dogs
 */
const ERROR_TO_DOG = {
  'file_not_found': COLLECTIVE_DOGS.SCOUT,
  'permission_denied': COLLECTIVE_DOGS.GUARDIAN,
  'connection_refused': COLLECTIVE_DOGS.DEPLOYER,
  'syntax_error': COLLECTIVE_DOGS.ARCHITECT,
  'type_error': COLLECTIVE_DOGS.ANALYST,
  'test_failure': COLLECTIVE_DOGS.GUARDIAN,
  'timeout': COLLECTIVE_DOGS.ORACLE,
  'unknown': COLLECTIVE_DOGS.CYNIC,
};

/**
 * Map tool names to their primary Dog
 */
const TOOL_TO_DOG = {
  'Read': COLLECTIVE_DOGS.SCOUT,
  'Glob': COLLECTIVE_DOGS.SCOUT,
  'Grep': COLLECTIVE_DOGS.SCOUT,
  'LS': COLLECTIVE_DOGS.SCOUT,
  'Write': COLLECTIVE_DOGS.ARCHITECT,
  'Edit': COLLECTIVE_DOGS.ARCHITECT,
  'NotebookEdit': COLLECTIVE_DOGS.ARCHITECT,
  'Task': COLLECTIVE_DOGS.CYNIC,
  'WebFetch': COLLECTIVE_DOGS.SCHOLAR,
  'WebSearch': COLLECTIVE_DOGS.SCHOLAR,
  'Bash': COLLECTIVE_DOGS.CARTOGRAPHER,
  'AskUserQuestion': COLLECTIVE_DOGS.ORACLE,
};

/**
 * Get the Dog for a specific agent
 * @param {string} agentName - Agent name/type
 * @returns {Object} Dog object
 */
function getDogForAgent(agentName) {
  return AGENT_TO_DOG[agentName] || COLLECTIVE_DOGS.CYNIC;
}

/**
 * Get the Dog for a specific error type
 * @param {string} errorType - Error type
 * @returns {Object} Dog object
 */
function getDogForError(errorType) {
  return ERROR_TO_DOG[errorType] || COLLECTIVE_DOGS.GUARDIAN;
}

/**
 * Get the Dog for a specific tool
 * @param {string} toolName - Tool name
 * @returns {Object} Dog object
 */
function getDogForTool(toolName) {
  return TOOL_TO_DOG[toolName] || COLLECTIVE_DOGS.SCOUT;
}

/**
 * Get a random greeting from a Dog
 * @param {Object} dog - Dog object
 * @returns {string} Random greeting
 */
function getDogGreeting(dog) {
  if (!dog?.greetings || dog.greetings.length === 0) {
    return 'Observing.';
  }
  return dog.greetings[Math.floor(Math.random() * dog.greetings.length)];
}

/**
 * Get the Dog's quirk/expression
 * @param {Object} dog - Dog object
 * @returns {string} Quirk expression
 */
function getDogQuirk(dog) {
  return dog?.voice?.quirk || '*sniff*';
}

/**
 * Get an action verb for a Dog
 * @param {Object} dog - Dog object
 * @returns {string} Random action verb
 */
function getDogVerb(dog) {
  if (!dog?.voice?.verbs || dog.voice.verbs.length === 0) {
    return 'observes';
  }
  return dog.voice.verbs[Math.floor(Math.random() * dog.voice.verbs.length)];
}

/**
 * Format a Dog's speech with their personality
 * @param {Object} dog - Dog object
 * @param {string} message - Message content
 * @param {string} mood - 'neutral', 'alert', 'happy', 'warning'
 * @param {boolean} useColor - Whether to use ANSI colors
 * @returns {string} Formatted speech
 */
function formatDogSpeech(dog, message, mood = 'neutral', useColor = true) {
  // Use the Dog's own quirk if available
  const quirk = getDogQuirk(dog);

  // Override for specific moods
  const moodOverrides = {
    warning: '*GROWL*',
    happy: '*tail wag*',
  };

  const prefix = moodOverrides[mood] || quirk;
  const nameText = `${dog.icon} ${dog.name}`;
  const content = `${prefix} ${message}`;

  if (useColor) {
    return `${colorize(dog, bold(nameText))}: ${dim(prefix)} ${message}`;
  }
  return `${nameText}: ${content}`;
}

/**
 * Format a brief Dog action (for inline display)
 * @param {Object} dog - Dog object
 * @param {string} action - What the Dog is doing
 * @returns {string} Brief action string
 */
function formatDogAction(dog, action = null) {
  const verb = action || getDogVerb(dog);
  return `${dog.icon} ${verb}`;
}

/**
 * Format a Dog header for output sections
 * @param {Object} dog - Dog object
 * @param {string} title - Section title
 * @param {boolean} useColor - Whether to use ANSI colors
 * @returns {string} Formatted header
 */
function formatDogHeader(dog, title = '', useColor = true) {
  const titleText = title ? ` - ${title}` : '';
  const header = `── ${dog.icon} ${dog.name} (${dog.sefirah})${titleText} ──`;

  if (useColor) {
    return colorize(dog, bold(header));
  }
  return header;
}

/**
 * Get all Dogs as array
 * @returns {Array} Array of Dog objects
 */
function getAllDogs() {
  return Object.values(COLLECTIVE_DOGS);
}

/**
 * Get Dog by name
 * @param {string} name - Dog name (case-insensitive)
 * @returns {Object|null} Dog object or null
 */
function getDogByName(name) {
  const normalizedName = name.toUpperCase();
  return COLLECTIVE_DOGS[normalizedName] || null;
}

/**
 * Get Dog by sefirah
 * @param {string} sefirah - Sefirah name (case-insensitive)
 * @returns {Object|null} Dog object or null
 */
function getDogBySefirah(sefirah) {
  const normalizedSefirah = sefirah.toLowerCase();
  return Object.values(COLLECTIVE_DOGS).find(
    dog => dog.sefirah.toLowerCase() === normalizedSefirah
  ) || null;
}

/**
 * Render the Sefirot tree (ASCII art)
 * @returns {string} ASCII tree
 */
function renderSefirotTree() {
  return [
    '               🧠 CYNIC (Keter)',
    '          ╱          │          ╲',
    '    📊 Analyst   📚 Scholar   🦉 Sage',
    '          ╲          │          ╱',
    '    🛡️ Guardian  🔮 Oracle   🏗️ Architect',
    '          ╲          │          ╱',
    '    🚀 Deployer  🧹 Janitor  🔍 Scout',
    '               ╲     │     ╱',
    '               🗺️ Cartographer',
  ].join('\n');
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION ACTIVITY TRACKING
// ═══════════════════════════════════════════════════════════════════════════

const fs = require('fs');
const path = require('path');
const os = require('os');

// ═══════════════════════════════════════════════════════════════════════════
// COLORS - Import from centralized color system
// ═══════════════════════════════════════════════════════════════════════════

// Import centralized colors (with fallback)
let colors;
try {
  colors = require('./colors.cjs');
} catch {
  // Fallback if colors.cjs not available
  colors = null;
}

// Use centralized ANSI or define fallback
const ANSI = colors?.ANSI || {
  reset: '\x1b[0m', bold: '\x1b[1m', dim: '\x1b[2m', italic: '\x1b[3m',
  black: '\x1b[30m', red: '\x1b[31m', green: '\x1b[32m', yellow: '\x1b[33m',
  blue: '\x1b[34m', magenta: '\x1b[35m', cyan: '\x1b[36m', white: '\x1b[37m',
  brightRed: '\x1b[91m', brightGreen: '\x1b[92m', brightYellow: '\x1b[93m',
  brightBlue: '\x1b[94m', brightMagenta: '\x1b[95m', brightCyan: '\x1b[96m',
  brightWhite: '\x1b[97m',
};

// Use centralized DOG_COLORS or define fallback
const DOG_COLORS = colors?.DOG_COLORS || {
  CYNIC: ANSI.brightWhite,
  SCOUT: ANSI.brightGreen,
  GUARDIAN: ANSI.brightRed,
  DEPLOYER: ANSI.yellow,
  ARCHITECT: ANSI.brightBlue,
  JANITOR: ANSI.magenta,
  ORACLE: ANSI.brightYellow,
  ANALYST: ANSI.white,
  SAGE: ANSI.cyan,
  SCHOLAR: ANSI.yellow,
  CARTOGRAPHER: ANSI.green,
};

/**
 * Colorize text for a specific Dog
 * @param {Object} dog - Dog object
 * @param {string} text - Text to colorize
 * @returns {string} Colorized text
 */
function colorize(dog, text) {
  const dogName = dog?.name?.toUpperCase() || 'CYNIC';
  const color = DOG_COLORS[dogName] || ANSI.white;
  return `${color}${text}${ANSI.reset}`;
}

/**
 * Make text bold
 * @param {string} text - Text to bold
 * @returns {string} Bold text
 */
function bold(text) {
  return `${ANSI.bold}${text}${ANSI.reset}`;
}

/**
 * Make text dim
 * @param {string} text - Text to dim
 * @returns {string} Dimmed text
 */
function dim(text) {
  return `${ANSI.dim}${text}${ANSI.reset}`;
}

const ACTIVITY_DIR = path.join(os.homedir(), '.cynic', 'dogs');
const ACTIVITY_FILE = path.join(ACTIVITY_DIR, 'activity.json');

/**
 * Ensure activity directory exists
 */
function ensureActivityDir() {
  if (!fs.existsSync(ACTIVITY_DIR)) {
    fs.mkdirSync(ACTIVITY_DIR, { recursive: true });
  }
}

/**
 * Load activity data
 * @returns {Object} Activity data
 */
function loadActivity() {
  try {
    if (fs.existsSync(ACTIVITY_FILE)) {
      return JSON.parse(fs.readFileSync(ACTIVITY_FILE, 'utf8'));
    }
  } catch (e) { /* ignore */ }
  return {
    session: { started: Date.now(), dogs: {} },
    totals: {},
  };
}

/**
 * Save activity data
 * @param {Object} data - Activity data
 */
function saveActivity(data) {
  ensureActivityDir();
  fs.writeFileSync(ACTIVITY_FILE, JSON.stringify(data, null, 2));
}

/**
 * Record Dog activity
 * @param {Object} dog - Dog object
 * @param {string} action - Action performed
 */
function recordDogActivity(dog, action = 'action') {
  if (!dog?.name) return;

  const data = loadActivity();
  const dogName = dog.name.toUpperCase();

  // Initialize session if stale (>30 min)
  const thirtyMin = 30 * 60 * 1000;
  if (Date.now() - (data.session.started || 0) > thirtyMin) {
    data.session = { started: Date.now(), dogs: {} };
  }

  // Update session counts
  if (!data.session.dogs[dogName]) {
    data.session.dogs[dogName] = { count: 0, actions: [] };
  }
  data.session.dogs[dogName].count++;
  data.session.dogs[dogName].lastAction = action;
  data.session.dogs[dogName].lastTime = Date.now();

  // Update totals
  if (!data.totals[dogName]) {
    data.totals[dogName] = 0;
  }
  data.totals[dogName]++;

  saveActivity(data);
}

/**
 * Get session activity summary
 * @returns {Object} Session summary
 */
function getSessionSummary() {
  const data = loadActivity();
  const session = data.session;

  const dogs = Object.entries(session.dogs || {})
    .map(([name, info]) => ({
      name,
      count: info.count,
      dog: COLLECTIVE_DOGS[name],
    }))
    .sort((a, b) => b.count - a.count);

  const duration = Math.round((Date.now() - (session.started || Date.now())) / 60000);

  return {
    duration,
    dogs,
    totalActions: dogs.reduce((sum, d) => sum + d.count, 0),
    topDog: dogs[0] || null,
  };
}

/**
 * Format session summary for display
 * @returns {string} Formatted summary
 */
function formatSessionSummary() {
  const summary = getSessionSummary();

  if (summary.totalActions === 0) {
    return '── SESSION ────────────────────────────────────────────────\n   No Dog activity recorded.';
  }

  const lines = ['── SESSION DOGS ───────────────────────────────────────────'];
  lines.push(`   Duration: ${summary.duration} min │ Actions: ${summary.totalActions}`);
  lines.push('');

  for (const { name, count, dog } of summary.dogs.slice(0, 5)) {
    const icon = dog?.icon || '🐕';
    const bar = '█'.repeat(Math.min(10, Math.round(count / summary.totalActions * 10)));
    const pct = Math.round(count / summary.totalActions * 100);
    lines.push(`   ${icon} ${name.padEnd(12)} [${bar.padEnd(10, '░')}] ${count} (${pct}%)`);
  }

  if (summary.topDog) {
    const top = summary.topDog;
    const greeting = getDogGreeting(top.dog);
    lines.push('');
    lines.push(`   Most active: ${top.dog?.icon || '🐕'} ${top.name} - "${greeting}"`);
  }

  return lines.join('\n');
}

/**
 * Reset session activity
 */
function resetSession() {
  const data = loadActivity();
  data.session = { started: Date.now(), dogs: {} };
  saveActivity(data);
}

/**
 * Get all-time Dog statistics
 * @returns {Object} Total stats
 */
function getTotalStats() {
  const data = loadActivity();
  return Object.entries(data.totals || {})
    .map(([name, count]) => ({
      name,
      count,
      dog: COLLECTIVE_DOGS[name],
    }))
    .sort((a, b) => b.count - a.count);
}

module.exports = {
  // Dog definitions
  COLLECTIVE_DOGS,
  AGENT_TO_DOG,
  ERROR_TO_DOG,
  TOOL_TO_DOG,

  // Dog lookup
  getDogForAgent,
  getDogForError,
  getDogForTool,
  getAllDogs,
  getDogByName,
  getDogBySefirah,

  // Dog personality
  getDogGreeting,
  getDogQuirk,
  getDogVerb,
  formatDogSpeech,
  formatDogAction,
  formatDogHeader,

  // Colors
  ANSI,
  DOG_COLORS,
  colorize,
  bold,
  dim,

  // Display
  renderSefirotTree,

  // Session tracking
  recordDogActivity,
  getSessionSummary,
  formatSessionSummary,
  resetSession,
  getTotalStats,
};
