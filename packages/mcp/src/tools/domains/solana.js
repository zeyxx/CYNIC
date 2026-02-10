/**
 * CYNIC Solana Tools - C2.* Row of 7×7 Matrix
 *
 * "On-chain is truth" - κυνικός
 *
 * Wires the 6 ghost Solana modules (judge/decide/act/learn/account/emerge)
 * into MCP tools accessible from Claude Code.
 *
 * Singletons are lazy-initialized — no upfront cost, no WebSocket connections
 * until explicitly called.
 *
 * @module @cynic/mcp/tools/domains/solana
 */

'use strict';

import {
  getSolanaJudge,
  getSolanaDecider,
  getSolanaActor,
  getSolanaLearner,
  getSolanaAccountant,
  getSolanaEmergence,
} from '@cynic/node/solana';

/**
 * Create brain_solana tool — unified access to all C2.* cells
 */
export function createSolanaTool() {
  return {
    name: 'brain_solana',
    description: `Solana blockchain analysis — C2.* row of 7×7 matrix.
Actions:
- judge_tx: Judge a transaction (signature required)
- judge_account: Judge an account (address required)
- judge_program: Judge a program (programId required)
- judge_network: Judge network state
- decide: Decision for a transaction (tx context required)
- decide_fee: Fee optimization recommendation
- stats: Stats from all 6 Solana modules
- health: Health check for all 6 modules
- patterns: Emergent patterns detected
- learn: Record a transaction outcome for learning
- accounting: Economic summary (totals, daily, breakdown)`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: [
            'judge_tx', 'judge_account', 'judge_program', 'judge_network',
            'decide', 'decide_fee',
            'stats', 'health', 'patterns',
            'learn', 'accounting',
          ],
          description: 'Action to perform',
        },
        signature: { type: 'string', description: 'Transaction signature (for judge_tx)' },
        address: { type: 'string', description: 'Account address (for judge_account)' },
        programId: { type: 'string', description: 'Program ID (for judge_program)' },
        tx: { type: 'object', description: 'Transaction context (for decide)' },
        outcome: { type: 'object', description: 'Transaction outcome (for learn)' },
        period: { type: 'string', enum: ['today', 'week', 'all'], description: 'Period for accounting' },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action } = params;

      switch (action) {
        // ═══════════════════════════════════════════════════════════════
        // C2.2: JUDGE
        // ═══════════════════════════════════════════════════════════════
        case 'judge_tx': {
          const judge = getSolanaJudge();
          const result = judge.judgeTransaction({
            signature: params.signature,
            ...(params.tx || {}),
          });
          return { action, judgment: result, timestamp: Date.now() };
        }

        case 'judge_account': {
          const judge = getSolanaJudge();
          const result = judge.judgeAccount({
            address: params.address,
            ...(params.tx || {}),
          });
          return { action, judgment: result, timestamp: Date.now() };
        }

        case 'judge_program': {
          const judge = getSolanaJudge();
          const result = judge.judgeProgram({
            programId: params.programId,
            ...(params.tx || {}),
          });
          return { action, judgment: result, timestamp: Date.now() };
        }

        case 'judge_network': {
          const judge = getSolanaJudge();
          const result = judge.judgeNetwork(params.tx || {});
          return { action, judgment: result, timestamp: Date.now() };
        }

        // ═══════════════════════════════════════════════════════════════
        // C2.3: DECIDE
        // ═══════════════════════════════════════════════════════════════
        case 'decide': {
          const decider = getSolanaDecider();
          const result = decider.decide(params.tx || {}, params);
          return { action, decision: result, timestamp: Date.now() };
        }

        case 'decide_fee': {
          const decider = getSolanaDecider();
          const result = decider.decideFee(params);
          return { action, fee: result, timestamp: Date.now() };
        }

        // ═══════════════════════════════════════════════════════════════
        // C2.5: LEARN
        // ═══════════════════════════════════════════════════════════════
        case 'learn': {
          if (!params.outcome) {
            return { error: 'outcome object required for learn action' };
          }
          const learner = getSolanaLearner();
          learner.recordOutcome(params.outcome);
          return { action, recorded: true, stats: learner.getStats(), timestamp: Date.now() };
        }

        // ═══════════════════════════════════════════════════════════════
        // C2.6: ACCOUNT
        // ═══════════════════════════════════════════════════════════════
        case 'accounting': {
          const accountant = getSolanaAccountant();
          return {
            action,
            totals: accountant.getTotals(),
            daily: accountant.getDailySummary(),
            breakdown: accountant.getTypeBreakdown(),
            efficiency: accountant.getFeeEfficiency(),
            timestamp: Date.now(),
          };
        }

        // ═══════════════════════════════════════════════════════════════
        // C2.7: EMERGE
        // ═══════════════════════════════════════════════════════════════
        case 'patterns': {
          const emergence = getSolanaEmergence();
          emergence.analyze(); // Run analysis before returning
          return {
            action,
            patterns: emergence.getPatterns(20),
            stats: emergence.getStats(),
            timestamp: Date.now(),
          };
        }

        // ═══════════════════════════════════════════════════════════════
        // UNIFIED: Stats + Health
        // ═══════════════════════════════════════════════════════════════
        case 'stats': {
          return {
            action,
            judge: getSolanaJudge().getStats(),
            decider: getSolanaDecider().getStats(),
            actor: getSolanaActor().getStats(),
            learner: getSolanaLearner().getStats(),
            accountant: getSolanaAccountant().getStats(),
            emergence: getSolanaEmergence().getStats(),
            timestamp: Date.now(),
          };
        }

        case 'health': {
          return {
            action,
            judge: getSolanaJudge().getHealth(),
            decider: getSolanaDecider().getHealth(),
            actor: getSolanaActor().getHealth(),
            learner: getSolanaLearner().getHealth(),
            accountant: getSolanaAccountant().getHealth(),
            emergence: getSolanaEmergence().getHealth(),
            timestamp: Date.now(),
          };
        }

        default:
          return { error: `Unknown action: ${action}`, timestamp: Date.now() };
      }
    },
  };
}
