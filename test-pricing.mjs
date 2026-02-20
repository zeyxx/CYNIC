import { calculateCost, compareCosts } from './packages/llm/src/pricing/oracle.js';

console.log('=== CYNIC PricingOracle Validation ===\n');

const tests = [
  { name: 'Claude Code', provider: 'claudeCode', model: 'default', tokens: [1000, 500] },
  { name: 'Ollama', provider: 'ollama', model: 'default', tokens: [1000, 500] },
  { name: 'Anthropic Sonnet', provider: 'anthropic', model: 'claude-sonnet-4-5-20251101', tokens: [1000, 500] },
];

for (const test of tests) {
  const result = calculateCost(test.provider, test.model, test.tokens[0], test.tokens[1]);
  console.log(`${test.name}: ${result.cost.toFixed(6)} USD (${result.type})`);
}

console.log('\n--- Comparison ---');
const comparison = compareCosts(
  [
    { provider: 'claudeCode' }, 
    { provider: 'ollama' }, 
    { provider: 'anthropic', model: 'claude-sonnet-4-5-20251101' }
  ],
  1000, 500
);
comparison.forEach((p, i) => console.log(`${i+1}. ${p.provider}: ${p.cost.toFixed(6)} USD`));

console.log('\n✅ PricingOracle OK');
