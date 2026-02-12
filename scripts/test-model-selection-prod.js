/**
 * Production Model Selection Test
 *
 * Tests model selection via daemon HTTP API (/llm/ask).
 * Validates Haiku/Sonnet switching based on task complexity.
 *
 * "Test in production — Popper would approve" - κυνικός
 */

import { createLogger } from '@cynic/core';

const log = createLogger('TestModelSelectionProd');

const DAEMON_URL = process.env.CYNIC_DAEMON_URL || 'http://localhost:6180';

/**
 * Call daemon /llm/ask endpoint
 */
async function askLLM(prompt, taskType, options = {}) {
  const response = await fetch(`${DAEMON_URL}/llm/ask`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      prompt,
      taskType,
      ...options,
    }),
  });

  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || `HTTP ${response.status}`);
  }

  return response.json();
}

/**
 * Test model selection in production
 */
async function testModelSelectionProd() {
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│ 🧪 TEST: Model Selection (Production via Daemon)        │');
  console.log('└─────────────────────────────────────────────────────────┘\n');

  // Check daemon availability
  console.log('1️⃣  Checking daemon availability...');
  try {
    const healthResponse = await fetch(`${DAEMON_URL}/health`);
    if (!healthResponse.ok) {
      throw new Error(`Daemon not healthy: ${healthResponse.status}`);
    }
    const health = await healthResponse.json();
    console.log(`   ✓ Daemon healthy (${DAEMON_URL})`);
    console.log(`   • Uptime: ${Math.floor(health.uptime / 1000)}s`);
    console.log(`   • Heap: ${health.heapUsedPercent}%`);
    console.log();
  } catch (err) {
    console.log(`   ✗ Daemon not available: ${err.message}`);
    console.log(`   ℹ️  Start daemon with: cynic daemon start`);
    console.log();
    return {
      success: false,
      error: 'daemon_unavailable',
      message: err.message,
    };
  }

  // Test scenarios
  const scenarios = [
    {
      name: 'Simple search (expect Haiku)',
      prompt: 'What is the capital of France?',
      taskType: 'simple',
      expectedTier: ['haiku', 'ollama'], // Cheap models OK
    },
    {
      name: 'Architecture design (expect Sonnet/Opus)',
      prompt: 'Design a microservices architecture for a high-traffic e-commerce platform with event sourcing',
      taskType: 'architecture',
      expectedTier: ['sonnet', 'opus'], // Need smart models
    },
    {
      name: 'Code review (expect Sonnet)',
      prompt: 'Review this function for security issues: function login(user, pass) { return db.query("SELECT * FROM users WHERE name=" + user); }',
      taskType: 'review',
      expectedTier: ['sonnet', 'opus'],
    },
  ];

  const results = [];
  let totalCost = 0;

  for (const scenario of scenarios) {
    console.log(`📋 Scenario: ${scenario.name}`);
    console.log(`   Prompt: "${scenario.prompt.slice(0, 60)}..."`);

    try {
      const response = await askLLM(scenario.prompt, scenario.taskType, {
        maxTokens: 100, // Keep it cheap for testing
      });

      const { tier, selection, tokens, duration } = response;
      const cost = ((tokens.input * 0.001) + (tokens.output * 0.005)).toFixed(4); // Approx
      totalCost += parseFloat(cost);

      console.log(`   ✓ Response received`);
      console.log(`   • Selected tier: ${tier}`);
      console.log(`   • Reason: ${selection.reason}`);
      console.log(`   • Confidence: ${(selection.confidence * 100).toFixed(1)}%`);
      console.log(`   • Tokens: ${tokens.input} in, ${tokens.output} out`);
      console.log(`   • Duration: ${duration}ms`);
      console.log(`   • Cost: ~$${cost}`);

      const matches = scenario.expectedTier.includes(tier);
      const status = matches ? '✓ PASS' : '✗ FAIL';
      console.log(`   ${status} (expected: ${scenario.expectedTier.join(' or ')})`);
      console.log();

      results.push({
        scenario: scenario.name,
        tier,
        expectedTier: scenario.expectedTier,
        passed: matches,
        confidence: selection.confidence,
        tokens: tokens.input + tokens.output,
        duration,
        cost: parseFloat(cost),
      });
    } catch (err) {
      console.log(`   ✗ FAILED: ${err.message}`);
      console.log();

      results.push({
        scenario: scenario.name,
        passed: false,
        error: err.message,
      });
    }
  }

  // Summary
  console.log('┌─────────────────────────────────────────────────────────┐');
  console.log('│ 📊 TEST RESULTS                                          │');
  console.log('├─────────────────────────────────────────────────────────┤');

  const passed = results.filter(r => r.passed).length;
  const total = results.length;
  const passRate = (passed / total) * 100;

  console.log(`│ Scenarios:  ${total}                                           │`);
  console.log(`│ Passed:     ${passed}                                           │`);
  console.log(`│ Failed:     ${total - passed}                                           │`);
  console.log(`│ Pass rate:  ${passRate.toFixed(1)}%                                      │`);
  console.log(`│ Total cost: ~$${totalCost.toFixed(4)}                                 │`);
  console.log('└─────────────────────────────────────────────────────────┘\n');

  if (passRate >= 66) {
    console.log('✅ MODEL SELECTION WORKING IN PRODUCTION');
    console.log('   • Thompson Sampling routing tasks correctly');
    console.log('   • Daemon LLM endpoints functional');
    console.log('   • Ready for production use');
  } else {
    console.log('⚠️  MODEL SELECTION NEEDS TUNING');
    console.log('   • Some scenarios not routing optimally');
    console.log('   • Review Thompson priors or task mapping');
  }
  console.log();

  return {
    success: passRate >= 66,
    passRate,
    passed,
    total,
    totalCost,
    results,
  };
}

// Run test
testModelSelectionProd()
  .then(result => {
    process.exit(result.success ? 0 : 1);
  })
  .catch(err => {
    console.error('❌ Test FAILED with error:');
    console.error(err);
    process.exit(1);
  });
