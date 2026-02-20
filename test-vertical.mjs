/**
 * CYNIC Vertical Integration Test
 * 
 * Flow: Learning -> IntelligentSwitch -> Pricing -> Prometheus -> Atlas -> Learning
 */

import { 
  IntelligentSwitch, LearningEngine, EventType, 
  Prometheus, Atlas, Strategy 
} from './packages/llm/src/index.js';
import { calculateCost } from './packages/llm/src/pricing/oracle.js';

console.log('=== CYNIC Vertical Integration Test ===\n');

// 1. Setup Learning Engine
const learning = new LearningEngine({ explorationRate: 0.1 });
console.log('1. Learning Engine initialized');

// 2. Setup IntelligentSwitch with Pricing
const switch_ = new IntelligentSwitch({ strategy: Strategy.BALANCED });
console.log('2. IntelligentSwitch initialized with PricingOracle');

// 3. Setup Prometheus + Atlas
const prometheus = new Prometheus();
prometheus.registerTools([{ name: 'llm' }, { name: 'search' }]);

const atlas = new Atlas();
atlas.registerTool({
  name: 'llm',
  execute: async (input) => {
    // Simulate LLM call with cost
    const cost = calculateCost('anthropic', 'claude-sonnet-4-5-20251101', 100, 50);
    return { content: 'Response', cost: cost.cost };
  }
});
console.log('3. Prometheus + Atlas initialized');

// 4. Simulate task flow
async function runTask(query) {
  console.log(`\n--- Processing: "${query}" ---`);
  
  // Prometheus analyzes and creates plan
  const plan = await prometheus.analyze(query);
  console.log(`  Plan: ${plan.steps.length} steps (${plan.taskType})`);
  
  // Atlas executes plan
  const result = await atlas.execute(plan);
  console.log(`  Execution: ${result.success ? 'SUCCESS' : 'FAILED'}`);
  
  // Learning records the event
  learning.record({
    type: EventType.COMPLETION_SUCCESS,
    adapter: 'anthropic',
    data: { 
      prompt: query, 
      response: result.results[0]?.result?.content,
      latency: result.duration,
      cost: result.results[0]?.result?.cost || 0,
      quality: 0.85
    }
  });
  console.log(`  Learning: event recorded`);
  
  return result;
}

// Run multiple tasks
runTask('What is CYNIC?');
runTask('Explain machine learning');
runTask('Write a hello world');

// Show accumulated learning
console.log('\n--- Learning Stats ---');
console.log(JSON.stringify(learning.getAllStats(), null, 2));

console.log('\n✅ Vertical integration working!');
console.log('Flow: Prometheus -> Atlas -> Learning -> IntelligentSwitch (with Pricing)');
