import { PricingOracle, PageIndex, Prometheus, Atlas, IntelligentSwitch, Strategy, calculateCost, LearningEngine } from './packages/llm/src/index.js';

console.log('=== CYNIC LLM Package Integration Test ===');

console.log('\n✅ PricingOracle:', typeof PricingOracle);
console.log('✅ calculateCost:', typeof calculateCost);
console.log('\n✅ PageIndex:', typeof PageIndex);
console.log('\n✅ Prometheus:', typeof Prometheus);
console.log('✅ Atlas:', typeof Atlas);
console.log('\n✅ IntelligentSwitch:', typeof IntelligentSwitch);
console.log('✅ Strategy:', Strategy.FREE, Strategy.SPEED, Strategy.QUALITY, Strategy.BALANCED);
console.log('\n✅ LearningEngine:', typeof LearningEngine);
console.log('\n✅ All modules connected to @cynic/llm!');
