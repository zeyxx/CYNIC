/**
 * Taste Engine - Hume/Kant
 *
 * "Beauty is no quality in things themselves: It exists merely in the mind
 *  which contemplates them; and each mind perceives a different beauty."
 *  — David Hume, "Of the Standard of Taste"
 *
 * Yet Hume argues there IS a standard...
 *
 * Implements:
 * - Hume's standard of taste (ideal critic)
 * - Kant's judgment of taste
 * - Aesthetic disagreement and convergence
 * - Critic calibration
 *
 * φ guides the thresholds for taste agreement.
 */

const fs = require('fs');
const path = require('path');

// φ constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;      // 61.8% - max confidence
const PHI_INV_2 = 0.381966011250105;    // 38.2%
const PHI_INV_3 = 0.236067977499790;    // 23.6%

// Storage
const STORAGE_DIR = path.join(require('os').homedir(), '.cynic', 'taste-engine');

// Hume's qualities of the ideal critic
const IDEAL_CRITIC_QUALITIES = {
  delicacy: {
    name: 'Delicacy of Imagination',
    description: 'Sensitivity to fine distinctions',
    weight: PHI_INV,
    measuredBy: 'discrimination_tests'
  },
  practice: {
    name: 'Practice',
    description: 'Experience with the art form',
    weight: PHI_INV_2,
    measuredBy: 'exposure_hours'
  },
  comparison: {
    name: 'Comparison',
    description: 'Familiarity with different works',
    weight: PHI_INV_2,
    measuredBy: 'works_compared'
  },
  freedom_prejudice: {
    name: 'Freedom from Prejudice',
    description: 'Unprejudiced consideration',
    weight: PHI_INV,
    measuredBy: 'bias_scores'
  },
  good_sense: {
    name: 'Good Sense',
    description: 'Rational judgment',
    weight: PHI_INV_2,
    measuredBy: 'consistency_scores'
  }
};

// Aesthetic categories
const AESTHETIC_CATEGORIES = {
  beautiful: {
    name: 'The Beautiful',
    characteristics: ['bounded', 'harmonious', 'pleasurable', 'restful'],
    kantMoment: 'free play of imagination and understanding'
  },
  sublime: {
    name: 'The Sublime',
    characteristics: ['vast', 'powerful', 'overwhelming', 'transcendent'],
    kantMoment: 'reason surpasses imagination'
  },
  picturesque: {
    name: 'The Picturesque',
    characteristics: ['irregular', 'varied', 'intricate', 'rough'],
    origin: 'Gilpin, Price, Knight'
  },
  grotesque: {
    name: 'The Grotesque',
    characteristics: ['distorted', 'hybrid', 'unsettling', 'playful'],
    origin: 'Renaissance ornament'
  },
  kitsch: {
    name: 'Kitsch',
    characteristics: ['sentimental', 'mass-produced', 'formulaic'],
    origin: 'Greenberg, Kulka'
  },
  camp: {
    name: 'Camp',
    characteristics: ['exaggerated', 'theatrical', 'ironic', 'failed_seriousness'],
    origin: 'Susan Sontag'
  }
};

// State
const state = {
  critics: new Map(),
  judgments: new Map(),       // artwork -> judgments
  consensuses: new Map(),     // artwork -> consensus
  calibration: new Map(),     // critic -> accuracy
  disputes: []                // documented disagreements
};

/**
 * Initialize the taste engine
 */
function init() {
  if (!fs.existsSync(STORAGE_DIR)) {
    fs.mkdirSync(STORAGE_DIR, { recursive: true });
  }

  // Load persisted state
  const statePath = path.join(STORAGE_DIR, 'state.json');
  if (fs.existsSync(statePath)) {
    try {
      const saved = JSON.parse(fs.readFileSync(statePath, 'utf8'));
      if (saved.critics) state.critics = new Map(Object.entries(saved.critics));
      if (saved.judgments) state.judgments = new Map(Object.entries(saved.judgments));
      if (saved.consensuses) state.consensuses = new Map(Object.entries(saved.consensuses));
      if (saved.calibration) state.calibration = new Map(Object.entries(saved.calibration));
      if (saved.disputes) state.disputes = saved.disputes;
    } catch {
      // Start fresh
    }
  }

  return { status: 'initialized', critics: state.critics.size };
}

/**
 * Save state
 */
function saveState() {
  const statePath = path.join(STORAGE_DIR, 'state.json');
  const toSave = {
    critics: Object.fromEntries(state.critics),
    judgments: Object.fromEntries(state.judgments),
    consensuses: Object.fromEntries(state.consensuses),
    calibration: Object.fromEntries(state.calibration),
    disputes: state.disputes
  };
  fs.writeFileSync(statePath, JSON.stringify(toSave, null, 2));
}

