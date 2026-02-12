/**
 * cynic metrics - Data-Driven Progress Dashboard
 *
 * Displays organism health metrics and weekly goal progress.
 *
 * Usage:
 *   cynic metrics week1          # Show Week 1 goal progress
 *   cynic metrics autonomy       # Show Functional Autonomy score
 *   cynic metrics velocity       # Show learning velocity
 *   cynic metrics snapshot       # Take consciousness snapshot
 *
 * @module @cynic/node/cli/commands/metrics
 */

'use strict';

import chalk from 'chalk';
import { MetricsDashboard } from '../../metrics/dashboard.js';
import { PHI_INV } from '@cynic/core';

/**
 * Format percentage with φ-bound color coding
 */
function formatPercent(value, bound = 1.0) {
  const pct = (value * 100).toFixed(1);
  if (value >= bound * PHI_INV) {
    return chalk.green(`${pct}%`);
  } else if (value >= bound * (1 - PHI_INV)) {
    return chalk.yellow(`${pct}%`);
  } else {
    return chalk.red(`${pct}%`);
  }
}

/**
 * Format goal result
 */
function formatGoal(goal) {
  const status = goal.pass ? chalk.green('✓ PASS') : chalk.red('✗ FAIL');
  const actual = goal.actual.toString().padStart(3);
  const target = goal.target.toString().padStart(3);
  return `${status}  ${actual}/${target}`;
}

/**
 * Display Week 1 progress
 */
