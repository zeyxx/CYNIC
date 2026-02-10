/**
 * Judgment Domain Tools
 *
 * Tools for CYNIC judgment operations:
 * - Judge: 25-dimension evaluation
 * - Refine: Self-refinement of judgments
 * - Feedback: Learning from corrections
 * - Learn: Pattern learning
 *
 * OCP: Domain module - implementations live here, index.js re-exports
 *
 * @module @cynic/mcp/tools/domains/judgment
 */

'use strict';

import {
  PHI_INV,
  PHI_INV_2,
  THRESHOLDS,
  createLogger,
} from '@cynic/core';
import { getEventBus, EventType } from '@cynic/node';

const log = createLogger('JudgmentTools');
import { enrichItem } from '../../item-enricher.js';
import { generateCard, toMarkdown, toASCII, toCompact } from '@cynic/node/judge';

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Profile level constants (Fibonacci)
 */
const PROFILE_LEVELS = {
  NOVICE: 1,
  APPRENTICE: 2,
  PRACTITIONER: 3,
  EXPERT: 5,
  MASTER: 8,
};

/**
 * Get axiom weight modifiers based on profile level
 * - NOVICE: VERIFY weight ↑ (more verification needed), BURN weight ↓ (less complexity)
 * - EXPERT: balanced (no adjustment)
 * - MASTER: CULTURE weight ↑ (pattern recognition), more risk allowed
 * @param {number} level - Profile level (1-8 Fibonacci)
 * @returns {Object} Weight modifiers { PHI, VERIFY, CULTURE, BURN }
 */
function getProfileWeightModifiers(level) {
  switch (level) {
    case PROFILE_LEVELS.NOVICE:
      return {
        PHI: 1.0,      // Normal - phi is universal
        VERIFY: 1.2,   // +20% - novices need more verification
        CULTURE: 0.9,  // -10% - less pattern weight
        BURN: 0.8,     // -20% - simpler solutions preferred
      };
    case PROFILE_LEVELS.APPRENTICE:
      return {
        PHI: 1.0,
        VERIFY: 1.1,   // +10%
        CULTURE: 0.95, // -5%
        BURN: 0.9,     // -10%
      };
    case PROFILE_LEVELS.PRACTITIONER:
      return {
        PHI: 1.0,
        VERIFY: 1.0,   // Balanced
        CULTURE: 1.0,
        BURN: 1.0,
      };
    case PROFILE_LEVELS.EXPERT:
      return {
        PHI: 1.0,
        VERIFY: 0.95,  // -5% - experts need less verification
        CULTURE: 1.1,  // +10% - pattern recognition valuable
        BURN: 1.05,    // +5% - can handle more complexity
      };
    case PROFILE_LEVELS.MASTER:
      return {
        PHI: 1.0,
        VERIFY: 0.9,   // -10% - trust master's judgment
        CULTURE: 1.2,  // +20% - deep pattern recognition
        BURN: 1.1,     // +10% - innovative solutions welcome
      };
    default:
      return { PHI: 1.0, VERIFY: 1.0, CULTURE: 1.0, BURN: 1.0 };
  }
}

/**
 * Apply profile weight modifiers to axiom scores
 * Normalizes result to stay within 0-100 range
 * @param {Object} axiomScores - Original axiom scores { PHI, VERIFY, CULTURE, BURN }
 * @param {Object} modifiers - Weight modifiers from getProfileWeightModifiers
 * @returns {Object} Adjusted axiom scores
 */
function applyProfileModifiers(axiomScores, modifiers) {
  const adjusted = {};
  for (const [axiom, score] of Object.entries(axiomScores)) {
    const modifier = modifiers[axiom] || 1.0;
    // Apply modifier and clamp to 0-100
    adjusted[axiom] = Math.min(100, Math.max(0, Math.round(score * modifier * 10) / 10));
  }
  return adjusted;
}

/**
 * Recalculate Q-Score from adjusted axiom scores
 * Q = 100 × ∜(φ × V × C × B / 100^4)
 * @param {Object} axiomScores - Adjusted axiom scores
 * @returns {number} Q-Score
 */
function recalculateQScore(axiomScores) {
  const PHI = axiomScores.PHI || 50;
  const VERIFY = axiomScores.VERIFY || 50;
  const CULTURE = axiomScores.CULTURE || 50;
  const BURN = axiomScores.BURN || 50;

  const product = (PHI * VERIFY * CULTURE * BURN) / Math.pow(100, 4);
  return Math.round(100 * Math.pow(product, 0.25) * 10) / 10;
}

/**
 * Calculate psychology composite states from dimensions and emotions
 * @param {Object} dimensions - Core dimensions
 * @param {Object} emotions - Emotional states
 * @returns {Object} Composite states
 */
