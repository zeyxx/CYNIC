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
 * @returns {string} Formatted speech
 */
function formatDogSpeech(dog, message, mood = 'neutral') {
  // Use the Dog's own quirk if available
  const quirk = getDogQuirk(dog);

  // Override for specific moods
  const moodOverrides = {
    warning: '*GROWL*',
    happy: '*tail wag*',
  };

  const prefix = moodOverrides[mood] || quirk;
  return `${dog.icon} ${dog.name}: ${prefix} ${message}`;
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
 * @returns {string} Formatted header
 */
function formatDogHeader(dog, title = '') {
  const titleText = title ? ` - ${title}` : '';
  return `── ${dog.icon} ${dog.name} (${dog.sefirah})${titleText} ──`;
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

module.exports = {
  COLLECTIVE_DOGS,
  AGENT_TO_DOG,
  ERROR_TO_DOG,
  TOOL_TO_DOG,
  getDogForAgent,
  getDogForError,
  getDogForTool,
  getDogGreeting,
  getDogQuirk,
  getDogVerb,
  formatDogSpeech,
  formatDogAction,
  formatDogHeader,
  getAllDogs,
  getDogByName,
  getDogBySefirah,
  renderSefirotTree,
};