async function showWeek1Progress() {
  console.log(chalk.bold('\n🐕 CYNIC Week 1 Progress'));
  console.log(chalk.gray('═'.repeat(60)));

  const dashboard = new MetricsDashboard();

  try {
    const progress = await dashboard.getWeek1Progress();

    console.log(chalk.bold('\nGoals:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  G1.1: Watchers polling ≥3 active`);
    console.log(`        ${formatGoal(progress.goals['G1.1'])}`);
    console.log(`  G1.2: Learning loops ≥5 consuming data`);
    console.log(`        ${formatGoal(progress.goals['G1.2'])}`);
    console.log(`  G1.3: Q-weights ≥10 updates/day`);
    console.log(`        ${formatGoal(progress.goals['G1.3'])}`);
    console.log(`  G1.4: KabbalisticRouter ≥20 calls`);
    console.log(`        ${formatGoal(progress.goals['G1.4'])}`);
    console.log(`  G1.5: LLMRouter ≥10 non-Anthropic routes`);
    console.log(`        ${formatGoal(progress.goals['G1.5'])}`);

    console.log(chalk.bold('\nSummary:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Pass ratio: ${formatPercent(progress.summary.passRatio)}`);
    console.log(`  Week status: ${progress.summary.weekComplete ? chalk.green('✓ COMPLETE') : chalk.yellow('○ IN PROGRESS')}`);

    console.log(chalk.bold('\nBudget:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Consumed: ${formatPercent(progress.budget.consumedRatio)}`);
    console.log(`  Status: ${progress.budget.status}`);

    console.log(chalk.gray('\n' + '═'.repeat(60)));
    console.log(chalk.gray(`Snapshot: ${progress.timestamp}\n`));
  } catch (error) {
    console.error(chalk.red(`\n✗ Failed to compute Week 1 progress: ${error.message}\n`));
    process.exit(1);
  } finally {
    await dashboard.close();
  }
}

/**
 * Display Functional Autonomy score
 */
async function showFunctionalAutonomy() {
  console.log(chalk.bold('\n🐕 CYNIC Functional Autonomy'));
  console.log(chalk.gray('═'.repeat(60)));

  const dashboard = new MetricsDashboard();

  try {
    const fa = await dashboard.getFunctionalAutonomy();

    console.log(chalk.bold('\nComposite Score:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Functional Autonomy: ${formatPercent(fa.functionalAutonomy)}`);
    console.log(`  Target: ${formatPercent(fa.target)}`);
    console.log(`  Progress: ${formatPercent(fa.progress)}`);

    console.log(chalk.bold('\nComponents (weighted):'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Perception (25%):   ${formatPercent(fa.components.perception)}`);
    console.log(`  Learning (25%):     ${formatPercent(fa.components.learning)}`);
    console.log(`  Routing (20%):      ${formatPercent(fa.components.routing)}`);
    console.log(`  Cost (15%):         ${formatPercent(fa.components.cost)}`);
    console.log(`  Multi-Domain (15%): ${formatPercent(fa.components.multiDomain)}`);

    console.log(chalk.gray('\n' + '═'.repeat(60)));
    console.log(chalk.gray(`Snapshot: ${fa.timestamp}\n`));
  } catch (error) {
    console.error(chalk.red(`\n✗ Failed to compute Functional Autonomy: ${error.message}\n`));
    process.exit(1);
  } finally {
    await dashboard.close();
  }
}

/**
 * Display learning velocity
 */
async function showLearningVelocity() {
  console.log(chalk.bold('\n🐕 CYNIC Learning Velocity'));
  console.log(chalk.gray('═'.repeat(60)));

  const dashboard = new MetricsDashboard();

  try {
    const velocity = await dashboard.getLearningVelocity();

    console.log(chalk.bold('\nVelocity:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Maturity gain/week: ${formatPercent(velocity.velocity / 100)}`);
    console.log(`  Trend: ${velocity.trend === 'improving' ? chalk.green('↑ Improving') : chalk.red('↓ Declining')}`);

    console.log(chalk.bold('\nMaturity:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Current: ${formatPercent(velocity.currentMaturity)}`);
    console.log(`  Week ago: ${formatPercent(velocity.weekAgoMaturity)}`);

    console.log(chalk.bold('\nData:'));
    console.log(chalk.gray('─'.repeat(60)));
    console.log(`  Snapshots analyzed: ${velocity.snapshotsAnalyzed}`);

    console.log(chalk.gray('\n' + '═'.repeat(60) + '\n'));
  } catch (error) {
    console.error(chalk.red(`\n✗ Failed to compute learning velocity: ${error.message}\n`));
    process.exit(1);
  } finally {
    await dashboard.close();
  }
}

/**
 * Take consciousness snapshot
 */
async function takeSnapshot() {
  console.log(chalk.bold('\n🐕 Taking Consciousness Snapshot'));
  console.log(chalk.gray('═'.repeat(60)));

  const dashboard = new MetricsDashboard();

  try {
    // Get all current metrics
    const [week1, fa] = await Promise.all([
      dashboard.getWeek1Progress(),
      dashboard.getFunctionalAutonomy(),
    ]);

    // Insert snapshot
    await dashboard.db.query(`
      INSERT INTO consciousness_snapshots (
        active_watchers,
        active_loops,
        budget_consumed,
        budget_status,
        q_updates_today,
        patterns_count,
        routing_accuracy_24h,
        snapshot_type
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    `, [
      week1.goals['G1.1'].actual,
      week1.goals['G1.2'].actual,
      week1.budget.consumedRatio,
      week1.budget.status,
      week1.goals['G1.3'].actual,
      0, // patterns_count - TODO: query patterns table
      fa.components.routing,
      'manual',
    ]);

    console.log(chalk.green('\n✓ Snapshot saved'));
    console.log(chalk.gray(`  Active watchers: ${week1.goals['G1.1'].actual}`));
    console.log(chalk.gray(`  Active loops: ${week1.goals['G1.2'].actual}`));
    console.log(chalk.gray(`  Budget consumed: ${formatPercent(week1.budget.consumedRatio)}`));
    console.log(chalk.gray('\n' + '═'.repeat(60) + '\n'));
  } catch (error) {
    console.error(chalk.red(`\n✗ Failed to take snapshot: ${error.message}\n`));
    process.exit(1);
  } finally {
    await dashboard.close();
  }
}

/**
 * Main metrics command
 */
export async function metricsCommand(args) {
  const subcommand = args._[1] || 'week1';

  switch (subcommand) {
    case 'week1':
      await showWeek1Progress();
      break;
    case 'autonomy':
      await showFunctionalAutonomy();
      break;
    case 'velocity':
      await showLearningVelocity();
      break;
    case 'snapshot':
      await takeSnapshot();
      break;
    default:
      console.error(chalk.red(`\nUnknown subcommand: ${subcommand}\n`));
      console.log('Usage: cynic metrics [week1|autonomy|velocity|snapshot]\n');
      process.exit(1);
  }
}

export default metricsCommand;
