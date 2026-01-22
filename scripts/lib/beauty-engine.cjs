/**
 * CYNIC Beauty Engine
 *
 * "The form of purposiveness without purpose"
 *
 * Philosophical foundations:
 * - Kant: Critique of Judgment, free/dependent beauty
 * - Burke: Sublime and beautiful
 * - Schopenhauer: Aesthetic contemplation
 * - Scruton: Beauty as value
 *
 * φ guides all ratios: 61.8% confidence max
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

// ─────────────────────────────────────────────────────────────
// φ CONSTANTS
// ─────────────────────────────────────────────────────────────

const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2% - uncertainty threshold
const PHI_INV_3 = 0.236067977499790;    // 23.6% - minimum threshold

// ─────────────────────────────────────────────────────────────
// BEAUTY TYPES (Kant)
// ─────────────────────────────────────────────────────────────

const BEAUTY_TYPES = {
  free: {
    name: 'Free Beauty (pulchritudo vaga)',
    description: 'Beauty without concept of what object should be',
    examples: ['Flowers', 'Ornamental designs', 'Music without text'],
    kantian: true,
    purity: PHI_INV + PHI_INV_3,
    requiresConcept: false
  },
  dependent: {
    name: 'Dependent Beauty (pulchritudo adhaerens)',
    description: 'Beauty presupposing concept of what object should be',
    examples: ['Human beauty', 'Horse beauty', 'Building beauty'],
    kantian: true,
    purity: PHI_INV_2,
    requiresConcept: true
  },
  natural: {
    name: 'Natural Beauty',
    description: 'Beauty found in nature',
    examples: ['Sunsets', 'Mountains', 'Flowers'],
    kantian: true,
    purity: PHI_INV
  },
  artistic: {
    name: 'Artistic Beauty',
    description: 'Beauty produced by human intention',
    examples: ['Paintings', 'Sculptures', 'Music'],
    kantian: true,
    purity: PHI_INV_2
  }
};

// ─────────────────────────────────────────────────────────────
// AESTHETIC CATEGORIES (Kant's Four Moments)
// ─────────────────────────────────────────────────────────────

const AESTHETIC_MOMENTS = {
  quality: {
    name: 'Quality',
    description: 'Disinterested pleasure',
    criterion: 'Pleasure without interest in existence of object',
    weight: PHI_INV
  },
  quantity: {
    name: 'Quantity',
    description: 'Universal without concept',
    criterion: 'Claims universal assent without conceptual grounds',
    weight: PHI_INV_2
  },
  relation: {
    name: 'Relation',
    description: 'Purposiveness without purpose',
    criterion: 'Form of finality without definite end',
    weight: PHI_INV
  },
  modality: {
    name: 'Modality',
    description: 'Necessary pleasure',
    criterion: 'Exemplary necessity based on common sense',
    weight: PHI_INV_2
  }
};

// ─────────────────────────────────────────────────────────────
// SUBLIME TYPES (Kant/Burke)
// ─────────────────────────────────────────────────────────────

const SUBLIME_TYPES = {
  mathematical: {
    name: 'Mathematical Sublime',
    description: 'Overwhelms imagination through sheer magnitude',
    examples: ['Starry sky', 'Ocean vastness', 'Grand Canyon'],
    emotion: 'Awe at infinity',
    kantian: true
  },
  dynamical: {
    name: 'Dynamical Sublime',
    description: 'Overwhelms through power and might',
    examples: ['Storms', 'Volcanoes', 'Waterfalls'],
    emotion: 'Respect for our moral nature',
    kantian: true
  },
  burkean_terror: {
    name: 'Burkean Sublime',
    description: 'Delightful terror at a safe distance',
    examples: ['Darkness', 'Vastness', 'Infinity'],
    emotion: 'Astonishment and horror',
    burkean: true
  }
};

// ─────────────────────────────────────────────────────────────
// AESTHETIC PROPERTIES
// ─────────────────────────────────────────────────────────────

const AESTHETIC_PROPERTIES = {
  // Formal properties
  harmony: {
    name: 'Harmony',
    description: 'Parts fitting together coherently',
    category: 'formal',
    weight: PHI_INV
  },
  proportion: {
    name: 'Proportion',
    description: 'Balanced relationships between parts',
    category: 'formal',
    weight: PHI_INV,
    goldenRatio: true // φ!
  },
  unity: {
    name: 'Unity',
    description: 'Coherence as a whole',
    category: 'formal',
    weight: PHI_INV_2
  },
  complexity: {
    name: 'Complexity',
    description: 'Richness of detail and structure',
    category: 'formal',
    weight: PHI_INV_2
  },
  balance: {
    name: 'Balance',
    description: 'Equilibrium of elements',
    category: 'formal',
    weight: PHI_INV_2
  },

  // Expressive properties
  grace: {
    name: 'Grace',
    description: 'Effortless elegance',
    category: 'expressive',
    weight: PHI_INV_2
  },
  elegance: {
    name: 'Elegance',
    description: 'Refined simplicity',
    category: 'expressive',
    weight: PHI_INV_2
  },
  dynamism: {
    name: 'Dynamism',
    description: 'Sense of energy and movement',
    category: 'expressive',
    weight: PHI_INV_3
  },
  serenity: {
    name: 'Serenity',
    description: 'Peaceful calm',
    category: 'expressive',
    weight: PHI_INV_3
  },

  // Evaluative properties
  beautiful: {
    name: 'Beautiful',
    description: 'Pleasing form',
    category: 'evaluative',
    weight: PHI_INV
  },
  sublime: {
    name: 'Sublime',
    description: 'Overwhelming greatness',
    category: 'evaluative',
    weight: PHI_INV
  },
  ugly: {
    name: 'Ugly',
    description: 'Displeasing form',
    category: 'evaluative',
    weight: PHI_INV_3
  },
  kitsch: {
    name: 'Kitsch',
    description: 'Cheap sentimentality',
    category: 'evaluative',
    weight: PHI_INV_3
  }
};

// ─────────────────────────────────────────────────────────────
// STATE
// ─────────────────────────────────────────────────────────────

const state = {
  objects: new Map(),             // Aesthetic objects
  judgments: [],                  // Beauty judgments
  experiences: [],                // Aesthetic experiences
  comparisons: [],                // Comparative assessments
  stats: {
    objectsRegistered: 0,
    judgmentsMade: 0,
    experiencesRecorded: 0,
    sublimeEncounters: 0
  }
};

// Storage
const STORAGE_DIR = path.join(os.homedir(), '.cynic', 'beauty-engine');
const STATE_FILE = path.join(STORAGE_DIR, 'state.json');
const HISTORY_FILE = path.join(STORAGE_DIR, 'history.jsonl');

// ─────────────────────────────────────────────────────────────
// CORE FUNCTIONS
// ─────────────────────────────────────────────────────────────

/**
 * Register an object for aesthetic evaluation
 */
