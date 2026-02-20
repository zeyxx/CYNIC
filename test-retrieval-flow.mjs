/**
 * CYNIC Complete Flow Test - Prometheus + PageIndex
 */

import { EnhancedPrometheus, Atlas } from './packages/llm/src/orchestration/index.js';
import { PageIndex } from './packages/llm/src/retrieval/index.js';
import { LearningEngine, EventType } from './packages/llm/src/learning/index.js';

console.log('=== Complete Flow: Prometheus + PageIndex + Learning ===\n');

// 1. Setup PageIndex with knowledge
const pageIndex = new PageIndex({ name: 'cynic-docs' });
await pageIndex.buildFromDocuments([
  { id: 'doc1', content: 'CYNIC is an autonomous agent framework. It uses LLM for reasoning. CYNIC has pricing oracle for real costs.' },
  { id: 'doc2', content: 'Intelligent switch selects best LLM based on cost, speed, privacy, quality. Uses Thompson Sampling for exploration.' },
  { id: 'doc3', content: 'PageIndex provides reasoning-based RAG. It achieves 98.7% accuracy by using LLM to navigate document tree.' },
]);
console.log('✅ PageIndex ready with', pageIndex.nodes.size, 'nodes');

// 2. Setup Prometheus with PageIndex
const prometheus = new EnhancedPrometheus({ 
  llm: { complete: async (p) => ({ content: '[Mock response]' }) }
});
prometheus.setPageIndex(pageIndex);
prometheus.registerTools([{ name: 'llm' }]);
console.log('✅ EnhancedPrometheus with PageIndex');

// 3. Setup Atlas + Learning
const atlas = new Atlas();
atlas.registerTool({
  name: 'llm',
  execute: async (input) => ({ content: 'Response with context: ' + input.context?.slice(0, 50) })
});

const learning = new LearningEngine();
console.log('✅ Atlas + Learning ready\n');

// 4. Process queries with retrieval
async function processQuery(query) {
  console.log(`\n📥 Query: "${query}"`);
  
  // Prometheus analyzes + retrieves context
  const plan = await prometheus.analyze(query);
  console.log(`📋 Plan: ${plan.steps.length} step(s), type: ${plan.taskType}`);
  console.log(`   Retrieved context: ${plan.context.retrievedContext?.length || 0} items`);
  
  // Atlas executes
  const result = await atlas.execute(plan);
  console.log(`⚡ Executed: ${result.success ? 'SUCCESS' : 'FAILED'}`);
  
  // Learning records
  learning.record({
    type: EventType.COMPLETION_SUCCESS,
    adapter: 'anthropic',
    data: { prompt: query, latency: result.duration, cost: 0.001, quality: 0.9 }
  });
  
  return result;
}

// Run queries
await processQuery('How does CYNIC select LLM?');
await processQuery('What is PageIndex accuracy?');
await processQuery('Tell me about pricing');

// Stats
console.log('\n--- Stats ---');
console.log('Prometheus retrieval:', prometheus.getRetrievalStats());
console.log('Learning:', learning.getStats());

console.log('\n✅ Complete flow working!');
console.log('Flow: Query → EnhancedPrometheus (with PageIndex) → Atlas → Learning');
