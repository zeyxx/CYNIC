#!/usr/bin/env node
/**
 * Test Daemon LLM Router Endpoints
 */

const DAEMON_URL = 'http://127.0.0.1:6180';

async function testRoute(prompt, expectedProvider) {
  console.log(`\n📝 Testing: "${prompt.substring(0, 60)}..."`);

  const response = await fetch(`${DAEMON_URL}/llm/route`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ prompt })
  });

  if (!response.ok) {
    console.error(`❌ HTTP ${response.status}: ${response.statusText}`);
    return false;
  }

  const data = await response.json();
  console.log(`   Provider: ${data.provider}`);
  console.log(`   Model: ${data.model}`);
  console.log(`   Classification: ${data.classification.complexity} (${data.classification.reason})`);
  console.log(`   Reason: ${data.reason}`);
  console.log(`   Latency: ${data.latency}ms`);

  if (data.provider === 'ollama' && data.response) {
    console.log(`   Response: "${data.response.substring(0, 80)}..."`);
  }

  const match = data.provider === expectedProvider;
  console.log(`   Result: ${match ? '✅ PASS' : '❌ FAIL'} (expected ${expectedProvider})`);

  return match;
}

async function testStats() {
  console.log('\n📊 Fetching routing stats...\n');

  const response = await fetch(`${DAEMON_URL}/llm/route/stats`);

  if (!response.ok) {
    console.error(`❌ HTTP ${response.status}: ${response.statusText}`);
    return false;
  }

  const stats = await response.json();
  console.log('   Total requests:', stats.totalRequests);
  console.log('   Distribution:', stats.distribution);
  console.log('   Target:', stats.targetDistribution);
  console.log('   Complexity breakdown:', JSON.stringify(stats.complexityBreakdown));
  console.log('   Avg latency:', JSON.stringify(stats.avgLatency));

  return true;
}

async function main() {
  console.log('🧪 Testing Daemon LLM Router Endpoints\n');
  console.log(`Daemon URL: ${DAEMON_URL}\n`);

  try {
    // Test 1: Simple task → Ollama
    await testRoute('What is 2+2?', 'ollama');

    // Test 2: Moderate task (code) → Ollama
    await testRoute('Write a function to sort an array', 'ollama');

    // Test 3: Complex task → Claude
    await testRoute('Design a distributed consensus algorithm using Raft protocol with leader election', 'claude');

    // Test 4: Critical task → Claude
    await testRoute('Create a Solana transaction to transfer 100 SOL to mainnet', 'claude');

    // Test 5: Fetch stats
    await testStats();

    console.log('\n✅ All tests completed\n');
  } catch (error) {
    console.error('\n❌ Error:', error.message);
    process.exit(1);
  }
}

main();