function registerObject(id, spec) {
  const obj = {
    id,
    name: spec.name || id,
    type: spec.type || 'artistic', // natural, artistic

    // Beauty type
    beautyType: spec.beautyType || 'free',
    beautyTypeInfo: BEAUTY_TYPES[spec.beautyType || 'free'],

    // Aesthetic properties
    properties: {},

    // Associated concept (for dependent beauty)
    concept: spec.concept || null,

    // Judgments received
    judgments: [],

    created: Date.now()
  };

  // Initialize properties
  for (const prop of (spec.properties || [])) {
    if (AESTHETIC_PROPERTIES[prop]) {
      obj.properties[prop] = {
        present: true,
        intensity: PHI_INV_2,
        info: AESTHETIC_PROPERTIES[prop]
      };
    }
  }

  state.objects.set(id, obj);
  state.stats.objectsRegistered++;

  appendHistory({
    type: 'object_registered',
    objectId: id,
    beautyType: obj.beautyType,
    timestamp: Date.now()
  });

  return obj;
}

/**
 * Make aesthetic judgment (Kantian analysis)
 */
function judgeBeauty(objectId, spec = {}) {
  const obj = state.objects.get(objectId);
  if (!obj) return null;

  const judgment = {
    id: `judge_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    objectId,

    // Four moments assessment
    moments: {
      quality: {
        satisfied: spec.disinterested !== false,
        note: spec.disinterested !== false
          ? 'Pleasure is disinterested'
          : 'Interest contaminates judgment'
      },
      quantity: {
        satisfied: spec.claimsUniversality !== false,
        note: spec.claimsUniversality !== false
          ? 'Claims universal agreement'
          : 'Merely personal preference'
      },
      relation: {
        satisfied: spec.purposivenessWithoutPurpose !== false,
        note: spec.purposivenessWithoutPurpose !== false
          ? 'Form of finality without end'
          : 'Has definite purpose'
      },
      modality: {
        satisfied: spec.necessaryPleasure === true,
        note: spec.necessaryPleasure
          ? 'Exemplary necessity'
          : 'Contingent pleasure'
      }
    },

    // Overall beauty score
    beautyScore: 0,

    // Verdict
    verdict: null,

    timestamp: Date.now()
  };

  // Calculate beauty score from moments
  let totalWeight = 0;
  let satisfiedWeight = 0;

  for (const [key, moment] of Object.entries(judgment.moments)) {
    const weight = AESTHETIC_MOMENTS[key].weight;
    totalWeight += weight;
    if (moment.satisfied) {
      satisfiedWeight += weight;
    }
  }

  judgment.beautyScore = Math.min(satisfiedWeight / totalWeight, PHI_INV);

  // Determine verdict
  const momentsSatisfied = Object.values(judgment.moments).filter(m => m.satisfied).length;

  if (momentsSatisfied === 4) {
    judgment.verdict = 'Pure judgment of beauty';
    judgment.kantianStatus = 'Satisfies all four moments';
  } else if (momentsSatisfied >= 2) {
    judgment.verdict = 'Impure aesthetic judgment';
    judgment.kantianStatus = `Satisfies ${momentsSatisfied}/4 moments`;
  } else {
    judgment.verdict = 'Not an aesthetic judgment proper';
    judgment.kantianStatus = 'Fails Kantian criteria';
  }

  judgment.confidence = PHI_INV;

  // Store judgment
  obj.judgments.push(judgment);
  state.judgments.push(judgment);
  state.stats.judgmentsMade++;

  appendHistory({
    type: 'beauty_judgment',
    judgment,
    timestamp: Date.now()
  });

  return judgment;
}

/**
 * Evaluate aesthetic properties
 */
function evaluateProperties(objectId, properties) {
  const obj = state.objects.get(objectId);
  if (!obj) return null;

  const evaluation = {
    objectId,
    properties: {},
    overallScore: 0,
    timestamp: Date.now()
  };

  let totalWeight = 0;
  let weightedSum = 0;

  for (const [prop, intensity] of Object.entries(properties)) {
    if (AESTHETIC_PROPERTIES[prop]) {
      const propInfo = AESTHETIC_PROPERTIES[prop];
      const normalizedIntensity = Math.min(intensity, 1);

      obj.properties[prop] = {
        present: true,
        intensity: normalizedIntensity,
        info: propInfo
      };

      evaluation.properties[prop] = {
        intensity: normalizedIntensity,
        weight: propInfo.weight,
        contribution: normalizedIntensity * propInfo.weight
      };

      totalWeight += propInfo.weight;
      weightedSum += normalizedIntensity * propInfo.weight;

      // Special handling for proportion with golden ratio
      if (prop === 'proportion' && propInfo.goldenRatio) {
        evaluation.properties[prop].phiNote = 'φ (golden ratio) is aesthetic ideal';
      }
    }
  }

  evaluation.overallScore = totalWeight > 0
    ? Math.min(weightedSum / totalWeight, PHI_INV)
    : 0;

  return evaluation;
}

/**
 * Record sublime experience
 */
function recordSublime(objectId, sublimeType = 'mathematical') {
  const obj = state.objects.get(objectId);

  const experience = {
    id: `sublime_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
    objectId,

    type: sublimeType,
    typeInfo: SUBLIME_TYPES[sublimeType],

    // Kantian analysis
    overwhelmsImagination: true,
    revealsReason: sublimeType !== 'burkean_terror',

    // Emotional response
    emotion: SUBLIME_TYPES[sublimeType]?.emotion || 'Awe',

    // Distinction from beauty
    notBeauty: true,
    note: 'Sublime differs from beautiful: formlessness vs form',

    timestamp: Date.now()
  };

  state.experiences.push(experience);
  state.stats.experiencesRecorded++;
  state.stats.sublimeEncounters++;

  appendHistory({
    type: 'sublime_experience',
    experience,
    timestamp: Date.now()
  });

  return experience;
}