function calculatePsychologyComposites(dimensions = {}, emotions = {}) {
  const d = dimensions;
  const e = emotions;

  // Extract values with defaults
  const energy = d.energy?.value ?? PHI_INV;
  const focus = d.focus?.value ?? PHI_INV;
  const creativity = d.creativity?.value ?? PHI_INV;
  const frustration = d.frustration?.value ?? PHI_INV_2;
  const curiosity = e.curiosity?.value ?? PHI_INV;
  const boredom = e.boredom?.value ?? 0;
  const riskAppetite = d.riskAppetite?.value ?? PHI_INV;

  return {
    // Flow: High energy + focus + creativity, low frustration
    flow: energy > PHI_INV && focus > PHI_INV && creativity > PHI_INV_2 && frustration < PHI_INV_2,

    // Burnout risk: Low energy + high frustration
    burnoutRisk: energy < PHI_INV_2 && frustration > PHI_INV,

    // Exploration: High curiosity + risk appetite
    exploration: curiosity > PHI_INV && riskAppetite > PHI_INV_2,

    // Grind: Low creativity, moderate focus (mechanical work)
    grind: creativity < PHI_INV_2 && focus > PHI_INV_2 && energy > PHI_INV_2,

    // Procrastination: Low focus, high boredom
    procrastination: focus < PHI_INV_2 && boredom > PHI_INV_2,

    // Breakthrough potential: High creativity + curiosity
    breakthrough: creativity > PHI_INV && curiosity > PHI_INV,
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TOOL IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create judge tool definition
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} [persistence] - PersistenceManager instance (for storing judgments)
 * @param {Object} [sessionManager] - SessionManager instance (for user/session context)
 * @param {Object} [pojChainManager] - PoJChainManager instance (for blockchain)
 * @param {Object} [graphIntegration] - JudgmentGraphIntegration instance (for graph edges)
 * @param {Function} [onJudgment] - Callback when judgment is completed (for SSE broadcast)
 * @param {Object} [burnEnforcer] - BurnEnforcer instance (for requiring burns)
 * @param {Object} [emergenceLayer] - EmergenceLayer instance (Layer 7 - consciousness, patterns, dimensions)
 * @param {Object} [thermodynamics] - ThermodynamicsTracker instance (Phase 2 - heat/work/efficiency)
 * @returns {Object} Tool definition
 */
export function createJudgeTool(judge, persistence = null, sessionManager = null, pojChainManager = null, graphIntegration = null, onJudgment = null, burnEnforcer = null, emergenceLayer = null, thermodynamics = null) {
  return {
    name: 'brain_cynic_judge',
    description: `Judge an item using CYNIC's 25-dimension evaluation across 4 axioms (PHI, VERIFY, CULTURE, BURN). Returns Q-Score (0-100), verdict (HOWL/WAG/GROWL/BARK), confidence (max ${(PHI_INV * 100).toFixed(1)}%), and dimension breakdown.`,
    inputSchema: {
      type: 'object',
      properties: {
        item: {
          type: 'object',
          description: 'The item to judge. Can contain: type, content, sources, verified, scores (explicit dimension scores)',
        },
        context: {
          type: 'object',
          description: 'Optional context: source, type, kScore (for Final score calculation)',
        },
      },
      required: ['item'],
    },
    handler: async (params) => {
      const { item, context = {} } = params;
      if (!item) throw new Error('Missing required parameter: item');

      // ═══════════════════════════════════════════════════════════════════════════
      // BURN ENFORCEMENT: Require valid burn before judgment
      // "No burn, no judgment" - κυνικός
      // ═══════════════════════════════════════════════════════════════════════════
      if (burnEnforcer) {
        const sessionContext = sessionManager?.getSessionContext() || {};
        const userId = sessionContext.userId || 'anonymous';
        burnEnforcer.requireBurn(userId, 'judge');
      }

      // Enrich item with metadata for richer judgment
      // This extracts sources, analyzes code/text, generates hashes, etc.
      const enrichedItem = enrichItem(item, context);

      // Use graph integration if available, otherwise direct judge
      // FIX J1: Use judgeAsync to enable 73 philosophy engines consultation
      const judgment = graphIntegration
        ? await graphIntegration.judgeWithGraph(enrichedItem, context)
        : await judge.judgeAsync(enrichedItem, context, {
            consultEngines: true,
            maxEngines: 5,
            engineTimeout: 3000,
          });

      // Generate fallback ID (used if persistence unavailable)
      let judgmentId = `jdg_${Date.now().toString(36)}`;

      // Get session context for user isolation
      const sessionContext = sessionManager?.getSessionContext() || {};

      // Store judgment in persistence FIRST to get the real DB ID
      if (persistence) {
        try {
          const stored = await persistence.storeJudgment({
            item: enrichedItem,
            itemType: enrichedItem.type || 'unknown',
            itemContent: typeof enrichedItem.content === 'string' ? enrichedItem.content : JSON.stringify(enrichedItem),
            qScore: judgment.qScore,
            globalScore: judgment.global_score,
            confidence: Math.round(judgment.confidence * 1000) / 1000,
            verdict: judgment.qVerdict?.verdict || judgment.verdict,
            axiomScores: judgment.axiomScores,
            dimensionScores: judgment.dimensionScores || null,
            weaknesses: judgment.weaknesses,
            context,
            // Reasoning trajectory for training pipeline + DB trigger extraction
            reasoningPath: judgment.reasoning_path || [],
            // Session context for multi-user isolation
            userId: sessionContext.userId || null,
            sessionId: sessionContext.sessionId || null,
          });

          // Use the DB-generated ID for consistency
          if (stored?.judgment_id) {
            judgmentId = stored.judgment_id;
          }

          // Add to PoJ chain (batched block creation)
          if (pojChainManager && stored) {
            await pojChainManager.addJudgment({
              judgment_id: stored.judgment_id,
              q_score: judgment.qScore,
              verdict: judgment.qVerdict?.verdict || judgment.verdict,
              created_at: stored.created_at,
            });
          }

          // Increment session counter
          if (sessionManager) {
            await sessionManager.incrementCounter('judgmentCount');
          }

          // Extract and persist patterns from judgment (for emergence detection)
          try {
            const itemType = item.type || 'unknown';
            const verdict = judgment.qVerdict?.verdict || judgment.verdict;
            const score = judgment.qScore;

            // 1. Verdict distribution pattern
            await persistence.upsertPattern({
              category: 'verdict',
              name: `verdict_${verdict.toLowerCase()}`,
              description: `Items receiving ${verdict} verdict`,
              confidence: judgment.confidence,
              sourceJudgments: [judgmentId],
              tags: ['verdict', verdict.toLowerCase()],
              data: { verdict, avgScore: score },
            });

            // 2. Item type pattern
            await persistence.upsertPattern({
              category: 'item_type',
              name: `type_${itemType}`,
              description: `Patterns for ${itemType} items`,
              confidence: judgment.confidence,
              sourceJudgments: [judgmentId],
              tags: ['item_type', itemType],
              data: { itemType, avgScore: score, verdict },
            });

            // 3. Weakness patterns (for learning)
            if (judgment.weaknesses?.length > 0) {
              for (const weakness of judgment.weaknesses.slice(0, 3)) {
                await persistence.upsertPattern({
                  category: 'weakness',
                  name: `weakness_${weakness.axiom?.toLowerCase() || 'unknown'}`,
                  description: weakness.reason,
                  confidence: 0.5,
                  sourceJudgments: [judgmentId],
                  tags: ['weakness', weakness.axiom?.toLowerCase()].filter(Boolean),
                  data: { axiom: weakness.axiom, reason: weakness.reason },
                });
              }
            }

            // 4. Anomaly detection (scores outside normal range)
            const isAnomaly = score < THRESHOLDS.ANOMALY_LOW || score > THRESHOLDS.ANOMALY_HIGH;
            if (isAnomaly) {
              const isLow = score < THRESHOLDS.ANOMALY_LOW;
              await persistence.upsertPattern({
                category: 'anomaly',
                name: `anomaly_${isLow ? 'low' : 'high'}_score`,
                description: `Unusual ${isLow ? 'low' : 'high'} score detected`,
                confidence: 0.6181,  // Must be >= φ⁻¹ (0.6181 survives DECIMAL(5,4) rounding)
                sourceJudgments: [judgmentId],
                tags: ['anomaly', isLow ? 'low_score' : 'high_score'],
                data: { score, verdict, itemType, threshold: isLow ? THRESHOLDS.ANOMALY_LOW : THRESHOLDS.ANOMALY_HIGH },
              });
            }
          } catch (patternErr) {
            // Non-blocking - pattern extraction is best-effort
            log.warn('Pattern extraction error', { error: patternErr.message });
          }
        } catch (e) {
          // Log but don't fail the judgment - persistence is best-effort
          log.error('Error persisting judgment', { error: e.message });
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // EMERGENCE LAYER (Layer 7 - Keter): The crown observes all
      // Feeds: ConsciousnessMonitor, PatternDetector, DimensionDiscovery
      // "Le cerveau s'éveille" - κυνικός
      // ═══════════════════════════════════════════════════════════════════════════
      let emergenceResult = null;
      if (emergenceLayer) {
        try {
          const verdict = judgment.qVerdict?.verdict || judgment.verdict;
          // observeJudgment feeds all Layer 7 components:
          // - ConsciousnessMonitor: tracks confidence, state, awareness
          // - PatternDetector: detects sequences, anomalies, trends
          // - DimensionDiscovery: analyzes axiom scores for new dimensions
          emergenceResult = emergenceLayer.observeJudgment({
            verdict,
            q_score: judgment.qScore,
            axiom_scores: judgment.axiomScores,
            dimension_scores: judgment.dimensionScores,
            confidence: judgment.confidence,
            raw_assessment: enrichedItem.content,
          });

          log.debug('Emergence layer observation', {
            consciousnessState: emergenceResult?.consciousness,
            patternsDetected: emergenceResult?.patternsDetected,
            awarenessLevel: emergenceLayer.consciousness?.awarenessLevel,
          });
        } catch (emergenceErr) {
          // Non-blocking - emergence observation is best-effort
          log.warn('EmergenceLayer observation error', { error: emergenceErr.message });
        }
      }

      // Build response with consistent ID
      // Debug logging for CULTURE score tracking
      log.debug('CULTURE score check', {
        cultureScore: judgment.axiomScores?.CULTURE,
        qScore: judgment.qScore,
        axiomScores: judgment.axiomScores,
      });

      // ═══════════════════════════════════════════════════════════════════════════
      // CONSCIOUSNESS-ADJUSTED CONFIDENCE
      // "Plus CYNIC est conscient, plus il peut être confiant"
      // When awakening: confidence scaled down. When aware: full confidence available.
      // ═══════════════════════════════════════════════════════════════════════════
      let adjustedConfidence = judgment.confidence;
      let consciousnessState = 'UNKNOWN';
      let awarenessLevel = 0;

      if (emergenceLayer?.consciousness) {
        awarenessLevel = emergenceLayer.consciousness.awarenessLevel || 0;
        consciousnessState = emergenceLayer.consciousness.state || 'DORMANT';

        // Scale confidence based on awareness:
        // - DORMANT (0): confidence capped at 50% of normal max
        // - AWAKENING (0.236): confidence at ~62% of normal max
        // - AWARE (0.382): confidence at ~69% of normal max
        // - HEIGHTENED (0.618): confidence at ~81% of normal max
        // - TRANSCENDENT (1.0): confidence at full normal max (PHI_INV)
        const awarenessScale = 0.5 + (awarenessLevel * 0.5);
        const maxAllowedConfidence = PHI_INV * awarenessScale;
        adjustedConfidence = Math.min(judgment.confidence, maxAllowedConfidence);

        log.debug('Consciousness-adjusted confidence', {
          originalConfidence: judgment.confidence,
          awarenessLevel,
          awarenessScale,
          maxAllowedConfidence,
          adjustedConfidence,
          consciousnessState,
        });
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // THERMODYNAMICS (Phase 2): Track heat/work and adjust confidence
      // "Ἐνέργεια - the activity of being" - κυνικός
      // Success → Work, Error/GROWL → Heat
      // Critical/Low efficiency → Lower confidence ceiling
      // ═══════════════════════════════════════════════════════════════════════════
      let thermoState = null;
      let thermoRecommendation = null;

      if (thermodynamics) {
        try {
          const verdict = judgment.qVerdict?.verdict || judgment.verdict;
          const qScore = judgment.qScore;

          // Record work or heat based on verdict and score
          if (verdict === 'HOWL' || verdict === 'WAG') {
            // Success: Record work proportional to Q-Score
            thermodynamics.recordWork('judgment', qScore / 100);
          } else if (verdict === 'GROWL' || verdict === 'BARK') {
            // Error/Warning: Record heat proportional to how bad
            thermodynamics.recordHeat('judgment', (100 - qScore) / 100);
          } else {
            // Neutral: Small work, proportional to score
            thermodynamics.recordWork('judgment', qScore / 200);
          }

          // Get current thermodynamic state
          thermoState = thermodynamics.getState();
          thermoRecommendation = thermodynamics.getRecommendation();

          // Apply thermodynamic confidence modifier
          // When critical or low efficiency, lower confidence ceiling further
          if (thermoRecommendation.confidenceModifier < 1.0) {
            const thermoAdjustedConfidence = adjustedConfidence * thermoRecommendation.confidenceModifier;
            log.debug('Thermodynamics-adjusted confidence', {
              beforeThermo: adjustedConfidence,
              modifier: thermoRecommendation.confidenceModifier,
              level: thermoRecommendation.level,
              afterThermo: thermoAdjustedConfidence,
              heat: thermoState.heat,
              efficiency: thermoState.efficiency,
            });
            adjustedConfidence = thermoAdjustedConfidence;
          }
        } catch (thermoErr) {
          // Non-blocking - thermodynamics is best-effort
          log.warn('Thermodynamics tracking error', { error: thermoErr.message });
        }
      }

      const result = {
        requestId: judgmentId,
        score: judgment.qScore,
        globalScore: judgment.global_score,
        verdict: judgment.qVerdict?.verdict || judgment.verdict,
        confidence: Math.round(adjustedConfidence * 1000) / 1000,
        axiomScores: judgment.axiomScores,
        weaknesses: judgment.weaknesses,
        finalScore: judgment.finalScore || null,
        phi: { maxConfidence: PHI_INV, minDoubt: PHI_INV_2 },
        // Layer 7 (Keter) consciousness state
        consciousness: emergenceLayer ? {
          state: consciousnessState,
          awarenessLevel: Math.round(awarenessLevel * 1000) / 1000,
          patternsDetected: emergenceResult?.patternsDetected || 0,
        } : null,
        // Thermodynamics state (Phase 2)
        thermodynamics: thermoState ? {
          heat: thermoState.heat,
          work: thermoState.work,
          efficiency: thermoState.efficiency,
          temperature: thermoState.temperature,
          isCritical: thermoState.isCritical,
          recommendation: thermoRecommendation?.level,
        } : null,
        timestamp: Date.now(),
      };

      // ═══════════════════════════════════════════════════════════════════════════
      // PSYCHOLOGY BIDIRECTIONAL (Phase 3): Human profile influences judgment
      // "L'humain complète CYNIC, CYNIC complète l'humain"
      // Profile level adjusts dimension weights for more personalized judgments
      // ═══════════════════════════════════════════════════════════════════════════
      let profileLevel = PROFILE_LEVELS.PRACTITIONER; // Default
      let profileAdjusted = false;

      if (persistence && sessionContext.userId) {
        try {
          const psychology = await persistence.loadPsychology(sessionContext.userId);
          if (psychology) {
            // Extract profile level from psychology state
            profileLevel = psychology.profileLevel || psychology.level || PROFILE_LEVELS.PRACTITIONER;

            // Get weight modifiers based on profile level
            const modifiers = getProfileWeightModifiers(profileLevel);

            // Check if modifiers are non-default (any modifier != 1.0)
            const hasModifiers = Object.values(modifiers).some(m => m !== 1.0);

            if (hasModifiers && result.axiomScores) {
              // Apply profile-based weight modifiers to axiom scores
              const originalAxiomScores = { ...result.axiomScores };
              const adjustedAxiomScores = applyProfileModifiers(result.axiomScores, modifiers);
              const adjustedQScore = recalculateQScore(adjustedAxiomScores);

              // Only apply if change is significant (> 1 point difference)
              if (Math.abs(adjustedQScore - result.score) > 1) {
                result.axiomScores = adjustedAxiomScores;
                result.score = adjustedQScore;
                result.verdict = adjustedQScore >= 80 ? 'HOWL' :
                                 adjustedQScore >= 50 ? 'WAG' :
                                 adjustedQScore >= 38.2 ? 'GROWL' : 'BARK';
                profileAdjusted = true;

                log.debug('Profile-adjusted judgment', {
                  profileLevel,
                  profileLevelName: ['', 'Novice', 'Apprentice', 'Practitioner', '', 'Expert', '', '', 'Master'][profileLevel] || 'Unknown',
                  modifiers,
                  originalScore: judgment.qScore,
                  adjustedScore: adjustedQScore,
                  originalAxiomScores,
                  adjustedAxiomScores,
                });
              }
            }

            // Add psychology context (informative + profile influence info)
            result.humanContext = {
              profileLevel,
              profileLevelName: ['', 'Novice', 'Apprentice', 'Practitioner', '', 'Expert', '', '', 'Master'][profileLevel] || 'Unknown',
              profileAdjusted,
              energy: psychology.dimensions?.energy?.value,
              focus: psychology.dimensions?.focus?.value,
              frustration: psychology.dimensions?.frustration?.value,
              composites: calculatePsychologyComposites(psychology.dimensions, psychology.emotions),
              sessionCount: psychology.sessionCount,
            };
          }
        } catch (e) {
          // Non-blocking - psychology context is optional
          log.debug('Psychology loading skipped', { error: e.message });
        }
      }

      // Include graph edge info if available
      if (judgment.graphEdge) {
        result.graphEdge = judgment.graphEdge;
      }

      // Emit event for SSE broadcast
      if (onJudgment) {
        try {
          onJudgment({
            judgmentId,
            qScore: result.score,
            verdict: result.verdict,
            confidence: result.confidence,
            itemType: item.type || 'unknown',
            timestamp: result.timestamp,
          });
        } catch (e) {
          // Non-blocking - don't fail judgment for callback errors
          log.warn('Judgment callback error', { error: e.message });
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // JUDGMENT CARD: Generate shareable artifact
      // "Le jugement visible" - distribution layer for CYNIC judgments
      // ═══════════════════════════════════════════════════════════════════════════
      try {
        const cardInput = {
          ...result,
          itemType: enrichedItem.type || enrichedItem.itemType || item.type || 'unknown',
          item: enrichedItem,
        };
        const card = generateCard(cardInput, {
          title: enrichedItem.name || enrichedItem.type || item.name || item.type || 'Item',
        });
        result.card = {
          markdown: card.markdown,
          ascii: card.ascii,
          compact: card.compact,
        };
      } catch (cardErr) {
        log.warn('Card generation error', { error: cardErr.message });
      }

      log.debug('Handler return', {
        cultureScore: result.axiomScores?.CULTURE,
        axiomScores: result.axiomScores,
        hasCard: !!result.card,
      });

      return result;
    },
  };
}

/**
 * Create self-refinement tool definition
 * @param {Object} judge - Judge instance
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createRefineTool(judge, persistence = null) {
  // Dynamic import to avoid circular dependencies
  let refinement = null;

  return {
    name: 'brain_cynic_refine',
    description: 'Self-refine a judgment by critiquing and re-judging. CYNIC critiques its own judgment, identifies weaknesses, and suggests improved scores. Returns original vs refined comparison.',
    inputSchema: {
      type: 'object',
      properties: {
        judgmentId: {
          type: 'string',
          description: 'ID of a previous judgment to refine (e.g., jdg_abc123)',
        },
        judgment: {
          type: 'object',
          description: 'Or provide judgment directly: { Q, breakdown: { PHI, VERIFY, CULTURE, BURN }, verdict }',
        },
        context: {
          type: 'object',
          description: 'Optional context for critique (source, type, etc.)',
        },
        maxIterations: {
          type: 'number',
          description: 'Max refinement iterations (default: 3)',
          default: 3,
        },
      },
    },
    handler: async (params) => {
      const { judgmentId, judgment: directJudgment, context = {}, maxIterations = 3 } = params;

      // Lazy load refinement module
      if (!refinement) {
        try {
          const core = await import('@cynic/core');
          refinement = {
            critiqueJudgment: core.critiqueJudgment,
            selfRefine: core.selfRefine,
            extractLearning: core.extractLearning,
          };
        } catch (e) {
          throw new Error(`Refinement module not available: ${e.message}`);
        }
      }

      let judgment = directJudgment;

      // If judgmentId provided, fetch from persistence
      if (judgmentId && !judgment) {
        if (!persistence) {
          throw new Error('Persistence required to fetch judgment by ID');
        }
        const stored = await persistence.getJudgment(judgmentId);
        if (!stored) {
          throw new Error(`Judgment not found: ${judgmentId}`);
        }
        judgment = {
          Q: stored.q_score,
          qScore: stored.q_score,
          verdict: stored.verdict,
          breakdown: stored.axiom_scores || stored.axiomScores,
          axiomScores: stored.axiom_scores || stored.axiomScores,
          confidence: stored.confidence,
        };
      }

      if (!judgment) {
        throw new Error('Must provide either judgmentId or judgment object');
      }

      // Normalize judgment format
      const normalizedJudgment = {
        Q: judgment.Q || judgment.qScore || judgment.q_score,
        qScore: judgment.Q || judgment.qScore || judgment.q_score,
        breakdown: judgment.breakdown || judgment.axiomScores || judgment.axiom_scores,
        verdict: judgment.verdict,
        confidence: judgment.confidence,
      };

      // Run self-refinement
      const result = refinement.selfRefine(normalizedJudgment, context, { maxIterations });

      // Extract learnings
      const learnings = refinement.extractLearning(result);

      // Track refinement for selfCorrection emergence indicator
      if (judge) {
        judge.refinementCount = (judge.refinementCount || 0) + 1;
      }

      // Store learnings if improved
      if (persistence && learnings.improved) {
        try {
          // Store as pattern for future reference (using upsertPattern)
          for (const learning of learnings.learnings) {
            await persistence.upsertPattern({
              category: 'refinement',
              name: `refinement_${learning.type || 'general'}`,
              description: learning.pattern || learning.correction,
              confidence: 0.6181,  // Must be >= φ⁻¹ (0.6181 survives DECIMAL(5,4) rounding)
              sourceJudgments: [judgmentId || 'direct'],
              tags: ['refinement', learning.type, learning.axiom].filter(Boolean),
              data: { learning, improved: result.improved, improvement: result.totalImprovement },
            });
          }

          // Track self-correction pattern for emergence
          await persistence.upsertPattern({
            category: 'self_correction',
            name: 'self_correction_event',
            description: `CYNIC self-corrected judgment with ${result.totalImprovement}% improvement`,
            confidence: 0.7,
            sourceJudgments: [judgmentId || 'direct'],
            tags: ['self_correction', 'emergence', 'meta_cognition'],
            data: {
              improvement: result.totalImprovement,
              iterations: result.iterationCount,
              originalScore: result.original?.Q,
              refinedScore: result.final?.Q,
            },
          });
        } catch (e) {
          // Non-blocking
          log.error('Error storing refinement learnings', { error: e.message });
        }
      }

      return {
        original: result.original,
        refined: result.final,
        improved: result.improved,
        improvement: result.totalImprovement,
        iterations: result.iterationCount,
        critiques: result.iterations.map(i => ({
          iteration: i.iteration,
          issues: i.critique?.critiques?.length || 0,
          severity: i.critique?.severity,
          adjustments: i.refinement?.adjustments?.length || 0,
        })),
        learnings: learnings.learnings,
        meta: {
          judgmentId: judgmentId || 'direct',
          maxIterations,
          timestamp: Date.now(),
        },
      };
    },
  };
}

/**
 * Create feedback tool definition
 * @param {Object} persistence - PersistenceManager instance
 * @param {Object} sessionManager - SessionManager instance
 * @returns {Object} Tool definition
 */
export function createFeedbackTool(persistence = null, sessionManager = null) {
  return {
    name: 'brain_cynic_feedback',
    description: 'Provide feedback on a past judgment to improve CYNIC learning. Mark judgments as correct/incorrect with reasoning.',
    inputSchema: {
      type: 'object',
      properties: {
        judgmentId: { type: 'string', description: 'ID of judgment to provide feedback on (e.g., jdg_abc123)' },
        outcome: { type: 'string', enum: ['correct', 'incorrect', 'partial'], description: 'Was the judgment correct?' },
        reason: { type: 'string', description: 'Explanation of why the judgment was correct/incorrect' },
        actualScore: { type: 'number', description: 'What the score should have been (0-100)' },
      },
      required: ['judgmentId', 'outcome'],
    },
    handler: async (params) => {
      const { judgmentId, outcome, reason = '', actualScore } = params;
      if (!judgmentId) throw new Error('Missing required parameter: judgmentId');
      if (!outcome) throw new Error('Missing required parameter: outcome');

      // Get session context for user isolation
      const sessionContext = sessionManager?.getSessionContext() || {};

      const feedback = {
        feedbackId: `fb_${Date.now().toString(36)}`,
        judgmentId,
        outcome,
        reason,
        actualScore,
        timestamp: Date.now(),
        // Session context for multi-user isolation
        userId: sessionContext.userId || null,
        sessionId: sessionContext.sessionId || null,
      };

      let learningDelta = null;
      let originalScore = null;

      // Store feedback (PersistenceManager handles PostgreSQL → File → Memory fallback)
      if (persistence) {
        try {
          await persistence.storeFeedback({
            judgmentId,
            outcome,
            reason,
            actualScore,
            // Session context for multi-user isolation
            userId: sessionContext.userId || null,
            sessionId: sessionContext.sessionId || null,
          });

          // Increment session counter
          if (sessionManager) {
            await sessionManager.incrementCounter('feedbackCount');
          }

          // Try to get original judgment for delta calculation and itemType
          const judgments = await persistence.getRecentJudgments(100);
          const original = judgments.find(j => j.judgment_id === judgmentId);
          if (original) {
            // Include itemType in feedback for user learning profile patterns
            feedback.itemType = original.item_type;

            if (typeof actualScore === 'number') {
              originalScore = original.q_score;
              learningDelta = actualScore - originalScore;
            }
          }
        } catch (e) {
          log.error('Error storing feedback', { error: e.message });
        }
      }

      return {
        ...feedback,
        learningDelta,
        originalScore,
        message: `Feedback recorded for ${judgmentId}. ${outcome === 'correct' ? '*wag*' : outcome === 'incorrect' ? '*growl* Learning...' : '*sniff* Partially noted.'}`,
      };
    },
  };
}

/**
 * Create learning tool definition
 * @param {Object} options - Options
 * @param {Object} [options.learningService] - LearningService instance
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createLearningTool(options = {}) {
  const { learningService, persistence } = options;

  // Use provided learning service or create a basic one
  let service = learningService;

  return {
    name: 'brain_learning',
    description: `Learning service that improves CYNIC's judgments based on feedback.
Supports external validation sources (Ralph-inspired):
Actions:
- feedback: Process manual feedback on a judgment (correct/incorrect/partial)
- test_result: Process test pass/fail as feedback (source: test_result)
- commit_result: Process commit success/failure as feedback (source: commit)
- pr_result: Process PR merged/rejected as feedback (source: pr_merged/pr_rejected)
- build_result: Process build success/failure as feedback (source: build)
- learn: Trigger weight learning from accumulated feedback
- patterns: Get learned patterns and insights
- state: Get current learning state
- export: Export learning state as markdown
- import: Import learning state from data
- reset: Reset learning state`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['feedback', 'test_result', 'commit_result', 'pr_result', 'build_result', 'learn', 'patterns', 'state', 'export', 'import', 'reset'],
          description: 'Action to perform',
        },
        judgmentId: {
          type: 'string',
          description: 'Judgment ID for feedback',
        },
        outcome: {
          type: 'string',
          enum: ['correct', 'incorrect', 'partial'],
          description: 'Feedback outcome (for manual feedback)',
        },
        actualScore: {
          type: 'number',
          description: 'What the Q-Score should have been (0-100)',
        },
        originalScore: {
          type: 'number',
          description: 'Original Q-Score from judgment',
        },
        itemType: {
          type: 'string',
          description: 'Type of item judged (code, config, etc.)',
        },
        context: {
          type: 'object',
          description: 'Additional context (source-specific data)',
        },
        // Test result specific
        passed: {
          type: 'boolean',
          description: 'Whether tests passed (for test_result)',
        },
        testSuite: {
          type: 'string',
          description: 'Test suite name (for test_result)',
        },
        passCount: {
          type: 'number',
          description: 'Number of tests passed (for test_result)',
        },
        failCount: {
          type: 'number',
          description: 'Number of tests failed (for test_result)',
        },
        // Commit result specific
        success: {
          type: 'boolean',
          description: 'Whether operation succeeded (for commit/build)',
        },
        commitHash: {
          type: 'string',
          description: 'Git commit hash (for commit_result)',
        },
        hooksPassed: {
          type: 'boolean',
          description: 'Whether pre-commit hooks passed (for commit_result)',
        },
        // PR result specific
        status: {
          type: 'string',
          enum: ['merged', 'rejected', 'open'],
          description: 'PR status (for pr_result)',
        },
        prNumber: {
          type: 'string',
          description: 'PR number (for pr_result)',
        },
        approvalCount: {
          type: 'number',
          description: 'Number of approvals (for pr_result)',
        },
        // Import specific
        data: {
          type: 'object',
          description: 'Learning state data to import',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action } = params;

      // Lazy-load if no service provided
      if (!service) {
        try {
          const { LearningService } = await import('@cynic/node');
          service = new LearningService({
            persistence, // ← Task #56: Connect to FeedbackRepository via persistence
            learningRate: 0.236,
            decayRate: 0.146,
            minFeedback: 5,
          });

          // Try to load existing state from persistence
          if (persistence) {
            try {
              const state = await persistence.getLearningState();
              if (state) {
                service.import(state);
              }
            } catch (e) {
              // No saved state, start fresh
            }
          }
        } catch (e) {
          throw new Error(`Learning service not available: ${e.message}`);
        }
      }

      // Helper to save state after modifications
      const saveState = async () => {
        if (persistence) {
          try {
            await persistence.saveLearningState(service.export());
          } catch (e) {
            // Non-blocking
          }
        }
      };

      // Helper to get judgment by ID
      const getJudgment = async (judgmentId) => {
        if (!judgmentId || !persistence) return null;
        try {
          const stored = await persistence.getJudgment(judgmentId);
          return stored ? stored.q_score : null;
        } catch (e) {
          return null;
        }
      };

      switch (action) {
        case 'state':
          return service.getState();

        case 'patterns':
          return service.getPatterns();

        case 'reset':
          service.reset();
          await saveState();
          return { reset: true, message: 'Learning state reset' };

        case 'learn': {
          const result = await service.learn();
          await saveState();
          return result;
        }

        case 'export':
          return {
            state: service.export(),
            markdown: service.exportToMarkdown(),
          };

        case 'import': {
          if (params.data) {
            service.import(params.data);
            await saveState();
            return { imported: true };
          }
          if (params.markdown) {
            service.importFromMarkdown(params.markdown);
            await saveState();
            return { imported: true, fromMarkdown: true };
          }
          throw new Error('data or markdown required for import');
        }

        case 'feedback': {
          // Tolerate outcome at top level OR nested under feedback object (digest.js pattern)
          const outcome = params.outcome || params.feedback?.outcome;
          if (!outcome) {
            throw new Error('outcome required for feedback action');
          }

          const feedbackData = params.feedback || {};
          const originalScore = params.originalScore || feedbackData.originalScore || await getJudgment(params.judgmentId) || 50;

          const result = service.processFeedback({
            judgmentId: params.judgmentId,
            outcome,
            actualScore: params.actualScore || feedbackData.actualScore,
            originalScore,
            itemType: params.itemType || feedbackData.itemType || 'unknown',
            source: feedbackData.source || 'manual',
            sourceContext: params.context || feedbackData.sourceContext || {},
          });

          await saveState();

          // Persist feedback to PostgreSQL for training pipeline
          // Now supports orphan feedback (no judgmentId required)
          if (persistence?.storeFeedback) {
            try {
              await persistence.storeFeedback({
                judgmentId: params.judgmentId || null,
                outcome,
                reason: params.context?.reason || feedbackData.sourceContext?.reason || `Manual feedback: ${outcome}`,
                actualScore: params.actualScore || feedbackData.actualScore || null,
                sourceType: feedbackData.source || 'manual',
                sourceContext: params.context || feedbackData.sourceContext || {},
              });
            } catch (e) {
              // Best-effort — don't block on persistence failure
            }
          }

          // ═══════════════════════════════════════════════════════════════════════
          // Task #84: Increment pattern frequency on feedback to boost Fisher scores
          // "φ renforce ce qui fonctionne" - feedback strengthens patterns
          // ═══════════════════════════════════════════════════════════════════════
          let patternsUpdated = 0;
          if (persistence?.query) {
            try {
              const feedbackType = params.context?.type || params.itemType || 'feedback';
              const isPositive = outcome === 'correct';

              // Increment frequency for related patterns (positive feedback boosts, negative doesn't)
              if (isPositive) {
                const updateResult = await persistence.query(`
                  UPDATE patterns
                  SET frequency = frequency + 1,
                      fisher_importance = LEAST(0.999,
                        fisher_importance + 0.01 * (1 - fisher_importance)
                      ),
                      updated_at = NOW()
                  WHERE (category = $1 OR name LIKE $2)
                    AND updated_at >= NOW() - INTERVAL '7 days'
                  RETURNING pattern_id
                `, [feedbackType, `%${feedbackType}%`]);
                patternsUpdated = updateResult.rows?.length || 0;
              }
            } catch (e) {
              // Pattern update is best-effort
              log.debug('Pattern frequency update error', { error: e.message });
            }
          }

          // Emit feedback event for automation layer
          try {
            const eventBus = getEventBus();
            eventBus.publish(EventType.FEEDBACK_RECEIVED, {
              source: feedbackData.source || 'manual',
              outcome,
              judgmentId: params.judgmentId,
              originalScore,
              actualScore: params.actualScore || feedbackData.actualScore,
              ...result,
            }, { source: 'brain_learning' });
          } catch (e) {
            // EventBus not available - continue without
          }

          return {
            action: 'feedback',
            source: feedbackData.source || 'manual',
            outcome,
            patternsUpdated,
            ...result,
          };
        }

        case 'test_result': {
          const originalScore = params.originalScore || await getJudgment(params.judgmentId) || 50;

          const result = service.processTestResult({
            judgmentId: params.judgmentId,
            passed: params.passed,
            testSuite: params.testSuite,
            passCount: params.passCount || 0,
            failCount: params.failCount || 0,
            itemType: params.itemType || 'code',
            originalScore,
          });

          await saveState();

          // Persist test feedback to PostgreSQL for training pipeline
          // Now supports orphan feedback (no judgmentId required)
          if (persistence?.storeFeedback) {
            try {
              const outcome = params.passed ? 'correct' : 'incorrect';
              const passRate = (params.passCount || 0) / Math.max(1, (params.passCount || 0) + (params.failCount || 0));
              await persistence.storeFeedback({
                judgmentId: params.judgmentId || null,
                outcome,
                reason: `Test ${params.passed ? 'passed' : 'failed'}: ${params.passCount || 0}/${(params.passCount || 0) + (params.failCount || 0)} (${params.testSuite || 'unknown'})`,
                actualScore: params.passed ? Math.round(passRate * 100) : Math.round(passRate * 50),
                sourceType: 'test_result',
                sourceContext: {
                  testSuite: params.testSuite,
                  passCount: params.passCount || 0,
                  failCount: params.failCount || 0,
                  passed: params.passed,
                },
              });
            } catch (e) {
              // Best-effort
            }
          }

          // Emit feedback event for automation layer
          try {
            const eventBus = getEventBus();
            eventBus.publish(EventType.FEEDBACK_RECEIVED, {
              source: 'test_result',
              passed: params.passed,
              testSuite: params.testSuite,
              passCount: params.passCount || 0,
              failCount: params.failCount || 0,
              ...result,
            }, { source: 'brain_learning' });
          } catch (e) {
            // EventBus not available - continue without
          }

          return {
            action: 'test_result',
            source: 'test_result',
            passed: params.passed,
            ...result,
          };
        }

        case 'commit_result': {
          const originalScore = params.originalScore || await getJudgment(params.judgmentId) || 50;

          const result = service.processCommitResult({
            judgmentId: params.judgmentId,
            success: params.success,
            commitHash: params.commitHash,
            hooksPassed: params.hooksPassed,
            itemType: params.itemType || 'code',
            originalScore,
          });

          await saveState();

          // Persist commit feedback to PostgreSQL
          // Now supports orphan feedback (no judgmentId required)
          if (persistence?.storeFeedback) {
            try {
              await persistence.storeFeedback({
                judgmentId: params.judgmentId || null,
                outcome: params.success ? 'correct' : 'incorrect',
                reason: `Commit ${params.success ? 'succeeded' : 'failed'}${params.hooksPassed === false ? ' (hooks failed)' : ''}: ${params.commitHash || 'unknown'}`,
                actualScore: params.success ? 75 : 25,
                sourceType: 'commit',
                sourceContext: {
                  commitHash: params.commitHash,
                  hooksPassed: params.hooksPassed,
                  success: params.success,
                },
              });
            } catch (e) { /* best-effort */ }
          }

          return {
            action: 'commit_result',
            source: 'commit',
            success: params.success,
            ...result,
          };
        }

        case 'pr_result': {
          const originalScore = params.originalScore || await getJudgment(params.judgmentId) || 50;

          const result = service.processPRResult({
            judgmentId: params.judgmentId,
            status: params.status,
            prNumber: params.prNumber,
            approvalCount: params.approvalCount || 0,
            itemType: params.itemType || 'code',
            originalScore,
          });

          await saveState();

          // Persist PR feedback to PostgreSQL
          // Now supports orphan feedback (no judgmentId required)
          if (persistence?.storeFeedback) {
            try {
              const outcome = params.status === 'merged' ? 'correct'
                : params.status === 'rejected' ? 'incorrect' : 'partial';
              await persistence.storeFeedback({
                judgmentId: params.judgmentId || null,
                outcome,
                reason: `PR #${params.prNumber || '?'} ${params.status} (${params.approvalCount || 0} approvals)`,
                actualScore: params.status === 'merged' ? 80 : params.status === 'rejected' ? 20 : 50,
                sourceType: params.status === 'merged' ? 'pr_merged' : params.status === 'rejected' ? 'pr_rejected' : 'code_review',
                sourceContext: {
                  prNumber: params.prNumber,
                  status: params.status,
                  approvalCount: params.approvalCount || 0,
                },
              });
            } catch (e) { /* best-effort */ }
          }

          return {
            action: 'pr_result',
            source: params.status === 'merged' ? 'pr_merged' : params.status === 'rejected' ? 'pr_rejected' : 'code_review',
            status: params.status,
            ...result,
          };
        }

        case 'build_result': {
          const originalScore = params.originalScore || await getJudgment(params.judgmentId) || 50;

          const result = service.processBuildResult({
            judgmentId: params.judgmentId,
            success: params.success,
            buildId: params.buildId,
            duration: params.duration,
            itemType: params.itemType || 'code',
            originalScore,
          });

          await saveState();

          // Persist build feedback to PostgreSQL
          // Now supports orphan feedback (no judgmentId required)
          if (persistence?.storeFeedback) {
            try {
              await persistence.storeFeedback({
                judgmentId: params.judgmentId || null,
                outcome: params.success ? 'correct' : 'incorrect',
                reason: `Build ${params.success ? 'succeeded' : 'failed'}${params.buildId ? ` (${params.buildId})` : ''}`,
                actualScore: params.success ? 80 : 20,
                sourceType: 'build',
                sourceContext: {
                  buildId: params.buildId,
                  duration: params.duration,
                  success: params.success,
                },
              });
            } catch (e) { /* best-effort */ }
          }

          return {
            action: 'build_result',
            source: 'build',
            success: params.success,
            ...result,
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// JUDGMENT CARD TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create judgment card tool - format/export judgments as shareable cards
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createJudgmentCardTool(persistence = null) {
  return {
    name: 'brain_judgment_card',
    description: 'Generate a shareable Judgment Card from a past judgment. Returns markdown, ASCII, and compact formats. Use for sharing judgments on GitHub, Discord, or docs.',
    inputSchema: {
      type: 'object',
      properties: {
        judgmentId: {
          type: 'string',
          description: 'ID of a past judgment (e.g., jdg_abc123). If omitted, uses the most recent judgment.',
        },
        format: {
          type: 'string',
          enum: ['all', 'markdown', 'ascii', 'compact'],
          description: 'Output format (default: all)',
          default: 'all',
        },
        title: {
          type: 'string',
          description: 'Custom title for the card (overrides item type)',
        },
        compact: {
          type: 'boolean',
          description: 'Compact mode - less detail (default: false)',
          default: false,
        },
        includeThermo: {
          type: 'boolean',
          description: 'Include thermodynamics section (default: false)',
          default: false,
        },
      },
    },
    handler: async (params) => {
      const {
        judgmentId,
        format = 'all',
        title,
        compact = false,
        includeThermo = false,
      } = params;

      let judgment = null;

      // Fetch judgment from persistence
      if (persistence) {
        try {
          if (judgmentId) {
            judgment = await persistence.getJudgment(judgmentId);
          } else {
            // Get most recent judgment
            const recent = await persistence.getRecentJudgments(1);
            judgment = recent?.[0] || null;
          }
        } catch (e) {
          throw new Error(`Failed to fetch judgment: ${e.message}`);
        }
      }

      if (!judgment) {
        throw new Error(judgmentId
          ? `Judgment not found: ${judgmentId}`
          : 'No judgments found. Judge something first.');
      }

      // Normalize stored judgment format to match handler output
      const normalized = {
        requestId: judgment.judgment_id || judgment.id || judgmentId,
        score: judgment.q_score ?? judgment.qScore ?? 50,
        verdict: judgment.verdict || 'WAG',
        confidence: judgment.confidence ?? PHI_INV,
        axiomScores: judgment.axiom_scores || judgment.axiomScores || {},
        weaknesses: judgment.weaknesses || [],
        itemType: judgment.item_type || judgment.itemType || 'unknown',
        timestamp: judgment.created_at
          ? new Date(judgment.created_at).getTime()
          : Date.now(),
      };

      const cardOptions = { title, compact, includeThermo };
      const card = generateCard(normalized, cardOptions);

      switch (format) {
        case 'markdown':
          return { format: 'markdown', card: card.markdown };
        case 'ascii':
          return { format: 'ascii', card: card.ascii };
        case 'compact':
          return { format: 'compact', card: card.compact };
        default:
          return {
            format: 'all',
            markdown: card.markdown,
            ascii: card.ascii,
            compact: card.compact,
            json: card.json,
          };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY (OCP-compliant)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Factory for judgment domain tools
 */
export const judgmentFactory = {
  name: 'judgment',
  domain: 'judgment',
  requires: ['judge'],

  /**
   * Create all judgment domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      judge,
      persistence,
      sessionManager,
      pojChainManager,
      graphIntegration,
      onJudgment,
      burnEnforcer,
    } = options;

    const tools = [];

    // Judge tool (core)
    if (judge) {
      tools.push(createJudgeTool(
        judge,
        persistence,
        sessionManager,
        pojChainManager,
        graphIntegration,
        onJudgment,
        burnEnforcer
      ));
    }

    // Refine tool
    if (judge) {
      tools.push(createRefineTool(judge, persistence));
    }

    // Feedback tool
    if (persistence) {
      tools.push(createFeedbackTool(persistence, sessionManager));
    }

    // Learning tool
    if (persistence) {
      tools.push(createLearningTool({ persistence }));
    }

    // Judgment Card tool (shareable artifacts)
    tools.push(createJudgmentCardTool(persistence));

    return tools;
  },
};
