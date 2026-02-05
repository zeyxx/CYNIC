#!/usr/bin/env node
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const { getPool } = require('../../packages/persistence/src/postgres/client.js');

const pool = await getPool();
const q = async (sql) => (await pool.query(sql)).rows;

const judgments = (await q('SELECT COUNT(*)::int as c FROM judgments'))[0].c;
const feedback = (await q('SELECT COUNT(*)::int as c FROM feedback'))[0].c;
const trajectories = (await q('SELECT COUNT(*)::int as c FROM reasoning_trajectories'))[0].c;
const withPath = (await q(`SELECT COUNT(*)::int as c FROM judgments WHERE reasoning_path IS NOT NULL AND jsonb_array_length(reasoning_path) > 0`))[0].c;
const verdicts = await q('SELECT verdict, COUNT(*)::int as c FROM judgments GROUP BY verdict ORDER BY c DESC');
const outcomes = await q('SELECT outcome, COUNT(*)::int as c FROM feedback GROUP BY outcome ORDER BY c DESC');
const itemTypes = await q('SELECT item_type, COUNT(*)::int as c FROM judgments GROUP BY item_type ORDER BY c DESC LIMIT 10');
const recent = await q('SELECT judgment_id, item_type, verdict, q_score, created_at FROM judgments ORDER BY created_at DESC LIMIT 3');

console.log('═══════════════════════════════════════════════════════════════');
console.log('  DB STATE');
console.log('═══════════════════════════════════════════════════════════════');
console.log(`  judgments:        ${judgments}`);
console.log(`  feedback:         ${feedback}`);
console.log(`  trajectories:     ${trajectories}`);
console.log(`  with reasoning:   ${withPath}`);
console.log('');
console.log('  verdicts:', verdicts.map(r => `${r.verdict}=${r.c}`).join(', '));
console.log('  feedback:', outcomes.map(r => `${r.outcome}=${r.c}`).join(', ') || 'none');
console.log('  item types:', itemTypes.map(r => `${r.item_type}=${r.c}`).join(', '));
console.log('');
console.log('  recent judgments:');
for (const r of recent) {
  console.log(`    ${r.judgment_id} | ${r.item_type} | ${r.verdict} | Q=${r.q_score} | ${new Date(r.created_at).toISOString().slice(0,16)}`);
}

// Training readiness
const minFeedback = 34; // Fib 9
const minJudgments = 55; // Fib 10
console.log('');
console.log('  TRAINING READINESS:');
console.log(`    judgments:  ${judgments >= minJudgments ? '✅' : '⚠️'} ${judgments}/${minJudgments}`);
console.log(`    feedback:   ${feedback >= minFeedback ? '✅' : '❌'} ${feedback}/${minFeedback}`);
console.log(`    verdict balance: ${verdicts.length >= 3 ? '✅' : '⚠️'} ${verdicts.length} types`);

process.exit(0);