/**
 * Assess disinterestedness (Kant's first moment)
 */
function assessDisinterestedness(judgment) {
  const analysis = {
    judgment,
    factors: [],
    isDisinterested: true,
    confidence: PHI_INV_2
  };

  // Check for contaminating interests
  const contaminants = [
    { type: 'cognitive', question: 'Is pleasure based on understanding?', weight: PHI_INV_3 },
    { type: 'moral', question: 'Is pleasure based on moral approval?', weight: PHI_INV_3 },
    { type: 'sensory', question: 'Is pleasure merely agreeable sensation?', weight: PHI_INV_2 },
    { type: 'utility', question: 'Is pleasure based on usefulness?', weight: PHI_INV_2 }
  ];

  for (const c of contaminants) {
    if (judgment[c.type + 'Interest']) {
      analysis.factors.push({
        type: c.type,
        present: true,
        contaminates: true
      });
      analysis.isDisinterested = false;
    }
  }

  analysis.kantianVerdict = analysis.isDisinterested
    ? 'Pleasure is disinterested: pure aesthetic'
    : 'Interest contaminates: not pure aesthetic';

  return analysis;
}

/**
 * Compare aesthetic merit
 */
function compareAesthetically(object1Id, object2Id) {
  const obj1 = state.objects.get(object1Id);
  const obj2 = state.objects.get(object2Id);

  if (!obj1 || !obj2) return null;

  const comparison = {
    objects: [object1Id, object2Id],
    propertyComparison: {},
    judgmentComparison: {},
    winner: null,
    confidence: PHI_INV_2,
    timestamp: Date.now()
  };

  // Compare properties
  const allProps = new Set([
    ...Object.keys(obj1.properties),
    ...Object.keys(obj2.properties)
  ]);

  let score1 = 0;
  let score2 = 0;

  for (const prop of allProps) {
    const p1 = obj1.properties[prop];
    const p2 = obj2.properties[prop];

    comparison.propertyComparison[prop] = {
      [object1Id]: p1?.intensity || 0,
      [object2Id]: p2?.intensity || 0,
      winner: (p1?.intensity || 0) > (p2?.intensity || 0)
        ? object1Id
        : (p2?.intensity || 0) > (p1?.intensity || 0)
          ? object2Id
          : 'tie'
    };

    score1 += (p1?.intensity || 0) * (p1?.info?.weight || PHI_INV_2);
    score2 += (p2?.intensity || 0) * (p2?.info?.weight || PHI_INV_2);
  }

  // Compare judgments
  const avgJudgment1 = obj1.judgments.length > 0
    ? obj1.judgments.reduce((s, j) => s + j.beautyScore, 0) / obj1.judgments.length
    : PHI_INV_2;
  const avgJudgment2 = obj2.judgments.length > 0
    ? obj2.judgments.reduce((s, j) => s + j.beautyScore, 0) / obj2.judgments.length
    : PHI_INV_2;

  comparison.judgmentComparison = {
    [object1Id]: avgJudgment1,
    [object2Id]: avgJudgment2
  };

  // Determine winner
  const total1 = score1 + avgJudgment1;
  const total2 = score2 + avgJudgment2;

  if (total1 > total2 + PHI_INV_3) {
    comparison.winner = object1Id;
  } else if (total2 > total1 + PHI_INV_3) {
    comparison.winner = object2Id;
  } else {
    comparison.winner = 'comparable';
    comparison.note = 'Aesthetic merit is roughly equal';
  }

  state.comparisons.push(comparison);

  return comparison;
}

