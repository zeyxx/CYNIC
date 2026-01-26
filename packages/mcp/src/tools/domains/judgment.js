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

const log = createLogger('JudgmentTools');
import { enrichItem } from '../../item-enricher.js';

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

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
 * @returns {Object} Tool definition
 */
export function createJudgeTool(judge, persistence = null, sessionManager = null, pojChainManager = null, graphIntegration = null, onJudgment = null, burnEnforcer = null) {
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
      const judgment = graphIntegration
        ? await graphIntegration.judgeWithGraph(enrichedItem, context)
        : judge.judge(enrichedItem, context);

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

      // Build response with consistent ID
      // Debug logging for CULTURE score tracking
      log.debug('CULTURE score check', {
        cultureScore: judgment.axiomScores?.CULTURE,
        qScore: judgment.qScore,
        axiomScores: judgment.axiomScores,
      });

      const result = {
        requestId: judgmentId,
        score: judgment.qScore,
        globalScore: judgment.global_score,
        verdict: judgment.qVerdict?.verdict || judgment.verdict,
        confidence: Math.round(judgment.confidence * 1000) / 1000,
        axiomScores: judgment.axiomScores,
        weaknesses: judgment.weaknesses,
        finalScore: judgment.finalScore || null,
        phi: { maxConfidence: PHI_INV, minDoubt: PHI_INV_2 },
        timestamp: Date.now(),
      };

      // ═══════════════════════════════════════════════════════════════════════════
      // PSYCHOLOGY CONTEXT: Informative metadata about user state at judgment time
      // "Comprendre l'humain pour mieux l'aider"
      // NOTE: This is INFORMATIVE ONLY - does NOT affect judgment scores
      // ═══════════════════════════════════════════════════════════════════════════
      if (persistence && sessionContext.userId) {
        try {
          const psychology = await persistence.loadPsychology(sessionContext.userId);
          if (psychology) {
            // Add as informative metadata (not used in scoring)
            result.humanContext = {
              note: 'INFORMATIVE ONLY - does not affect scores',
              energy: psychology.dimensions?.energy?.value,
              focus: psychology.dimensions?.focus?.value,
              frustration: psychology.dimensions?.frustration?.value,
              composites: calculatePsychologyComposites(psychology.dimensions, psychology.emotions),
              sessionCount: psychology.sessionCount,
            };
          }
        } catch (e) {
          // Non-blocking - psychology context is optional
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

      // DEBUG: FINAL RETURN - log what we're actually returning
      log.debug('Handler return', {
        cultureScore: result.axiomScores?.CULTURE,
        axiomScores: result.axiomScores,
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
          if (!params.outcome) {
            throw new Error('outcome required for feedback action');
          }

          const originalScore = params.originalScore || await getJudgment(params.judgmentId) || 50;

          const result = service.processFeedback({
            judgmentId: params.judgmentId,
            outcome: params.outcome,
            actualScore: params.actualScore,
            originalScore,
            itemType: params.itemType || 'unknown',
            source: 'manual',
            sourceContext: params.context || {},
          });

          await saveState();

          return {
            action: 'feedback',
            source: 'manual',
            outcome: params.outcome,
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

    return tools;
  },
};
