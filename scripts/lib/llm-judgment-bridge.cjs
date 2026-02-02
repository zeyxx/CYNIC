#!/usr/bin/env node
/**
 * LLM Judgment Bridge
 *
 * Connects open source LLMs to CYNIC's auto-judgment system.
 * Enables autonomous reasoning and self-improvement.
 *
 * "phi voit, phi juge, phi apprend"
 *
 * @module scripts/lib/llm-judgment-bridge
 */

'use strict';

const path = require('path');
const fs = require('fs');
const os = require('os');

// Constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;   // 61.8% max confidence
const PHI_INV_2 = 0.381966011250105; // 38.2%

// Default model - small, fast, fits in any RAM
const DEFAULT_MODEL = process.env.CYNIC_LLM_MODEL || 'gemma2:2b';
const OLLAMA_HOST = process.env.OLLAMA_HOST || 'http://localhost:11434';

// Consensus configuration
const CONSENSUS_MODELS = (process.env.CYNIC_CONSENSUS_MODELS || 'gemma2:2b,qwen2:0.5b').split(',');
const CONSENSUS_THRESHOLD = PHI_INV; // 61.8% agreement required

// AirLLM configuration (for large models via disk offloading)
const AIRLLM_ENABLED = process.env.CYNIC_AIRLLM === 'true';
const AIRLLM_MODEL = process.env.CYNIC_AIRLLM_MODEL || 'mistral:7b-instruct-q4_0';

// State
const STATE_DIR = path.join(os.homedir(), '.cynic');
const BRIDGE_STATE_FILE = path.join(STATE_DIR, 'llm-bridge.json');

// ═══════════════════════════════════════════════════════════════════════════
// STATE MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════

function ensureDir(dir) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function loadState() {
  ensureDir(STATE_DIR);
  try {
    if (fs.existsSync(BRIDGE_STATE_FILE)) {
      return JSON.parse(fs.readFileSync(BRIDGE_STATE_FILE, 'utf8'));
    }
  } catch (e) { /* ignore */ }
  return {
    available: null,
    lastCheck: null,
    model: DEFAULT_MODEL,
    stats: {
      calls: 0,
      successes: 0,
      failures: 0,
      totalLatencyMs: 0,
      tokensUsed: 0,
    },
    calibration: {
      accuracy: null,
      samples: 0,
    },
  };
}

function saveState(state) {
  ensureDir(STATE_DIR);
  fs.writeFileSync(BRIDGE_STATE_FILE, JSON.stringify(state, null, 2));
}

// ═══════════════════════════════════════════════════════════════════════════
// OLLAMA INTEGRATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Check if Ollama is available
 */
async function checkOllama() {
  const state = loadState();

  // Cache check for 5 minutes
  if (state.lastCheck && Date.now() - state.lastCheck < 300000) {
    return state.available;
  }

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 5000);

    const response = await fetch(`${OLLAMA_HOST}/api/tags`, {
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    if (response.ok) {
      const data = await response.json();
      const models = data.models?.map(m => m.name) || [];

      state.available = true;
      state.lastCheck = Date.now();
      state.availableModels = models;
      saveState(state);

      return true;
    }
  } catch (e) {
    state.available = false;
    state.lastCheck = Date.now();
    saveState(state);
  }

  return false;
}

/**
 * Call Ollama for completion
 */