/**
 * Register a critic (Hume's ideal critic)
 */
function registerCritic(id, spec = {}) {
  const critic = {
    id,
    name: spec.name || id,
    qualities: {},
    totalScore: 0,
    judgmentHistory: [],
    registeredAt: Date.now()
  };

  // Evaluate against ideal critic qualities
  for (const [qualityId, quality] of Object.entries(IDEAL_CRITIC_QUALITIES)) {
    const score = spec[qualityId] || 0.5; // Default to middle
    critic.qualities[qualityId] = {
      name: quality.name,
      score: Math.min(score, PHI_INV), // Cap at φ⁻¹
      weight: quality.weight
    };
    critic.totalScore += score * quality.weight;
  }

  // Normalize to 0-1
  const maxScore = Object.values(IDEAL_CRITIC_QUALITIES)
    .reduce((sum, q) => sum + q.weight, 0);
  critic.normalizedScore = critic.totalScore / maxScore;

  // Hume's ideal critic threshold
  critic.isIdealCritic = critic.normalizedScore >= PHI_INV;

  state.critics.set(id, critic);
  saveState();

  return critic;
}

/**
 * Record an aesthetic judgment
 */
function judgeAesthetic(artworkId, criticId, judgment) {
  const critic = state.critics.get(criticId);

  if (!critic) {
    return { error: 'Critic not registered' };
  }

  const aestheticJudgment = {
    id: `judge_${Date.now()}`,
    artworkId,
    criticId,
    criticScore: critic.normalizedScore,

    // The judgment itself
    category: judgment.category || 'beautiful',    // beautiful, sublime, etc.
    rating: Math.min(judgment.rating || 0.5, PHI_INV), // 0-1, capped at φ⁻¹
    confidence: judgment.confidence || PHI_INV_2,

    // Kant's four moments
    quality: judgment.quality || null,       // Disinterested?
    quantity: judgment.quantity || null,     // Universal claim?
    relation: judgment.relation || null,     // Purposiveness?
    modality: judgment.modality || null,     // Necessary?

    // Reasoning
    reasons: judgment.reasons || [],

    timestamp: Date.now()
  };

  // Add to judgments
  if (!state.judgments.has(artworkId)) {
    state.judgments.set(artworkId, []);
  }
  state.judgments.get(artworkId).push(aestheticJudgment);

  // Add to critic history
  critic.judgmentHistory.push(aestheticJudgment);

  // Recalculate consensus
  updateConsensus(artworkId);

  saveState();

  return aestheticJudgment;
}

/**
 * Update consensus for an artwork
 */
function updateConsensus(artworkId) {
  const judgments = state.judgments.get(artworkId) || [];

  if (judgments.length === 0) {
    return null;
  }

  // Weight judgments by critic quality (Hume's standard)
  let weightedSum = 0;
  let totalWeight = 0;
  const categoryVotes = {};

  for (const j of judgments) {
    const weight = j.criticScore;
    weightedSum += j.rating * weight;
    totalWeight += weight;

    // Category voting
    categoryVotes[j.category] = (categoryVotes[j.category] || 0) + weight;
  }

  const consensus = {
    artworkId,
    rating: totalWeight > 0 ? weightedSum / totalWeight : 0.5,
    judgmentCount: judgments.length,
    category: Object.entries(categoryVotes)
      .sort((a, b) => b[1] - a[1])[0]?.[0] || 'unknown',
    agreement: calculateAgreement(judgments),
    lastUpdated: Date.now()
  };

  state.consensuses.set(artworkId, consensus);

  return consensus;
}

/**
 * Calculate agreement level among judgments
 */
function calculateAgreement(judgments) {
  if (judgments.length < 2) return 1;

  const ratings = judgments.map(j => j.rating);
  const mean = ratings.reduce((a, b) => a + b, 0) / ratings.length;
  const variance = ratings.reduce((sum, r) => sum + Math.pow(r - mean, 2), 0) / ratings.length;
  const stdDev = Math.sqrt(variance);

  // Convert standard deviation to agreement (lower variance = higher agreement)
  // Scale: stdDev of 0 = agreement 1, stdDev of 0.5 = agreement 0
  const agreement = Math.max(0, 1 - (stdDev * 2));

  return agreement;
}

/**
 * Identify and record a taste dispute
 */