/**
 * Analyze for golden ratio (φ) presence
 */
function analyzeGoldenRatio(objectId, measurements) {
  const analysis = {
    objectId,
    measurements,
    ratiosFound: [],
    goldenRatioPresent: false,
    aestheticSignificance: null,
    timestamp: Date.now()
  };

  // Check ratios between adjacent measurements
  for (let i = 0; i < measurements.length - 1; i++) {
    const ratio = measurements[i] / measurements[i + 1];
    const inverseRatio = measurements[i + 1] / measurements[i];

    const closeToPhiDirect = Math.abs(ratio - PHI) < 0.1;
    const closeToPhiInverse = Math.abs(inverseRatio - PHI) < 0.1;

    if (closeToPhiDirect || closeToPhiInverse) {
      analysis.ratiosFound.push({
        pair: [measurements[i], measurements[i + 1]],
        ratio: closeToPhiDirect ? ratio : inverseRatio,
        deviation: closeToPhiDirect
          ? Math.abs(ratio - PHI)
          : Math.abs(inverseRatio - PHI)
      });
      analysis.goldenRatioPresent = true;
    }
  }

  if (analysis.goldenRatioPresent) {
    analysis.aestheticSignificance = {
      phiPresent: true,
      note: 'φ (golden ratio) associated with aesthetic harmony',
      historicalUse: ['Greek architecture', 'Renaissance art', 'Nature']
    };
  }

  return analysis;
}

