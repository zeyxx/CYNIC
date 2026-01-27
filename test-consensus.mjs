import { createCollectivePack } from './packages/node/src/agents/collective/index.js';

const PHI_INV = 0.618;
const PHI_INV_2 = 0.382;

const pack = createCollectivePack();

function testConsensus(question, context) {
  const allAgents = [
    ['guardian', pack.guardian],
    ['analyst', pack.analyst],
    ['scholar', pack.scholar],
    ['architect', pack.architect],
    ['sage', pack.sage],
    ['cynic', pack.cynic],
    ['janitor', pack.janitor],
    ['scout', pack.scout],
    ['cartographer', pack.cartographer],
    ['oracle', pack.oracle],
    ['deployer', pack.deployer],
  ];
  
  const votes = [];
  for (const [name, agent] of allAgents) {
    if (agent && typeof agent.voteOnConsensus === 'function') {
      const result = agent.voteOnConsensus(question, context);
      votes.push({
        agent: name,
        vote: result.vote?.toUpperCase?.() || 'ABSTAIN',
        reason: result.reason,
      });
    }
  }
  
  const approveCount = votes.filter(v => v.vote === 'APPROVE').length;
  const rejectCount = votes.filter(v => v.vote === 'REJECT').length;
  const abstainCount = votes.filter(v => v.vote === 'ABSTAIN').length;
  const decidingVotes = approveCount + rejectCount;
  const approvalRatio = decidingVotes > 0 ? approveCount / decidingVotes : 0;
  const hasVeto = decidingVotes > 0 && (rejectCount / decidingVotes) > PHI_INV_2;
  const approved = !hasVeto && approvalRatio >= PHI_INV;
  
  console.log('\n╔══════════════════════════════════════════════════════════════════╗');
  console.log('║ CONSENSUS: ' + question.substring(0, 52).padEnd(55) + '║');
  console.log('╠══════════════════════════════════════════════════════════════════╣');
  votes.forEach(v => {
    const icon = v.vote === 'APPROVE' ? '✓' : v.vote === 'REJECT' ? '✗' : '○';
    console.log('║ ' + icon + ' ' + v.agent.padEnd(12) + ': ' + v.vote.padEnd(8) + '                                   ║');
  });
  console.log('╠══════════════════════════════════════════════════════════════════╣');
  console.log('║ Approve=' + String(approveCount).padEnd(2) + ' Reject=' + String(rejectCount).padEnd(2) + ' Abstain=' + String(abstainCount).padEnd(2) + ' → ' + (approvalRatio * 100).toFixed(1) + '% (need 61.8%)       ║');
  console.log('║ RESULT: ' + (approved ? '✓ APPROVED' : '✗ NOT APPROVED').padEnd(57) + '║');
  console.log('╚══════════════════════════════════════════════════════════════════╝');
}

// Test 1: Monitoring (Oracle approves)
testConsensus('Should we add comprehensive monitoring to track system health?', { risk: 'low' });

// Test 2: Dangerous delete (Guardian, Deployer reject)
testConsensus('Should we delete all user data and force deploy without rollback?', { risk: 'critical' });

// Test 3: Code refactor (Janitor, Architect approve)
testConsensus('Should we refactor and clean up the authentication module?', { risk: 'low' });

// Test 4: Exploration (Scout approves)
testConsensus('Should we explore and discover new API endpoints?', { risk: 'low' });
