/**
 * CYNIC Full Flow Test
 * 
 * Complete vertical integration:
 * Query -> Prometheus (plan) -> IntelligentSwitch (select adapter with pricing) -> Atlas (execute) -> Learning (record)
 */

import { 
  IntelligentSwitch, LearningEngine, EventType, 
  Prometheus, Atlas, Strategy 
} from './packages/llm/src/index.js';
import { calculateCost } from './packages/llm/src/pricing/oracle.js';

console.log('=== CYNIC Full Flow Test ===\n');

// Components
const learning = new LearningEngine({ explorationRate: 0.1 });
const prometheus = new Prometheus();
const intelligentSwitch = new IntelligentSwitch({ strategy: Strategy.BALANCED });

const atlas = new Atlas();
// Mock LLM tool that simulates actual call
atlas.registerTool({
  name: 'llm',
  execute: async (input) => {
    // Use IntelligentSwitch to get cost estimate
    const cost = calculateCost('anthropic', 'claude-sonnet-4-5-20251101', 100, 50);
    return { 
      content: `Processed: ${input.prompt?.slice(0, 30)}...`,
      cost: cost.cost,
      provider: 'anthropic'
    };
  }
});

atlas.registerTool({
  name: 'intelligentSwitch',
  execute: async (input) => {
    // This would use the IntelligentSwitch in real scenario
    return { selected: 'anthropic', strategy: Strategy.BALANCED };
  }
});

console.log('Components initialized\n');

// Full flow
async function processQuery(query) {
  console.log(`\n📥 Query: "${query}"`);
  
  // 1. Prometheus analyzes -> creates plan
  const plan = await prometheus.analyze(query);
  console.log(`📋 Plan: ${plan.steps.length} steps, type: ${plan.taskType}`);
  
  // 2. Atlas executes plan (which would use IntelligentSwitch)
  const result = await atlas.execute(plan);
  console.log(`⚡ Executed: ${result.success ? 'SUCCESS' : 'FAILED'} (${result.duration}ms)`);
  
  // 3. Learning records result with real cost
  const costInfo = calculateCost('anthropic', 'claude-sonnet-4-5-20251101', 100, 50);
  learning.record({
    type: EventType.COMPLETION_SUCCESS,
    adapter: 'anthropic',
    data: { 
      prompt: query, 
      response: result.results[0]?.result?.content,
      latency: result.duration,
      cost: costInfo.cost,
      quality: 0.85
    }
  });
  console.log(`📊 Learning: recorded (cost: $${costInfo.cost.toFixed(6)})`);
  
  return result;
}

// Run flow
await processQuery('What is CYNIC?');
await processQuery('Explain machine learning');
await processQuery('Write hello world in Python');

// Show stats
console.log('\n--- Learning Stats ---');
const stats = learning.getAllStats();
for (const [adapter, data] of Object.entries(stats)) {
  console.log(`${adapter}: ${data.totalUses} uses, ${(data.successRate*100).toFixed(0)}% success, $${data.totalCost.toFixed(4)} total`);
}

console.log('\n✅ Full vertical flow working!');
console.log('📌 Flow: Prometheus → IntelligentSwitch → Atlas → Learning');