// ─────────────────────────────────────────────────────────────
// PERSISTENCE
// ─────────────────────────────────────────────────────────────

function ensureStorageDir() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }
}

function saveState() {
  ensureStorageDir();

  const serializable = {
    objects: Array.from(state.objects.entries()),
    judgments: state.judgments.slice(-100),
    experiences: state.experiences.slice(-50),
    comparisons: state.comparisons.slice(-50),
    stats: state.stats
  };

  fs.writeFileSync(STATE_FILE, JSON.stringify(serializable, null, 2));
}

function loadState() {
  ensureStorageDir();

  if (fs.existsSync(STATE_FILE)) {
    try {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf8'));
      state.objects = new Map(data.objects || []);
      state.judgments = data.judgments || [];
      state.experiences = data.experiences || [];
      state.comparisons = data.comparisons || [];
      state.stats = data.stats || state.stats;
    } catch (e) {
      console.error('Failed to load beauty engine state:', e.message);
    }
  }
}

function appendHistory(entry) {
  ensureStorageDir();
  fs.appendFileSync(HISTORY_FILE, JSON.stringify(entry) + '\n');
}

// ─────────────────────────────────────────────────────────────
// FORMATTING
// ─────────────────────────────────────────────────────────────

function formatStatus() {
  const lines = [
    '── BEAUTY ENGINE ──────────────────────────────────────────',
    ''
  ];

  lines.push(`   Objects: ${state.objects.size} | Judgments: ${state.stats.judgmentsMade}`);
  lines.push(`   Experiences: ${state.stats.experiencesRecorded} | Sublime: ${state.stats.sublimeEncounters}`);
  lines.push('');

  return lines.join('\n');
}

function getStats() {
  return {
    ...state.stats,
    objectCount: state.objects.size,
    recentJudgments: state.judgments.slice(-5)
  };
}

// ─────────────────────────────────────────────────────────────
// INITIALIZATION
// ─────────────────────────────────────────────────────────────

function init() {
  loadState();

  // Auto-save periodically
  setInterval(() => saveState(), 60000);

  return {
    initialized: true,
    objects: state.objects.size
  };
}

// ─────────────────────────────────────────────────────────────
// EXPORTS
// ─────────────────────────────────────────────────────────────

module.exports = {
  // Constants
  PHI,
  PHI_INV,
  PHI_INV_2,
  PHI_INV_3,

  // Type definitions
  BEAUTY_TYPES,
  AESTHETIC_MOMENTS,
  SUBLIME_TYPES,
  AESTHETIC_PROPERTIES,

  // Core functions
  registerObject,
  judgeBeauty,
  evaluateProperties,
  recordSublime,
  assessDisinterestedness,
  compareAesthetically,
  analyzeGoldenRatio,

  // State access
  getObject: (id) => state.objects.get(id),
  getJudgments: (objectId) => state.judgments.filter(j => j.objectId === objectId),
  getExperiences: () => [...state.experiences],

  // Persistence
  saveState,
  loadState,

  // Formatting
  formatStatus,
  getStats,
  init
};