function recordDispute(artworkId, critic1Id, critic2Id, description) {
  const j1 = state.judgments.get(artworkId)?.find(j => j.criticId === critic1Id);
  const j2 = state.judgments.get(artworkId)?.find(j => j.criticId === critic2Id);

  if (!j1 || !j2) {
    return { error: 'Both critics must have judged this artwork' };
  }

  const dispute = {
    id: `dispute_${Date.now()}`,
    artworkId,
    critics: [critic1Id, critic2Id],
    positions: {
      [critic1Id]: { rating: j1.rating, category: j1.category },
      [critic2Id]: { rating: j2.rating, category: j2.category }
    },
    ratingDifference: Math.abs(j1.rating - j2.rating),
    categoryDisagreement: j1.category !== j2.category,
    description,
    resolution: null,
    timestamp: Date.now()
  };

  state.disputes.push(dispute);
  saveState();

  // Hume's insight
  const insight = dispute.ratingDifference > PHI_INV_2
    ? 'Significant disagreement - but Hume argues verdicts of ideal critics converge'
    : 'Minor disagreement - within expected variation';

  return { dispute, insight };
}

/**
 * Apply Kant's four moments analysis
 */
function analyzeKantianJudgment(judgment) {
  const analysis = {
    judgment: judgment.id || judgment,
    moments: {},
    isPureAesthetic: true
  };

  // Quality: Is the pleasure disinterested?
  analysis.moments.quality = {
    name: 'Quality',
    question: 'Is the pleasure disinterested?',
    answer: judgment.quality || 'unknown',
    kantSays: 'Taste is the faculty of judging an object by a satisfaction without any interest'
  };

  if (judgment.quality === false) {
    analysis.isPureAesthetic = false;
    analysis.impurity = 'interested pleasure';
  }

  // Quantity: Does it claim universality?
  analysis.moments.quantity = {
    name: 'Quantity',
    question: 'Does the judgment claim universal validity?',
    answer: judgment.quantity || 'unknown',
    kantSays: 'The beautiful is that which pleases universally without a concept'
  };

  // Relation: Is there purposiveness without purpose?
  analysis.moments.relation = {
    name: 'Relation',
    question: 'Is there purposiveness without definite purpose?',
    answer: judgment.relation || 'unknown',
    kantSays: 'Beauty is the form of purposiveness without representation of an end'
  };

  // Modality: Is the pleasure felt as necessary?
  analysis.moments.modality = {
    name: 'Modality',
    question: 'Is the satisfaction felt as necessary for all?',
    answer: judgment.modality || 'unknown',
    kantSays: 'The beautiful is that which is cognized without a concept as the object of necessary satisfaction'
  };

  // Verdict
  if (analysis.isPureAesthetic) {
    analysis.verdict = 'Pure aesthetic judgment (free beauty)';
  } else {
    analysis.verdict = 'Impure/dependent beauty or agreeable';
  }

  return analysis;
}

/**
 * Evaluate critic calibration against consensus
 */
function calibrateCritic(criticId) {
  const critic = state.critics.get(criticId);
  if (!critic) {
    return { error: 'Critic not found' };
  }

  const judgments = critic.judgmentHistory;
  if (judgments.length === 0) {
    return { critic: criticId, accuracy: 0.5, samples: 0 };
  }

  let totalDeviation = 0;
  let samples = 0;

  for (const j of judgments) {
    const consensus = state.consensuses.get(j.artworkId);
    if (consensus && consensus.judgmentCount > 1) {
      const deviation = Math.abs(j.rating - consensus.rating);
      totalDeviation += deviation;
      samples++;
    }
  }

  const avgDeviation = samples > 0 ? totalDeviation / samples : 0.5;
  const accuracy = 1 - avgDeviation; // Lower deviation = higher accuracy

  const calibration = {
    criticId,
    accuracy: Math.min(accuracy, PHI_INV), // Cap at φ⁻¹
    samples,
    isCalibrated: samples >= 5,
    lastUpdated: Date.now()
  };

  state.calibration.set(criticId, calibration);
  saveState();

  // Hume's insight
  calibration.insight = accuracy >= PHI_INV_2
    ? 'Critic aligns with collective judgment (approaching ideal critic)'
    : 'Critic diverges from consensus (may have unique perspective or bias)';

  return calibration;
}

/**
 * Get taste verdicts (Hume's convergence of ideal critics)
 */
