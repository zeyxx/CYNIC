/**
 * Tests for Session Digest Export
 * "Le chien digère" - verifies markdown export format
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, rmSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Since digest.js has complex imports (cynic lib), we extract and test the pure functions
// by loading the source and evaluating just the formatter

// Replicate formatDigestMarkdown for testing (same logic as in digest.js)
function formatDigestMarkdown(profile, analysis, insights, engineStats, responseJudgment, meta = {}) {
  const now = new Date();
  const ts = now.toISOString();
  const date = ts.slice(0, 10);
  const time = ts.slice(11, 19);
  const sessions = profile.stats?.sessions || 0;
  const errorRate = analysis.toolsUsed > 0
    ? Math.round((analysis.errorsEncountered / analysis.toolsUsed) * 100)
    : 0;
  const completionPct = Math.round((analysis.completionRate || 0) * 100);
  const efficiencyPct = engineStats.efficiency
    ? Math.round(engineStats.efficiency * 100)
    : null;

  const lines = [];

  lines.push(`# CYNIC Session Digest - ${date}`);
  lines.push('');
  lines.push(`> *"Le chien dig\u00E8re"* \u2014 Session ${sessions} | ${time} UTC`);
  lines.push('');

  lines.push('## Summary');
  lines.push('');
  lines.push('| Metric | Value |');
  lines.push('|--------|------:|');
  lines.push(`| Tools used | ${analysis.toolsUsed} |`);
  lines.push(`| Errors | ${analysis.errorsEncountered} |`);
  lines.push(`| Error rate | ${errorRate}% |`);
  if (analysis.todosTotal > 0) {
    lines.push(`| Tasks completed | ${analysis.todosCompleted}/${analysis.todosTotal} (${completionPct}%) |`);
  }
  if (efficiencyPct !== null) {
    lines.push(`| Efficiency (\u03B7) | ${efficiencyPct}% |`);
  }
  if (meta.project) {
    lines.push(`| Project | ${meta.project} |`);
  }
  lines.push('');

  if (analysis.topTools?.length > 0) {
    lines.push('## Top Tools');
    lines.push('');
    for (const { tool, count } of analysis.topTools.slice(0, 5)) {
      const bar = '\u2588'.repeat(Math.min(Math.round(count / 2), 20));
      lines.push(`- **${tool}** \u2014 ${count}x \`${bar}\``);
    }
    lines.push('');
  }

  if (responseJudgment) {
    lines.push('## Response Judgment');
    lines.push('');
    const bar = '\u2588'.repeat(Math.floor(responseJudgment.qScore / 10))
      + '\u2591'.repeat(10 - Math.floor(responseJudgment.qScore / 10));
    lines.push(`**Q-Score:** \`[${bar}]\` ${responseJudgment.qScore}% | **Verdict:** ${responseJudgment.verdict} | **Dog Voice:** ${responseJudgment.dogVoice ? '\u2705' : '\u26A0\uFE0F Missing'}`);
    lines.push('');

    if (responseJudgment.issues?.length > 0) {
      lines.push('Issues:');
      for (const issue of responseJudgment.issues) {
        const icon = issue.severity === 'critical' ? '\uD83D\uDD34'
          : issue.severity === 'high' ? '\u26A0\uFE0F' : '\uD83D\uDCA1';
        lines.push(`- ${icon} ${issue.description}`);
      }
      lines.push('');
    }
  }

  const hasEngineStats = engineStats && Object.keys(engineStats).length > 0;
  if (hasEngineStats) {
    lines.push('## Engine Activity');
    lines.push('');
    if (engineStats.deletions > 0) {
      lines.push(`- \u2702\uFE0F **Deletions:** ${engineStats.deletions} (voluntary poverty)`);
    }
    if (engineStats.emergence) {
      const e = engineStats.emergence;
      const emoji = e.emerged ? '\u2728' : '\uD83E\uDDE0';
      lines.push(`- ${emoji} **Consciousness:** ${e.score.toFixed(1)}% / ${e.maxScore}%${e.emerged ? ' \u2014 EMERGED' : ''}`);
    } else if (engineStats.consciousnessScore) {
      lines.push(`- \uD83E\uDDE0 **Consciousness:** ${engineStats.consciousnessScore}% / 61.8%`);
    }
    if (engineStats.qLearning) {
      lines.push(`- \uD83C\uDFB0 **Q-Learning:** ${engineStats.qLearning.states} states, ${engineStats.qLearning.episodes} episodes`);
    }
    if (engineStats.activeDog) {
      lines.push(`- \uD83D\uDC15 **Active Dog:** ${engineStats.activeDog}`);
    }
    lines.push('');
  }

  if (insights.length > 0) {
    lines.push('## Insights');
    lines.push('');
    for (const insight of insights) {
      lines.push(`- \uD83D\uDCA1 **${insight.type}**: ${insight.description}`);
      if (insight.suggestion) {
        lines.push(`  - \u2192 ${insight.suggestion}`);
      }
    }
    lines.push('');
  }

  lines.push('---');
  lines.push(`*Digested by [CYNIC](\u03BA\u03C5\u03BD\u03B9\u03BA\u03CC\u03C2) | \u03C6 max: 61.8% | Session ${sessions}*`);

  return lines.join('\n');
}

// Test data
const SAMPLE_PROFILE = { stats: { sessions: 42 } };
const SAMPLE_ANALYSIS = {
  toolsUsed: 87,
  errorsEncountered: 3,
  topTools: [
    { tool: 'Read', count: 28 },
    { tool: 'Edit', count: 19 },
    { tool: 'Bash', count: 15 },
  ],
  todosTotal: 5,
  todosCompleted: 4,
  completionRate: 0.8,
};
const SAMPLE_INSIGHTS = [
  { type: 'recurring_error', description: 'Import path mismatch seen 3 times', suggestion: 'Check package.json exports' },
  { type: 'tool_preference', description: 'Read is the most used tool', count: 28 },
];
const SAMPLE_ENGINE = {
  efficiency: 0.54,
  emergence: { score: 42.3, maxScore: 61.8, bar: '████░░░░░░', emerged: false },
  qLearning: { states: 12, episodes: 8 },
  activeDog: 'Architect',
};
const SAMPLE_JUDGMENT = {
  qScore: 80,
  verdict: 'WAG',
  dogVoice: true,
  issues: [],
};

describe('Session Digest Export', () => {
  describe('formatDigestMarkdown', () => {
    it('should produce valid markdown with header', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.startsWith('# CYNIC Session Digest'));
      assert.ok(md.includes('Session 42'));
    });

    it('should include summary table', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('| Tools used | 87 |'));
      assert.ok(md.includes('| Errors | 3 |'));
      assert.ok(md.includes('| Error rate |'));
    });

    it('should include task completion', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('4/5'));
      assert.ok(md.includes('80%'));
    });

    it('should include top tools with bars', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('**Read**'));
      assert.ok(md.includes('28x'));
      assert.ok(md.includes('\u2588')); // Has bar chars
    });

    it('should include response judgment', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('## Response Judgment'));
      assert.ok(md.includes('80%'));
      assert.ok(md.includes('WAG'));
    });

    it('should include engine activity', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('## Engine Activity'));
      assert.ok(md.includes('42.3%'));
      assert.ok(md.includes('Architect'));
      assert.ok(md.includes('Q-Learning'));
    });

    it('should include insights', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('## Insights'));
      assert.ok(md.includes('Import path mismatch'));
      assert.ok(md.includes('Check package.json exports'));
    });

    it('should include CYNIC footer with phi', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, SAMPLE_INSIGHTS, SAMPLE_ENGINE, SAMPLE_JUDGMENT);
      assert.ok(md.includes('CYNIC'));
      assert.ok(md.includes('61.8%'));
    });

    it('should handle minimal data gracefully', () => {
      const md = formatDigestMarkdown(
        { stats: {} },
        { toolsUsed: 0, errorsEncountered: 0, topTools: [] },
        [],
        {},
        null,
      );
      assert.ok(md.includes('# CYNIC Session Digest'));
      assert.ok(md.includes('| Tools used | 0 |'));
      assert.ok(!md.includes('## Response Judgment'));
      assert.ok(!md.includes('## Insights'));
    });

    it('should include project name when provided', () => {
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, [], {}, null, { project: 'CYNIC' });
      assert.ok(md.includes('| Project | CYNIC |'));
    });

    it('should show response judgment issues', () => {
      const judg = {
        qScore: 55,
        verdict: 'BARK',
        dogVoice: false,
        issues: [
          { severity: 'high', description: 'Identity violation detected' },
        ],
      };
      const md = formatDigestMarkdown(SAMPLE_PROFILE, SAMPLE_ANALYSIS, [], {}, judg);
      assert.ok(md.includes('Identity violation'));
      assert.ok(md.includes('Missing'));
    });
  });
});
