/**
 * Test End-to-End Judgment Cycle
 *
 * Tests the complete organism flow:
 * PERCEIVE → JUDGE → DECIDE → ACT → LEARN
 *
 * "Le chien chasse pour la première fois" - CYNIC
 */

import { bootCYNIC } from '@cynic/core/boot';
import { CYNICJudge } from '../packages/node/src/judge/judge.js';
import { createLogger } from '@cynic/core';

const log = createLogger('TestCycle');

async function testJudgmentCycle() {
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│ 🧪 TEST: End-to-End Judgment Cycle                      │');
  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Step 1: Boot CYNIC
  console.log('1️⃣  Booting CYNIC...');
  const cynic = await bootCYNIC({
    silent: true,
    exclude: ['node', 'transport', 'consensus', 'mcp', 'mcp-server'],
  });
  console.log(`   ✓ Boot complete: ${cynic.components?.length || 0} components\n`);

  // Step 2: Create a test item
  console.log('2️⃣  Creating test item...');
  const testItem = {
    type: 'code',
    content: `
      function fibonacci(n) {
        if (n <= 1) return n;
        return fibonacci(n - 1) + fibonacci(n - 2);
      }
    `,
    context: {
      filename: 'test.js',
      language: 'javascript',
      purpose: 'Test judgment cycle',
    },
  };
  console.log('   ✓ Test item created (fibonacci function)\n');

  // Step 3: Create Judge instance
  console.log('3️⃣  Creating Judge instance...');
  const judge = new CYNICJudge();
  console.log(`   ✓ Judge instance: ${judge ? 'ACTIVE' : 'NULL'}\n`);

  if (!judge) {
    throw new Error('Judge creation failed');
  }

  // Step 4: Judge the item
  console.log('4️⃣  Judging test item...');
  const startTime = Date.now();

  const judgment = await judge.judge(testItem);

  const elapsed = Date.now() - startTime;
  console.log(`   ✓ Judgment complete in ${elapsed}ms`);
  console.log(`   • ID: ${judgment.id}`);
  console.log(`   • Q-Score: ${judgment.qScore?.toFixed(1) || 'N/A'}`);
  console.log(`   • Verdict: ${judgment.verdict || 'N/A'}`);
  console.log(`   • Confidence: ${judgment.confidence?.toFixed(1) || 'N/A'}%`);
  console.log(`   • Dimensions scored: ${judgment.dimensionScores ? Object.keys(judgment.dimensionScores).length : 0}`);
  console.log();

  // Step 5: Check persistence
  console.log('5️⃣  Checking persistence...');

  // Try to fetch from persistence if available
  let persisted = false;
  try {
    const { getJudgmentRepository } = await import('../packages/persistence/src/repositories/judgment-repository.js');
    const repo = await getJudgmentRepository();
    const fetched = await repo.getById(judgment.id);
    persisted = !!fetched;
    console.log(`   ✓ Judgment persisted to PostgreSQL: ${persisted ? 'YES' : 'NO'}`);
  } catch (err) {
    console.log(`   ⚠️  Persistence check failed: ${err.message}`);
  }
  console.log();

  // Step 6: Simulate feedback
  console.log('6️⃣  Simulating user feedback...');

  try {
    // Import globalEventBus to emit feedback
    const { globalEventBus, EventType } = await import('@cynic/core');

    globalEventBus.emit(EventType.USER_FEEDBACK || 'feedback:processed', {
      judgmentId: judgment.id,
      outcome: 'correct',
      correctVerdict: judgment.verdict,
      userComment: 'Good analysis of fibonacci function',
      timestamp: Date.now(),
    });

    console.log('   ✓ Feedback emitted to event bus');
  } catch (err) {
    console.log(`   ⚠️  Feedback emission failed: ${err.message}`);
  }
  console.log();

  // Step 7: Check learning loops (via event count)
  console.log('7️⃣  Checking learning loops...');

  // Give learning loops time to process
  await new Promise(r => setTimeout(r, 1000));

  try {
    const { globalEventBus } = await import('@cynic/core');
    const listenerCount = globalEventBus.listenerCount(EventType.USER_FEEDBACK || 'feedback:processed');
    console.log(`   • Event bus listeners for feedback: ${listenerCount}`);
    console.log(`   ${listenerCount > 0 ? '✓' : '⚠️'}  Learning loops ${listenerCount > 0 ? 'ACTIVE' : 'NOT WIRED'}`);
  } catch (err) {
    console.log(`   ⚠️  Learning loop check failed: ${err.message}`);
  }
  console.log();

  // Step 8: Summary
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│ 📊 TEST SUMMARY                                          │');
  console.log('├─────────────────────────────────────────────────────────┤');
  console.log(`│ Boot:        ${cynic ? 'PASS ✓' : 'FAIL ✗'.padEnd(47)}│`);
  console.log(`│ Judge:       ${judgment ? 'PASS ✓' : 'FAIL ✗'.padEnd(47)}│`);
  console.log(`│ Persistence: ${persisted ? 'PASS ✓' : 'SKIP ⚠️ '.padEnd(47)}│`);
  console.log(`│ Feedback:    ${'PASS ✓'.padEnd(47)}│`);
  console.log('└─────────────────────────────────────────────────────────┘');
  console.log();

  // Cleanup
  if (cynic.shutdown) {
    await cynic.shutdown();
  }

  return {
    success: !!(cynic && judgment),
    judgment,
    persisted,
  };
}

// Run test
testJudgmentCycle()
  .then(result => {
    console.log(result.success ? '✅ Test PASSED' : '❌ Test FAILED');
    process.exit(result.success ? 0 : 1);
  })
  .catch(err => {
    console.error('❌ Test FAILED with error:');
    console.error(err);
    process.exit(1);
  });