async function callOllama(prompt, options = {}) {
  const model = options.model || DEFAULT_MODEL;
  const timeout = options.timeout || 30000;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeout);

  try {
    const startTime = Date.now();

    const response = await fetch(`${OLLAMA_HOST}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model,
        prompt,
        stream: false,
        options: {
          temperature: options.temperature || 0.3, // Low for consistency
          num_predict: options.maxTokens || 500,
        },
      }),
      signal: controller.signal,
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      throw new Error(`Ollama error: ${response.status}`);
    }

    const data = await response.json();
    const latencyMs = Date.now() - startTime;

    // Update stats
    const state = loadState();
    state.stats.calls++;
    state.stats.successes++;
    state.stats.totalLatencyMs += latencyMs;
    state.stats.tokensUsed += (data.prompt_eval_count || 0) + (data.eval_count || 0);
    saveState(state);

    return {
      text: data.response,
      latencyMs,
      tokens: {
        input: data.prompt_eval_count || 0,
        output: data.eval_count || 0,
      },
      model,
    };

  } catch (err) {
    clearTimeout(timeoutId);

    const state = loadState();
    state.stats.calls++;
    state.stats.failures++;
    saveState(state);

    if (err.name === 'AbortError') {
      throw new Error(`Ollama timeout after ${timeout}ms`);
    }
    throw err;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// JUDGMENT PROMPTS
// ═══════════════════════════════════════════════════════════════════════════

const JUDGMENT_SYSTEM_PROMPT = `You are CYNIC, a skeptical AI that judges code and decisions.

PHILOSOPHY:
- phi (golden ratio) guides all ratios
- Maximum confidence: 61.8% (phi^-1) - NEVER claim certainty
- Verdicts: HOWL (excellent), WAG (good), BARK (warning), GROWL (danger)

AXIOMS:
- PHI: Harmony and proportion in all things
- VERIFY: Don't trust, verify. Question everything.
- CULTURE: Patterns and consistency matter.
- BURN: Simplicity wins. Don't extract, burn.

OUTPUT FORMAT (JSON only):
{
  "score": 0-100,
  "verdict": "HOWL|WAG|BARK|GROWL",
  "reasoning": "Brief explanation",
  "confidence": 0.0-0.618,
  "axiomScores": {
    "PHI": 0-100,
    "VERIFY": 0-100,
    "CULTURE": 0-100,
    "BURN": 0-100
  }
}`;

const REFINEMENT_PROMPT = `You are reviewing a judgment for accuracy and bias.

ORIGINAL JUDGMENT:
{judgment}

CONTEXT:
{context}

Check for:
1. Overconfidence (>61.8% is always wrong)
2. Axiom imbalance (scores too far apart)
3. Missing context that would change verdict
4. Bias towards/against certain patterns

OUTPUT FORMAT (JSON only):
{
  "shouldRefine": true|false,
  "refinedScore": 0-100,
  "refinedVerdict": "HOWL|WAG|BARK|GROWL",
  "refinedConfidence": 0.0-0.618,
  "refinementReason": "Why refined (or why not)",
  "critiques": ["list", "of", "issues"]
}`;

const PATTERN_ANALYSIS_PROMPT = `Analyze this pattern for CYNIC's learning system.

PATTERN:
{pattern}

RECENT OBSERVATIONS:
{observations}

Determine:
1. Is this pattern significant?
2. Should CYNIC adjust its thresholds?
3. What should CYNIC learn from this?

OUTPUT FORMAT (JSON only):
{
  "significant": true|false,
  "significance": 0.0-1.0,
  "suggestedThresholdChange": null|{"category": "...", "delta": +/-N},
  "learning": "What CYNIC should remember",
  "confidence": 0.0-0.618
}`;

// ═══════════════════════════════════════════════════════════════════════════
// JUDGMENT FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Judge an item using LLM
 */
async function llmJudge(item, context = {}) {
  const available = await checkOllama();
  if (!available) {
    return { success: false, error: 'Ollama not available' };
  }

  const prompt = `${JUDGMENT_SYSTEM_PROMPT}

ITEM TO JUDGE:
${JSON.stringify(item, null, 2)}

${context.additionalContext ? `ADDITIONAL CONTEXT:\n${context.additionalContext}` : ''}

Respond with JSON only:`;

  try {
    const result = await callOllama(prompt, {
      temperature: 0.3,
      maxTokens: 500,
    });

    // Parse JSON from response
    const jsonMatch = result.text.match(/\{[\s\S]*\}/);
    if (!jsonMatch) {
      throw new Error('No JSON in response');
    }

    const judgment = JSON.parse(jsonMatch[0]);

    // Enforce phi constraints
    judgment.confidence = Math.min(judgment.confidence || 0.5, PHI_INV);
    judgment.score = Math.min(judgment.score || 50, 100);

    return {
      success: true,
      judgment: {
        ...judgment,
        source: 'llm',
        model: result.model,
        latencyMs: result.latencyMs,
      },
    };

  } catch (err) {
    return { success: false, error: err.message };
  }
}

/**
 * Refine a judgment using LLM
 */
async function llmRefine(judgment, context = {}) {
  const available = await checkOllama();
  if (!available) {
    return { success: false, shouldRefine: false, error: 'Ollama not available' };
  }

  const prompt = REFINEMENT_PROMPT
    .replace('{judgment}', JSON.stringify(judgment, null, 2))
    .replace('{context}', JSON.stringify(context, null, 2));

  try {
    const result = await callOllama(prompt, {
      temperature: 0.2, // Very low for refinement
      maxTokens: 400,
    });

    const jsonMatch = result.text.match(/\{[\s\S]*\}/);
    if (!jsonMatch) {
      throw new Error('No JSON in response');
    }

    const refinement = JSON.parse(jsonMatch[0]);

    // Enforce phi constraints
    if (refinement.refinedConfidence) {
      refinement.refinedConfidence = Math.min(refinement.refinedConfidence, PHI_INV);
    }

    return {
      success: true,
      ...refinement,
      source: 'llm',
      latencyMs: result.latencyMs,
    };

  } catch (err) {
    return { success: false, shouldRefine: false, error: err.message };
  }
}

/**
 * Analyze a pattern for learning
 */
async function llmAnalyzePattern(pattern, observations = []) {
  const available = await checkOllama();
  if (!available) {
    return { success: false, error: 'Ollama not available' };
  }

  const prompt = PATTERN_ANALYSIS_PROMPT
    .replace('{pattern}', JSON.stringify(pattern, null, 2))
    .replace('{observations}', JSON.stringify(observations.slice(-10), null, 2));

  try {
    const result = await callOllama(prompt, {
      temperature: 0.4,
      maxTokens: 400,
    });

    const jsonMatch = result.text.match(/\{[\s\S]*\}/);
    if (!jsonMatch) {
      throw new Error('No JSON in response');
    }

    const analysis = JSON.parse(jsonMatch[0]);

    // Enforce phi constraints
    analysis.confidence = Math.min(analysis.confidence || 0.5, PHI_INV);
    analysis.significance = Math.min(analysis.significance || 0.5, 1);

    return {
      success: true,
      ...analysis,
      source: 'llm',
      latencyMs: result.latencyMs,
    };

  } catch (err) {
    return { success: false, error: err.message };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONSENSUS JUDGMENT (Multi-LLM Agreement)
// "Le collectif juge" - Multiple models must agree (φ⁻¹ threshold)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Run consensus judgment with multiple models
 * Requires φ⁻¹ (61.8%) agreement on verdict
 */
async function llmConsensusJudge(item, context = {}) {
  const available = await checkOllama();
  if (!available) {
    return { success: false, error: 'Ollama not available' };
  }

  const models = context.models || CONSENSUS_MODELS;
  const results = [];
  const startTime = Date.now();

  // Run all models in parallel
  const promises = models.map(async (model) => {
    try {
      const prompt = `${JUDGMENT_SYSTEM_PROMPT}

ITEM TO JUDGE:
${JSON.stringify(item, null, 2)}

${context.additionalContext ? `ADDITIONAL CONTEXT:\n${context.additionalContext}` : ''}

Respond with JSON only:`;

      const result = await callOllama(prompt, {
        model,
        temperature: 0.3,
        maxTokens: 500,
        timeout: 45000, // Longer timeout for consensus
      });

      const jsonMatch = result.text.match(/\{[\s\S]*\}/);
      if (!jsonMatch) throw new Error('No JSON');

      const judgment = JSON.parse(jsonMatch[0]);
      judgment.model = model;
      judgment.latencyMs = result.latencyMs;

      return { success: true, judgment };
    } catch (err) {
      return { success: false, model, error: err.message };
    }
  });

  const responses = await Promise.all(promises);

  // Collect successful judgments
  const judgments = responses
    .filter(r => r.success)
    .map(r => r.judgment);

  if (judgments.length === 0) {
    return { success: false, error: 'No models responded successfully' };
  }

  // Calculate consensus
  const verdictCounts = {};
  let totalScore = 0;
  let totalConfidence = 0;
  const axiomTotals = { PHI: 0, VERIFY: 0, CULTURE: 0, BURN: 0 };

  for (const j of judgments) {
    verdictCounts[j.verdict] = (verdictCounts[j.verdict] || 0) + 1;
    totalScore += j.score || 50;
    totalConfidence += j.confidence || 0.5;

    if (j.axiomScores) {
      for (const [axiom, score] of Object.entries(j.axiomScores)) {
        if (axiomTotals[axiom] !== undefined) {
          axiomTotals[axiom] += score;
        }
      }
    }
  }

  // Find majority verdict
  const sortedVerdicts = Object.entries(verdictCounts)
    .sort((a, b) => b[1] - a[1]);

  const majorityVerdict = sortedVerdicts[0][0];
  const majorityCount = sortedVerdicts[0][1];
  const agreementRatio = majorityCount / judgments.length;

  // Check if consensus reached (φ⁻¹ threshold)
  const consensusReached = agreementRatio >= CONSENSUS_THRESHOLD;

  // Average scores
  const n = judgments.length;
  const avgScore = Math.round(totalScore / n);
  const avgConfidence = Math.min(totalConfidence / n, PHI_INV);
  const avgAxioms = {};
  for (const [axiom, total] of Object.entries(axiomTotals)) {
    avgAxioms[axiom] = Math.round(total / n);
  }

  // Boost confidence if strong consensus, reduce if weak
  let finalConfidence = avgConfidence;
  if (consensusReached) {
    // Stronger consensus = higher confidence (but never exceed φ⁻¹)
    finalConfidence = Math.min(avgConfidence * (1 + (agreementRatio - CONSENSUS_THRESHOLD)), PHI_INV);
  } else {
    // Weak consensus = lower confidence
    finalConfidence = avgConfidence * agreementRatio;
  }

  const totalLatencyMs = Date.now() - startTime;

  return {
    success: true,
    consensusReached,
    agreement: agreementRatio,
    threshold: CONSENSUS_THRESHOLD,
    judgment: {
      score: avgScore,
      verdict: majorityVerdict,
      confidence: Math.round(finalConfidence * 1000) / 1000,
      axiomScores: avgAxioms,
      reasoning: consensusReached
        ? `Consensus reached: ${majorityCount}/${n} models agree on ${majorityVerdict}`
        : `No consensus: ${majorityCount}/${n} models chose ${majorityVerdict} (need ${Math.ceil(n * CONSENSUS_THRESHOLD)})`,
      source: 'consensus',
      models: judgments.map(j => j.model),
      latencyMs: totalLatencyMs,
    },
    votes: judgments.map(j => ({
      model: j.model,
      verdict: j.verdict,
      score: j.score,
      latencyMs: j.latencyMs,
    })),
    dissent: sortedVerdicts.length > 1 ? sortedVerdicts.slice(1).map(([v, c]) => ({ verdict: v, count: c })) : [],
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// AIRLLM INTEGRATION (Disk offloading for large models)
// "Mistral 7B avec 28GB RAM" - Run larger models via disk offloading
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Check if AirLLM model is available
 * AirLLM uses Ollama but with specific quantization for disk offloading
 */
async function checkAirLLM() {
  if (!AIRLLM_ENABLED) {
    return { available: false, reason: 'AirLLM disabled (set CYNIC_AIRLLM=true)' };
  }

  const available = await checkOllama();
  if (!available) {
    return { available: false, reason: 'Ollama not running' };
  }

  // Check if the AirLLM model is available
  try {
    const response = await fetch(`${OLLAMA_HOST}/api/tags`);
    if (!response.ok) return { available: false, reason: 'Cannot list models' };

    const data = await response.json();
    const models = data.models?.map(m => m.name) || [];

    if (models.some(m => m.includes(AIRLLM_MODEL.split(':')[0]))) {
      return { available: true, model: AIRLLM_MODEL };
    }

    return {
      available: false,
      reason: `Model ${AIRLLM_MODEL} not found. Run: ollama pull ${AIRLLM_MODEL}`,
      availableModels: models,
    };
  } catch (e) {
    return { available: false, reason: e.message };
  }
}

/**
 * Run deep judgment with AirLLM (larger model, slower but better reasoning)
 */
async function airllmJudge(item, context = {}) {
  const airllmStatus = await checkAirLLM();
  if (!airllmStatus.available) {
    return { success: false, error: airllmStatus.reason };
  }

  // Use longer timeout for large model
  const timeout = context.timeout || 120000; // 2 minutes

  const prompt = `${JUDGMENT_SYSTEM_PROMPT}

DEEP ANALYSIS REQUESTED - Take your time to reason carefully.

ITEM TO JUDGE:
${JSON.stringify(item, null, 2)}

${context.additionalContext ? `ADDITIONAL CONTEXT:\n${context.additionalContext}` : ''}

Provide thorough reasoning, then output JSON:`;

  try {
    const result = await callOllama(prompt, {
      model: AIRLLM_MODEL,
      temperature: 0.2, // Lower for consistency
      maxTokens: 1000,  // More tokens for deeper reasoning
      timeout,
    });

    const jsonMatch = result.text.match(/\{[\s\S]*\}/);
    if (!jsonMatch) {
      throw new Error('No JSON in response');
    }

    const judgment = JSON.parse(jsonMatch[0]);

    // Enforce phi constraints
    judgment.confidence = Math.min(judgment.confidence || 0.5, PHI_INV);
    judgment.score = Math.min(judgment.score || 50, 100);

    return {
      success: true,
      judgment: {
        ...judgment,
        source: 'airllm',
        model: AIRLLM_MODEL,
        latencyMs: result.latencyMs,
        deepAnalysis: true,
      },
    };

  } catch (err) {
    return { success: false, error: err.message };
  }
}

/**
 * Hybrid judgment: Fast consensus + Deep AirLLM if no consensus
 * "Le chien doute, le chien approfondit"
 */
async function hybridJudge(item, context = {}) {
  // Step 1: Try fast consensus
  const consensusResult = await llmConsensusJudge(item, context);

  if (consensusResult.success && consensusResult.consensusReached) {
    // Strong consensus - return fast result
    return {
      ...consensusResult,
      method: 'consensus',
    };
  }

  // Step 2: No consensus - try deep analysis with AirLLM
  if (AIRLLM_ENABLED) {
    const airllmStatus = await checkAirLLM();
    if (airllmStatus.available) {
      const deepResult = await airllmJudge(item, {
        ...context,
        additionalContext: `${context.additionalContext || ''}

PREVIOUS CONSENSUS ATTEMPT (no agreement):
${JSON.stringify(consensusResult.votes, null, 2)}
Dissent: ${JSON.stringify(consensusResult.dissent)}

Please resolve the disagreement with careful analysis.`,
      });

      if (deepResult.success) {
        return {
          success: true,
          method: 'hybrid',
          consensusFailed: true,
          initialVotes: consensusResult.votes,
          judgment: {
            ...deepResult.judgment,
            resolvedBy: 'airllm_deep_analysis',
          },
        };
      }
    }
  }

  // Fallback: Return consensus result even if weak
  return {
    ...consensusResult,
    method: 'consensus_weak',
    warning: 'No strong consensus and AirLLM unavailable',
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// CALIBRATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Record feedback for calibration
 */
function recordFeedback(judgmentId, wasCorrect, correction = null) {
  const state = loadState();

  state.calibration.samples++;

  if (state.calibration.accuracy === null) {
    state.calibration.accuracy = wasCorrect ? 1 : 0;
  } else {
    // Exponential moving average (phi-weighted)
    const alpha = PHI_INV_2; // 38.2%
    state.calibration.accuracy =
      alpha * (wasCorrect ? 1 : 0) + (1 - alpha) * state.calibration.accuracy;
  }

  saveState(state);

  return {
    accuracy: state.calibration.accuracy,
    samples: state.calibration.samples,
  };
}

/**
 * Get bridge stats
 */
function getStats() {
  const state = loadState();
  return {
    available: state.available,
    model: state.model,
    stats: {
      ...state.stats,
      avgLatencyMs: state.stats.calls > 0
        ? Math.round(state.stats.totalLatencyMs / state.stats.calls)
        : 0,
      successRate: state.stats.calls > 0
        ? (state.stats.successes / state.stats.calls)
        : 0,
    },
    calibration: state.calibration,
    availableModels: state.availableModels || [],
  };
}

/**
 * Set model to use
 */
function setModel(model) {
  const state = loadState();
  state.model = model;
  saveState(state);
  return { success: true, model };
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

module.exports = {
  // Core functions
  checkOllama,
  callOllama,

  // Single-model judgment
  llmJudge,
  llmRefine,
  llmAnalyzePattern,

  // Consensus judgment (multi-LLM)
  llmConsensusJudge,
  CONSENSUS_THRESHOLD,
  CONSENSUS_MODELS,

  // AirLLM (large models via disk offloading)
  checkAirLLM,
  airllmJudge,
  AIRLLM_ENABLED,
  AIRLLM_MODEL,

  // Hybrid (consensus + deep analysis)
  hybridJudge,

  // Calibration
  recordFeedback,
  getStats,
  setModel,

  // Constants
  PHI_INV,
  DEFAULT_MODEL,
  OLLAMA_HOST,
};
