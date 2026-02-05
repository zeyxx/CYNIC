import { readFileSync } from 'fs';
const d = JSON.parse(readFileSync('training-checkpoints/eval-dog0-v1/eval-report.json', 'utf-8'));
const vc = {};
const rc = {};
d.results.forEach(r => {
  const p = r.predicted.verdict || 'INVALID';
  const ref = r.reference.verdict;
  vc[p] = (vc[p] || 0) + 1;
  rc[ref] = (rc[ref] || 0) + 1;
});
console.log('Predicted verdicts:', JSON.stringify(vc));
console.log('Reference verdicts:', JSON.stringify(rc));

const matches = d.results.filter(r => r.verdictMatch);
console.log('Matches:', matches.length, '/', d.results.length);

const close = d.results.filter(r => r.scoreDiff <= 10);
console.log('Score within 10:', close.length);

// Wrong verdict but score was close
const wrongVerdictClose = d.results.filter(r => !r.verdictMatch && r.scoreDiff <= 20);
console.log('Wrong verdict but close score (<20):', wrongVerdictClose.length);
wrongVerdictClose.forEach(r =>
  console.log(`  pred: ${r.predicted.qScore} ${r.predicted.verdict} | ref: ${r.reference.qScore} ${r.reference.verdict}`)
);

// Score in WAG range (61-75) but predicted wrong verdict
const scoreWagButWrong = d.results.filter(r =>
  !r.verdictMatch && r.predicted.qScore >= 61 && r.predicted.qScore <= 75
);
console.log('\nScore in WAG range (61-75) but wrong verdict:', scoreWagButWrong.length);
scoreWagButWrong.forEach(r =>
  console.log(`  pred: ${r.predicted.qScore} ${r.predicted.verdict} | ref: ${r.reference.qScore} ${r.reference.verdict}`)
);

// Cases where model output HOWL when should be WAG
const howlWhenWag = d.results.filter(r =>
  r.predicted.verdict === 'HOWL' && r.reference.verdict === 'WAG'
);
console.log('\nHOWL when WAG:', howlWhenWag.length);
