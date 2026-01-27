#!/usr/bin/env node
/**
 * Total Memory + Full Autonomy Demo
 *
 * Demonstrates how CYNIC remembers and acts autonomously.
 */

import 'dotenv/config';
import { getPool } from '../packages/persistence/src/postgres/client.js';
import { MemoryRetriever } from '../packages/persistence/src/services/memory-retriever.js';
import { AutonomousGoalsRepository } from '../packages/persistence/src/postgres/repositories/autonomous-goals.js';
import { ProactiveNotificationsRepository } from '../packages/persistence/src/postgres/repositories/proactive-notifications.js';

const pool = getPool();
const userId = 'demo-user';

console.log('🐕 CYNIC Total Memory Demo\n');
console.log('═'.repeat(50));

// 1. Initialize MemoryRetriever
const memory = new MemoryRetriever({ pool });
console.log('\n📚 MemoryRetriever initialized');

// 2. Store a conversation memory
console.log('\n─── Storing Memories ───\n');

const mem1 = await memory.rememberConversation(userId, 'decision',
  'User prefers TypeScript over JavaScript for new projects', {
    importance: 0.8,
    context: { topic: 'language-preference' }
  }
);
console.log('✓ Stored preference:', mem1.content.slice(0, 50) + '...');

const mem2 = await memory.rememberConversation(userId, 'key_moment',
  'Successfully deployed CYNIC MCP server to Render', {
    importance: 0.9,
    context: { topic: 'deployment', project: 'cynic-mcp' }
  }
);
console.log('✓ Stored key moment:', mem2.content.slice(0, 50) + '...');

// 3. Store an architectural decision
const decision = await memory.rememberDecision(userId, {
  projectPath: '/cynic',
  decisionType: 'pattern',
  title: 'Use Repository Pattern for persistence',
  description: 'All database access goes through repository classes',
  rationale: 'Enables testing with mocks and separates concerns',
  alternatives: [
    { option: 'Direct SQL', reason_rejected: 'Hard to test' },
    { option: 'ORM', reason_rejected: 'Too much magic' }
  ],
  consequences: {
    positive: ['Testable', 'Consistent API'],
    negative: ['More boilerplate'],
  }
});
console.log('✓ Stored decision:', decision.title);

// 4. Store a lesson learned
const lesson = await memory.rememberLesson(userId, {
  category: 'bug',
  mistake: 'Forgot to handle null case in user lookup',
  correction: 'Added null check before accessing user properties',
  prevention: 'Always use optional chaining (?.) for potentially null objects',
  severity: 'medium',
});
console.log('✓ Stored lesson:', lesson.mistake.slice(0, 40) + '...');

// 5. Search memories
console.log('\n─── Searching Memories ───\n');

const searchResults = await memory.search(userId, 'TypeScript deployment', {
  limit: 5
});
console.log(`Found ${Object.values(searchResults.sources).flat().length} results:`);
for (const [source, results] of Object.entries(searchResults.sources)) {
  if (results.length > 0) {
    console.log(`  ${source}: ${results.length} match(es)`);
  }
}

// 6. Check for mistakes before action
console.log('\n─── Self-Correction Check ───\n');

const check = await memory.checkForMistakes(userId,
  'accessing user.name without null check'
);
if (check.warning) {
  console.log('⚠️  CYNIC warns:', check.message);
} else {
  console.log('✓ No similar mistakes found');
}

// 7. Create a goal
console.log('\n─── Autonomous Goals ───\n');

const goalsRepo = new AutonomousGoalsRepository(pool);
const goal = await goalsRepo.create({
  userId,
  goalType: 'quality',
  title: 'Improve test coverage to 80%',
  description: 'Add tests for all new Total Memory repositories',
  successCriteria: [
    { criterion: 'conversation-memories.js tested', weight: 0.2, met: false },
    { criterion: 'architectural-decisions.js tested', weight: 0.2, met: false },
    { criterion: 'lessons-learned.js tested', weight: 0.2, met: false },
    { criterion: 'autonomous-*.js tested', weight: 0.4, met: false },
  ],
  priority: 70,
});
console.log('✓ Created goal:', goal.title);
console.log('  Progress:', Math.round(goal.progress * 100) + '%');

// 8. Create a proactive notification
console.log('\n─── Proactive Notifications ───\n');

const notifRepo = new ProactiveNotificationsRepository(pool);
const notif = await notifRepo.create({
  userId,
  notificationType: 'insight',
  title: 'Pattern detected: Repository consistency',
  message: 'All 6 new repositories follow the same BaseRepository pattern. Consider generating tests from a template.',
  priority: 60,
  context: { source: 'code-analysis' }
});
console.log('✓ Created notification:', notif.title);

// 9. Get pending notifications (what user sees at session start)
const pending = await notifRepo.getPending(userId, 5);
console.log(`\n📬 Pending notifications for next session: ${pending.length}`);
pending.forEach(n => console.log(`  • [${n.notificationType}] ${n.title}`));

// 10. Get memory stats
console.log('\n─── Memory Statistics ───\n');
const stats = await memory.getStats(userId);
console.log('Memories:', stats.totals.memories);
console.log('Decisions:', stats.totals.decisions);
console.log('Lessons:', stats.totals.lessons);
console.log('Total:', stats.totals.combined);

// Cleanup
console.log('\n═'.repeat(50));
console.log('🐕 Demo complete. CYNIC remembers.\n');

// Pool cleanup handled by process exit
