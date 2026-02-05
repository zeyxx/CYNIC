/**
 * CYNIC Reward Function
 *
 * Converts judgment feedback into scalar rewards for GRPO training.
 * φ-aligned: max reward = φ⁻¹ (0.618), penalties scale with confidence.
 *
 * Reward Design:
 *   correct   → +φ⁻¹ (0.618) — maximum praise
 *   incorrect → -(confidence × φ) — confident mistakes hurt more
 *   partial   → proportional to score gap, capped at φ⁻²
 *   no feedback → 0 (neutral, no gradient signal)
 *
 * "φ distrusts φ" — even rewards are bounded
 *
 * @module cynic/training/reward
 */

// Golden ratio constants
const PHI = 1.618033988749895;
const PHI_INV = 0.6180339887498949;   // φ⁻¹ = 1/φ
const PHI_INV_2 = 0.3819660112501051; // φ⁻² = 1/φ²

/**
 * Compute reward for a judgment given feedback.
 *
 * @param {number} qScore - Q-Score of the judgment (0-100)
 * @param {number} confidence - Confidence of the judgment (0-1)
 * @param {Object|null} feedback - Feedback object { outcome, actual_score, reason }
 * @returns {number} Reward scalar in [-φ, +φ⁻¹]
 */
export function computeReward(qScore, confidence, feedback) {
  // No feedback = neutral (no training signal)
  if (!feedback) return 0;

  const { outcome, actual_score } = feedback;

  switch (outcome) {
    case 'correct':
      // Correct judgment: reward scaled by confidence (confident + correct = best)
      // But capped at φ⁻¹ — even perfect work doesn't exceed the golden bound
      return Math.min(PHI_INV, 0.3 + confidence * 0.4);

    case 'incorrect':
      // Incorrect judgment: penalty scales with confidence
      // High confidence + wrong = worst outcome (overconfident mistake)
      // Low confidence + wrong = smaller penalty (appropriately uncertain)
      return -(confidence * PHI);

    case 'partial': {
      // Partial: reward proportional to how close we were
      if (actual_score == null) return 0;

      const gap = Math.abs(qScore - actual_score) / 100;

      // Close gap (<10%) = mild positive, large gap = negative
      if (gap < 0.1) return PHI_INV_2;            // Very close: +0.382
      if (gap < 0.2) return PHI_INV_2 * 0.5;      // Close: +0.191
      if (gap < 0.3) return 0;                     // Neutral zone
      return -(gap * confidence);                   // Far: negative, scaled by confidence
    }

    default:
      return 0;
  }
}

/**
 * Compute batch rewards for a set of judgment-feedback pairs.
 *
 * @param {Array<{qScore: number, confidence: number, feedback: Object}>} batch
 * @returns {Array<number>} Reward scalars
 */
export function computeBatchRewards(batch) {
  return batch.map(({ qScore, confidence, feedback }) =>
    computeReward(qScore, confidence, feedback)
  );
}

/**
 * Get reward statistics for a batch.
 *
 * @param {number[]} rewards
 * @returns {Object} { mean, std, min, max, positive, negative, neutral }
 */
export function rewardStats(rewards) {
  if (rewards.length === 0) {
    return { mean: 0, std: 0, min: 0, max: 0, positive: 0, negative: 0, neutral: 0 };
  }

  const sum = rewards.reduce((a, b) => a + b, 0);
  const mean = sum / rewards.length;
  const variance = rewards.reduce((a, r) => a + (r - mean) ** 2, 0) / rewards.length;

  return {
    mean: Math.round(mean * 1000) / 1000,
    std: Math.round(Math.sqrt(variance) * 1000) / 1000,
    min: Math.round(Math.min(...rewards) * 1000) / 1000,
    max: Math.round(Math.max(...rewards) * 1000) / 1000,
    positive: rewards.filter(r => r > 0).length,
    negative: rewards.filter(r => r < 0).length,
    neutral: rewards.filter(r => r === 0).length,
    total: rewards.length,
  };
}

/**
 * Format training example for SFT (Supervised Fine-Tuning).
 *
 * Converts a CYNIC judgment into a chat-format training example.
 *
 * @param {Object} record - Training record from export
 * @returns {Object} { messages: [{role, content}], reward }
 */
export function formatSFTExample(record) {
  const { input, judgment, feedback, reward } = record;

  // System message: CYNIC identity
  const system = `You are CYNIC (κυνικός), a cynical judgment system. Score items on 25 dimensions across 4 axioms (PHI, VERIFY, CULTURE, BURN). Your confidence never exceeds 61.8% (φ⁻¹). Be direct, skeptical, honest.`;

  // User message: the item to judge
  const user = `Judge this ${input.item_type}:\n${JSON.stringify(input.context, null, 2)}`;

  // Assistant message: the judgment (what we're training to produce)
  const assistant = [
    `Q-Score: ${judgment.q_score}/100 — ${judgment.verdict}`,
    `Confidence: ${Math.round(judgment.confidence * 100)}%`,
    '',
    'Axiom Breakdown:',
    ...Object.entries(judgment.axiom_scores).map(([k, v]) => `  ${k}: ${v}`),
    '',
    judgment.reasoning_path.length > 0
      ? `Reasoning: ${judgment.reasoning_path.length} steps`
      : '',
    judgment.verdict === 'BARK' ? '*GROWL* High risk detected.' :
    judgment.verdict === 'GROWL' ? '*low growl* Caution advised.' :
    judgment.verdict === 'WAG' ? '*tail wag* Acceptable.' :
    '*ears perk* Strong signal.',
  ].filter(Boolean).join('\n');

  return {
    messages: [
      { role: 'system', content: system },
      { role: 'user', content: user },
      { role: 'assistant', content: assistant },
    ],
    reward,
    metadata: {
      judgment_id: record.id,
      feedback_outcome: feedback?.outcome || null,
    },
  };
}
