import { LearningEngine, EventType } from './packages/llm/src/learning/index.js';

console.log('=== Learning Engine Test ===\n');

const engine = new LearningEngine({ explorationRate: 0.2 });

// Record some events
engine.record({
  type: EventType.COMPLETION_SUCCESS,
  adapter: 'anthropic',
  data: { latency: 1000, cost: 0.01, quality: 0.9 }
});

engine.record({
  type: EventType.COMPLETION_SUCCESS,
  adapter: 'ollama',
  data: { latency: 500, cost: 0.003, quality: 0.7 }
});

engine.record({
  type: EventType.COMPLETION_SUCCESS,
  adapter: 'ollama',
  data: { latency: 600, cost: 0.003, quality: 0.75 }
});

engine.record({
  type: EventType.COMPLETION_FAILURE,
  adapter: 'claudeCode',
  data: { latency: 100, cost: 0.001 }
});

console.log('Stats:', engine.getStats());

console.log('\n--- Adapter Stats ---');
console.log(JSON.stringify(engine.getAllStats(), null, 2));

console.log('\n--- Best Adapter Selection ---');
const adapters = ['anthropic', 'ollama', 'claudeCode'];
for (let i = 0; i < 5; i++) {
  const best = engine.getBestAdapter(adapters);
  console.log(`  Selection ${i+1}: ${best}`);
}

console.log('\n✅ Learning Engine OK');
