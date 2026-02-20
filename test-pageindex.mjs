import { PageIndex } from './packages/llm/src/retrieval/page-index.js';

console.log('=== PageIndex Validation ===\n');

const index = new PageIndex({ name: 'test' });

// Test documents
const docs = [
  { id: 'doc1', content: 'CYNIC is an autonomous agent framework. It uses LLM for reasoning and planning. CYNIC can work with multiple LLM providers.' },
  { id: 'doc2', content: 'The intelligent switch selects the best LLM based on cost, speed, privacy, and quality. It considers real pricing from the Oracle.' },
  { id: 'doc3', content: 'PageIndex provides reasoning-based retrieval. It builds a tree index and uses LLM to navigate branches. This achieves 98.7% accuracy.' },
];

// Build index
await index.buildFromDocuments(docs);

console.log('Index built:', index.nodes.size, 'nodes');
console.log('Stats:', index.getStats());

// Test retrieval
const result = await index.retrieve('How does CYNIC select LLM providers?');

console.log('\n--- Query: "How does CYNIC select LLM providers?" ---');
console.log('Retrieved', result.context.length, 'nodes');
result.context.forEach((ctx, i) => {
  console.log(`\n${i+1}. [${ctx.level}] ${ctx.summary.slice(0, 80)}...`);
});

console.log('\n✅ PageIndex OK');
