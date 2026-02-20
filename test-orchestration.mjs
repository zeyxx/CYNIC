import { Prometheus, Atlas, TaskType } from './packages/llm/src/orchestration/index.js';

console.log('=== Orchestration (Prometheus + Atlas) Test ===\n');

// Create mock LLM
const mockLLM = {
  complete: async (prompt) => {
    return { content: '[Mock response for: ' + prompt.slice(0, 50) + '...]' };
  }
};

// Create Prometheus (planner)
const prometheus = new Prometheus({ llm: mockLLM });
prometheus.registerTools([
  { name: 'search', description: 'Search docs' },
  { name: 'llm', description: 'LLM response' },
  { name: 'code', description: 'Execute code' },
]);

// Create Atlas (executor)
const atlas = new Atlas();
atlas.registerTool({
  name: 'llm',
  execute: async (input) => {
    return `LLM response to: ${input.prompt.slice(0, 30)}...`;
  }
});
atlas.registerTool({
  name: 'search',
  execute: async (input) => {
    return `Search results for: ${input.query}`;
  }
});

async function test() {
  // Test 1: Simple task
  console.log('Test 1: Simple task');
  const plan1 = await prometheus.analyze('What is CYNIC?');
  console.log('  Task type:', plan1.taskType);
  console.log('  Steps:', plan1.steps.length);
  
  const result1 = await atlas.execute(plan1);
  console.log('  Result:', result1.success ? '✅' : '❌');
  
  // Test 2: Multi-step task
  console.log('\nTest 2: Multi-step task');
  const plan2 = await prometheus.analyze('First search for CYNIC docs, then analyze the results');
  console.log('  Task type:', plan2.taskType);
  console.log('  Steps:', plan2.steps.length);
  plan2.steps.forEach((s, i) => {
    console.log(`    ${i+1}. ${s.action} (${s.tool})`);
  });
  
  // Test 3: Execution
  console.log('\nTest 3: Execute plan');
  const result2 = await atlas.execute(plan2);
  console.log('  Success:', result2.success ? '✅' : '❌');
  console.log('  Duration:', result2.duration, 'ms');
  
  console.log('\n✅ Orchestration OK');
}

test().catch(console.error);