function getTasteVerdict(artworkId) {
  const consensus = state.consensuses.get(artworkId);
  const judgments = state.judgments.get(artworkId) || [];

  if (!consensus) {
    return { artworkId, verdict: 'no_judgments', confidence: 0 };
  }

  // Filter to ideal critics
  const idealJudgments = judgments.filter(j => {
    const critic = state.critics.get(j.criticId);
    return critic && critic.isIdealCritic;
  });

  let verdict;
  let confidence;

  if (idealJudgments.length === 0) {
    verdict = 'no_ideal_critics';
    confidence = PHI_INV_3;
  } else {
    // Calculate ideal critic consensus
    const idealRatings = idealJudgments.map(j => j.rating);
    const idealMean = idealRatings.reduce((a, b) => a + b, 0) / idealRatings.length;
    const idealAgreement = calculateAgreement(idealJudgments);

    verdict = idealMean >= PHI_INV_2 ? 'aesthetically_valuable' :
      idealMean >= PHI_INV_3 ? 'mixed_reception' : 'aesthetically_lacking';
    confidence = idealAgreement * PHI_INV; // Scale by max confidence
  }

  return {
    artworkId,
    verdict,
    confidence,
    overallRating: consensus.rating,
    category: consensus.category,
    agreement: consensus.agreement,
    judgmentCount: judgments.length,
    idealCriticCount: idealJudgments.length,

    // Hume
    humeInsight: `"The joint verdict of ${idealJudgments.length > 0 ? 'ideal' : 'available'} critics ` +
      `constitutes the true standard of taste."`
  };
}

/**
 * Format status for display
 */
function formatStatus() {
  const lines = [
    '═══════════════════════════════════════════════════════════',
    '🎭 TASTE ENGINE - "Beauty exists in the mind that contemplates"',
    '═══════════════════════════════════════════════════════════',
    '',
    '── IDEAL CRITIC QUALITIES (Hume) ──────────────────────────'
  ];

  for (const [key, quality] of Object.entries(IDEAL_CRITIC_QUALITIES)) {
    lines.push(`   ${quality.name}: weight ${(quality.weight * 100).toFixed(1)}%`);
  }

  lines.push('');
  lines.push('── CRITICS ────────────────────────────────────────────────');

  let idealCount = 0;
  for (const critic of state.critics.values()) {
    if (critic.isIdealCritic) idealCount++;
  }
  lines.push(`   Total critics: ${state.critics.size}`);
  lines.push(`   Ideal critics: ${idealCount} (score ≥ ${(PHI_INV * 100).toFixed(1)}%)`);

  lines.push('');
  lines.push('── AESTHETIC CATEGORIES ───────────────────────────────────');
  for (const [key, cat] of Object.entries(AESTHETIC_CATEGORIES)) {
    lines.push(`   ${cat.name}: ${cat.characteristics.join(', ')}`);
  }

  lines.push('');
  lines.push('── STATISTICS ─────────────────────────────────────────────');
  const totalJudgments = Array.from(state.judgments.values())
    .reduce((sum, arr) => sum + arr.length, 0);
  lines.push(`   Artworks judged: ${state.judgments.size}`);
  lines.push(`   Total judgments: ${totalJudgments}`);
  lines.push(`   Disputes recorded: ${state.disputes.length}`);

  lines.push('');
  lines.push('═══════════════════════════════════════════════════════════');
  lines.push('*sniff* "Strong sense united to delicate sentiment."');
  lines.push('═══════════════════════════════════════════════════════════');

  return lines.join('\n');
}

/**
 * Get statistics
 */
function getStats() {
  let idealCount = 0;
  for (const critic of state.critics.values()) {
    if (critic.isIdealCritic) idealCount++;
  }

  const totalJudgments = Array.from(state.judgments.values())
    .reduce((sum, arr) => sum + arr.length, 0);

  // Average agreement
  let totalAgreement = 0;
  let consensusCount = 0;
  for (const c of state.consensuses.values()) {
    totalAgreement += c.agreement;
    consensusCount++;
  }
  const avgAgreement = consensusCount > 0 ? totalAgreement / consensusCount : 0;

  return {
    totalCritics: state.critics.size,
    idealCritics: idealCount,
    artworksJudged: state.judgments.size,
    totalJudgments,
    disputes: state.disputes.length,
    averageAgreement: avgAgreement,
    consensuses: state.consensuses.size
  };
}

module.exports = {
  // Core
  init,
  formatStatus,
  getStats,

  // Critics (Hume)
  registerCritic,
  calibrateCritic,
  IDEAL_CRITIC_QUALITIES,

  // Judgments
  judgeAesthetic,
  recordDispute,
  getTasteVerdict,

  // Kant
  analyzeKantianJudgment,

  // Categories
  AESTHETIC_CATEGORIES,

  // State access
  updateConsensus,

  // Constants
  PHI,
  PHI_INV
};
