/**
 * CLI: Budget Management
 *
 * Commands:
 * - budget status - Show current budget status
 * - budget reset - Reset budget consumption
 * - budget set <tokens> - Set session budget limit
 */

import { getCostLedger } from '../../accounting/cost-ledger.js';
import { getLLMRouter } from '../../orchestration/llm-router.js';

export async function handleBudgetCommand(args) {
  const subcommand = args[0] || 'status';
  const costLedger = getCostLedger();
  const router = getLLMRouter();

  if (subcommand === 'status') {
    // Show budget status
    const budget = costLedger.getBudgetStatus();
    const burnRate = costLedger.getBurnRate();
    const circuitBreaker = router.getCircuitBreakerState();

    console.log('\n💰 Budget Status\n');
    console.log('Level:', budget.level.toUpperCase());
    console.log('Consumed:', budget.consumed, 'tokens');
    if (budget.budget) {
      console.log('Budget:', budget.budget, 'tokens');
      console.log('Remaining:', budget.remaining, 'tokens');
      console.log('Consumed ratio:', (budget.consumedRatio * 100).toFixed(1) + '%');
      if (budget.timeToLimitMinutes !== null) {
        console.log('Time to limit:', budget.timeToLimitMinutes.toFixed(1), 'minutes');
      }
    } else {
      console.log('Budget: UNLIMITED');
    }
    console.log('\nBurn Rate:');
    console.log('  Tokens/min:', burnRate.tokensPerMinute);
    console.log('  Cost/min: $' + burnRate.costPerMinute.toFixed(6));
    console.log('  Velocity:', (burnRate.velocity * 100).toFixed(1) + '%');
    console.log('  Trend:', burnRate.trend);

    console.log('\nCircuit Breaker:');
    console.log('  Anthropic blocked:', circuitBreaker.anthropicBlocked ? 'YES' : 'NO');
    if (circuitBreaker.anthropicBlocked) {
      console.log('  Blocked since:', new Date(circuitBreaker.blockedSince).toISOString());
      console.log('  Reason:', circuitBreaker.blockReason);
    }

    const routerStats = router.getStats();
    console.log('\nRouting Stats:');
    console.log('  Total routes:', routerStats.routesTotal);
    console.log('  Ollama ratio:', (routerStats.ollamaRatio * 100).toFixed(1) + '%');
    console.log('  Degraded routes:', routerStats.routesDegraded);
    console.log('  Blocked routes:', routerStats.routesBlocked);
    console.log('  Cost saved: $' + routerStats.costSaved.toFixed(4));

  } else if (subcommand === 'reset') {
    // Reset budget
    const reason = args[1] || 'manual';
    costLedger.resetBudget(reason);
    router.resetCircuitBreaker();
    console.log('✓ Budget reset (' + reason + ')');

    const budget = costLedger.getBudgetStatus();
    console.log('New status:', budget.level.toUpperCase());

  } else if (subcommand === 'set') {
    // Set budget limit
    const tokens = parseInt(args[1], 10);
    if (isNaN(tokens) || tokens <= 0) {
      console.error('Error: Budget must be a positive number');
      console.error('Usage: budget set <tokens>');
      console.error('Example: budget set 1000000  (1M tokens)');
      return;
    }

    costLedger.setSessionBudget(tokens);
    console.log('✓ Budget set to', tokens, 'tokens');

    const budget = costLedger.getBudgetStatus();
    console.log('Current consumption:', budget.consumed, 'tokens');
    console.log('Ratio:', (budget.consumedRatio * 100).toFixed(1) + '%');
    console.log('Level:', budget.level.toUpperCase());

  } else if (subcommand === 'schedule') {
    // Show time until next reset
    const frequency = args[1] || 'daily';
    const resetInfo = costLedger.getTimeUntilReset(frequency);

    console.log('\n⏰ Budget Reset Schedule\n');
    console.log('Frequency:', frequency);
    console.log('Next reset:', resetInfo.nextReset.toISOString());
    console.log('Time until reset:', resetInfo.hoursUntilReset.toFixed(1), 'hours');

  } else {
    console.error('Unknown budget subcommand:', subcommand);
    console.error('Available: status, reset, set, schedule');
  }
}

export default { handleBudgetCommand };
